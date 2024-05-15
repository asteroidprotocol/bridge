use cosmwasm_std::{entry_point, DepsMut, Env, Response};
use neutron_sdk::{
    bindings::{msg::NeutronMsg, query::NeutronQuery},
    sudo::msg::TransferSudoMsg,
};

use crate::{error::ContractError, helpers::build_mint_messages, state::BRIDGE_INFLIGHT};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(
    deps: DepsMut<NeutronQuery>,
    env: Env,
    msg: TransferSudoMsg,
) -> Result<Response<NeutronMsg>, ContractError> {
    // Neutron requires sudo endpoint to be implemented to handle success and
    // failures of the IBC transfers
    // We use this to mint and return the funds to the sender in case of an error
    // by storing the currently in-flight assets in the contract state
    // based on channel and sequence id
    match msg {
        TransferSudoMsg::Response { request, .. } => {
            let channel_id =
                request
                    .source_channel
                    .ok_or_else(|| ContractError::IBCResponseFail {
                        detail: "missing channel id in success".to_string(),
                    })?;

            let sequence_id = request
                .sequence
                .ok_or_else(|| ContractError::IBCResponseFail {
                    detail: "missing sequence id in success".to_string(),
                })?;

            // The IBC transfer succeeded, we can remove the bridging asset from the in-flight
            BRIDGE_INFLIGHT.remove(deps.storage, (&channel_id, sequence_id));

            Ok(Response::new()
                .add_attribute("action", "ibc_bridge_response")
                .add_attribute("state", format!("success on sequence {:?}", sequence_id)))
        }
        TransferSudoMsg::Error { request, .. } => {
            let channel_id =
                request
                    .source_channel
                    .ok_or_else(|| ContractError::IBCResponseFail {
                        detail: "missing channel id in error".to_string(),
                    })?;

            let sequence_id = request
                .sequence
                .ok_or_else(|| ContractError::IBCResponseFail {
                    detail: "missing sequence id in error".to_string(),
                })?;

            // Get the assets being bridged for this channel and sequence
            // We need to mint and return the funds to the sender
            let payload = BRIDGE_INFLIGHT.load(deps.storage, (&channel_id, sequence_id))?;

            let mint_messages = build_mint_messages(
                env.contract.address.to_string(),
                payload.funds.clone(),
                payload.sender.to_string(),
            );

            // Remove the in-flight asset as it has been handled
            BRIDGE_INFLIGHT.remove(deps.storage, (&channel_id, sequence_id));

            Ok(Response::new()
                .add_messages(mint_messages)
                .add_attribute("action", "ibc_bridge_response")
                .add_attribute("state", format!("error on sequence {:?}", sequence_id)))
        }
        TransferSudoMsg::Timeout { request } => {
            let channel_id =
                request
                    .source_channel
                    .ok_or_else(|| ContractError::IBCResponseFail {
                        detail: "missing channel id in timeout".to_string(),
                    })?;

            let sequence_id = request
                .sequence
                .ok_or_else(|| ContractError::IBCResponseFail {
                    detail: "missing sequence id in timeout".to_string(),
                })?;

            // Get the assets being bridged for this channel and sequence
            // We need to mint and return the funds to the sender
            let payload = BRIDGE_INFLIGHT.load(deps.storage, (&channel_id, sequence_id))?;

            let mint_messages = build_mint_messages(
                env.contract.address.to_string(),
                payload.funds.clone(),
                payload.sender.to_string(),
            );

            // Remove the in-flight asset as it has been handled
            BRIDGE_INFLIGHT.remove(deps.storage, (&channel_id, sequence_id));

            Ok(Response::new()
                .add_messages(mint_messages)
                .add_attribute("action", "ibc_bridge_response")
                .add_attribute("state", format!("timeout on sequence {:?}", sequence_id)))
        }
    }
}

#[cfg(test)]
mod testing {
    use std::marker::PhantomData;

    use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockQuerier, MockStorage};
    use cosmwasm_std::{
        coin, coins, to_json_binary, Addr, BankMsg, Coin, ContractResult, OwnedDeps, SubMsg,
        SystemResult,
    };
    use neutron_sdk::bindings::msg::IbcFee;
    use neutron_sdk::query::min_ibc_fee::MinIbcFeeResponse;
    use neutron_sdk::sudo::msg::RequestPacket;
    use osmosis_std::types::osmosis::tokenfactory::v1beta1::MsgMint;

    use crate::contract::instantiate;
    use crate::msg::InstantiateMsg;
    use crate::sudo::sudo;
    use crate::types::{BridgingAsset, FEE_DENOM};

    use super::*;

    pub const OWNER: &str = "owner";
    pub const USER: &str = "cosmos_user";

    fn mock_neutron_dependencies(
        balances: &[(&str, &[Coin])],
    ) -> OwnedDeps<MockStorage, MockApi, MockQuerier<NeutronQuery>, NeutronQuery> {
        let neutron_custom_handler = |request: &NeutronQuery| {
            let contract_result: ContractResult<_> = match request {
                NeutronQuery::MinIbcFee {} => to_json_binary(&MinIbcFeeResponse {
                    min_fee: IbcFee {
                        recv_fee: vec![],
                        ack_fee: coins(100_000, FEE_DENOM),
                        timeout_fee: coins(100_000, FEE_DENOM),
                    },
                })
                .into(),
                _ => unimplemented!("Unsupported query request: {:?}", request),
            };
            SystemResult::Ok(contract_result)
        };

        // QueryRequest::Ibc(IbcQuery::ListChannels { .. }) => {
        //         let response = ListChannelsResponse {
        //             channels: vec![
        //                 IbcChannel::new(
        //                     IbcEndpoint {
        //                         port_id: "wasm".to_string(),
        //                         channel_id: "channel-3".to_string(),
        //                     },
        //                     IbcEndpoint {
        //                         port_id: "wasm".to_string(),
        //                         channel_id: "channel-1".to_string(),
        //                     },
        //                     IbcOrder::Unordered,
        //                     "version",
        //                     "connection-1",
        //                 ),
        //             ],
        //         };
        //         SystemResult::Ok(to_json_binary(&response).into())
        //     }

        OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: MockQuerier::new(balances).with_custom_handler(neutron_custom_handler),
            custom_query_type: PhantomData,
        }
    }

    #[test]
    fn test_bridge_sudo_success() {
        let mut deps = mock_neutron_dependencies(&[]);
        let env = mock_env();

        let info = mock_info(OWNER, &[]);

        instantiate(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            InstantiateMsg {
                owner: OWNER.to_string(),
                bridge_chain_id: "localgaia-1".to_string(),
                bridge_ibc_channel: "channel-1".to_string(),
                ibc_timeout_seconds: 300,
            },
        )
        .unwrap();

        BRIDGE_INFLIGHT
            .save(
                &mut deps.storage,
                ("channel-1", 1),
                &BridgingAsset {
                    sender: Addr::unchecked(USER),
                    funds: coin(100, "factory/contract0/TESTTOKEN"),
                },
            )
            .unwrap();

        // Invalid channel
        let err = sudo(
            deps.as_mut(),
            env.clone(),
            neutron_sdk::sudo::msg::TransferSudoMsg::Response {
                request: RequestPacket {
                    sequence: Some(1u64),
                    source_port: Some("transfer".to_string()),
                    source_channel: None,
                    destination_port: Some("transfer".to_string()),
                    destination_channel: Some("channel-1".to_string()),
                    timeout_height: None,
                    timeout_timestamp: None,
                    data: None,
                },
                data: to_json_binary("").unwrap(),
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            ContractError::IBCResponseFail {
                detail: "missing channel id in success".to_string()
            }
        );

        // Invalid sequence
        let err = sudo(
            deps.as_mut(),
            env.clone(),
            neutron_sdk::sudo::msg::TransferSudoMsg::Response {
                request: RequestPacket {
                    sequence: None,
                    source_port: Some("transfer".to_string()),
                    source_channel: Some("channel-1".to_string()),
                    destination_port: Some("transfer".to_string()),
                    destination_channel: Some("channel-1".to_string()),
                    timeout_height: None,
                    timeout_timestamp: None,
                    data: None,
                },
                data: to_json_binary("").unwrap(),
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            ContractError::IBCResponseFail {
                detail: "missing sequence id in success".to_string()
            }
        );

        sudo(
            deps.as_mut(),
            env.clone(),
            neutron_sdk::sudo::msg::TransferSudoMsg::Response {
                request: RequestPacket {
                    sequence: Some(1u64),
                    source_port: Some("transfer".to_string()),
                    source_channel: Some("channel-1".to_string()),
                    destination_port: Some("transfer".to_string()),
                    destination_channel: Some("channel-1".to_string()),
                    timeout_height: None,
                    timeout_timestamp: None,
                    data: None,
                },
                data: to_json_binary("").unwrap(),
            },
        )
        .unwrap();

        // Check that the inflight was removed
        assert!(!BRIDGE_INFLIGHT.has(&deps.storage, ("channel-1", 1)));
    }

    #[test]
    fn test_bridge_sudo_error() {
        let mut deps = mock_neutron_dependencies(&[]);
        let env = mock_env();

        let info = mock_info(OWNER, &[]);

        instantiate(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            InstantiateMsg {
                owner: OWNER.to_string(),
                bridge_chain_id: "localgaia-1".to_string(),
                bridge_ibc_channel: "channel-0".to_string(),
                ibc_timeout_seconds: 300,
            },
        )
        .unwrap();

        BRIDGE_INFLIGHT
            .save(
                &mut deps.storage,
                ("channel-1", 1),
                &BridgingAsset {
                    sender: Addr::unchecked(USER),
                    funds: coin(100, "factory/contract0/TESTTOKEN"),
                },
            )
            .unwrap();

        // Invalid channel
        let err = sudo(
            deps.as_mut(),
            env.clone(),
            neutron_sdk::sudo::msg::TransferSudoMsg::Error {
                request: RequestPacket {
                    sequence: Some(1u64),
                    source_port: Some("transfer".to_string()),
                    source_channel: None,
                    destination_port: Some("transfer".to_string()),
                    destination_channel: Some("channel-1".to_string()),
                    timeout_height: None,
                    timeout_timestamp: None,
                    data: None,
                },
                details: "".to_string(),
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            ContractError::IBCResponseFail {
                detail: "missing channel id in error".to_string()
            }
        );

        // Invalid sequence
        let err = sudo(
            deps.as_mut(),
            env.clone(),
            neutron_sdk::sudo::msg::TransferSudoMsg::Error {
                request: RequestPacket {
                    sequence: None,
                    source_port: Some("transfer".to_string()),
                    source_channel: Some("channel-1".to_string()),
                    destination_port: Some("transfer".to_string()),
                    destination_channel: Some("channel-1".to_string()),
                    timeout_height: None,
                    timeout_timestamp: None,
                    data: None,
                },
                details: "".to_string(),
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            ContractError::IBCResponseFail {
                detail: "missing sequence id in error".to_string()
            }
        );

        let response = sudo(
            deps.as_mut(),
            env.clone(),
            neutron_sdk::sudo::msg::TransferSudoMsg::Error {
                request: RequestPacket {
                    sequence: Some(1u64),
                    source_port: Some("transfer".to_string()),
                    source_channel: Some("channel-1".to_string()),
                    destination_port: Some("transfer".to_string()),
                    destination_channel: Some("channel-1".to_string()),
                    timeout_height: None,
                    timeout_timestamp: None,
                    data: None,
                },
                details: "".to_string(),
            },
        )
        .unwrap();

        // Verify the tokens are minted
        assert_eq!(
            response.messages[0],
            SubMsg::new(MsgMint {
                sender: "cosmos2contract".to_string(),
                amount: Some(osmosis_std::types::cosmos::base::v1beta1::Coin {
                    amount: "100".to_string(),
                    denom: "factory/contract0/TESTTOKEN".to_string()
                }),
                mint_to_address: "cosmos2contract".to_string()
            }),
        );

        // And sent to the original sender
        assert_eq!(
            response.messages[1],
            SubMsg::new(BankMsg::Send {
                to_address: USER.to_string(),
                amount: coins(100u128, "factory/contract0/TESTTOKEN".to_string())
            })
        );

        // Check that the inflight was removed
        assert!(!BRIDGE_INFLIGHT.has(&deps.storage, ("channel-1", 1)));
    }

    #[test]
    fn test_bridge_sudo_timeout() {
        let mut deps = mock_neutron_dependencies(&[]);
        let env = mock_env();

        let info = mock_info(OWNER, &[]);

        instantiate(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            InstantiateMsg {
                owner: OWNER.to_string(),
                bridge_chain_id: "localgaia-1".to_string(),
                bridge_ibc_channel: "channel-1".to_string(),
                ibc_timeout_seconds: 300,
            },
        )
        .unwrap();

        BRIDGE_INFLIGHT
            .save(
                &mut deps.storage,
                ("channel-1", 1),
                &BridgingAsset {
                    sender: Addr::unchecked(USER),
                    funds: coin(1000, "factory/contract0/TESTTOKEN"),
                },
            )
            .unwrap();

        // Invalid channel
        let err = sudo(
            deps.as_mut(),
            env.clone(),
            neutron_sdk::sudo::msg::TransferSudoMsg::Timeout {
                request: RequestPacket {
                    sequence: Some(1u64),
                    source_port: Some("transfer".to_string()),
                    source_channel: None,
                    destination_port: Some("transfer".to_string()),
                    destination_channel: Some("channel-1".to_string()),
                    timeout_height: None,
                    timeout_timestamp: None,
                    data: None,
                },
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            ContractError::IBCResponseFail {
                detail: "missing channel id in timeout".to_string()
            }
        );

        // Invalid sequence
        let err = sudo(
            deps.as_mut(),
            env.clone(),
            neutron_sdk::sudo::msg::TransferSudoMsg::Timeout {
                request: RequestPacket {
                    sequence: None,
                    source_port: Some("transfer".to_string()),
                    source_channel: Some("channel-1".to_string()),
                    destination_port: Some("transfer".to_string()),
                    destination_channel: Some("channel-1".to_string()),
                    timeout_height: None,
                    timeout_timestamp: None,
                    data: None,
                },
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            ContractError::IBCResponseFail {
                detail: "missing sequence id in timeout".to_string()
            }
        );

        let response = sudo(
            deps.as_mut(),
            env.clone(),
            neutron_sdk::sudo::msg::TransferSudoMsg::Timeout {
                request: RequestPacket {
                    sequence: Some(1u64),
                    source_port: Some("transfer".to_string()),
                    source_channel: Some("channel-1".to_string()),
                    destination_port: Some("transfer".to_string()),
                    destination_channel: Some("channel-1".to_string()),
                    timeout_height: None,
                    timeout_timestamp: None,
                    data: None,
                },
            },
        )
        .unwrap();

        // Verify the tokens are minted
        assert_eq!(
            response.messages[0],
            SubMsg::new(MsgMint {
                sender: "cosmos2contract".to_string(),
                amount: Some(osmosis_std::types::cosmos::base::v1beta1::Coin {
                    amount: "1000".to_string(),
                    denom: "factory/contract0/TESTTOKEN".to_string()
                }),
                mint_to_address: "cosmos2contract".to_string()
            }),
        );

        // And sent to the original sender
        assert_eq!(
            response.messages[1],
            SubMsg::new(BankMsg::Send {
                to_address: USER.to_string(),
                amount: coins(1000u128, "factory/contract0/TESTTOKEN".to_string())
            })
        );

        // Check that the inflight was removed
        assert!(!BRIDGE_INFLIGHT.has(&deps.storage, ("channel-1", 1)));
    }
}

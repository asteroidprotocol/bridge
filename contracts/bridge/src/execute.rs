use astroport::common::{claim_ownership, drop_ownership_proposal, propose_new_owner};
use cosmwasm_std::{
    coin, entry_point, BankMsg, CosmosMsg, IbcMsg, IbcTimeout, Reply, StdError, SubMsg, Uint128,
};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw_utils::one_coin;
use neutron_sdk::bindings::query::NeutronQuery;
use neutron_sdk::query::min_ibc_fee::query_min_ibc_fee;
use neutron_sdk::sudo::msg::RequestPacketTimeoutHeight;
use osmosis_std::types::cosmos::bank::v1beta1::{DenomUnit, Metadata};

use crate::msg::ExecuteMsg;
use crate::state::{OWNERSHIP_PROPOSAL, SIGNERS, TOKEN_MAPPING, TOKEN_METADATA};
use crate::types::{
    Config, CustomIbcMsg, TokenMetadata, MAX_IBC_TIMEOUT_SECONDS, MIN_IBC_TIMEOUT_SECONDS,
};
use crate::{error::ContractError, state::CONFIG};

use neutron_sdk::bindings::msg::{IbcFee, NeutronMsg};

use osmosis_std::types::osmosis::tokenfactory::v1beta1::{
    MsgBurn, MsgCreateDenom, MsgCreateDenomResponse, MsgMint, MsgSetBeforeSendHook,
    MsgSetDenomMetadata,
};

/// This contract accepts only one fee denom
const FEE_DENOM: &str = "untrn";

/// A `reply` call code ID used for sub-messages.
enum ReplyIds {
    InstantiateNewDenom = 1,
}

impl TryFrom<u64> for ReplyIds {
    type Error = ContractError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(ReplyIds::InstantiateNewDenom),
            // 2 => Ok(ReplyIds::InstantiateTrackingContract),
            _ => Err(ContractError::FailedToParseReply {}),
        }
    }
}

/// Exposes all the execute functions available in the contract.
///
/// ## Execute messages
///
/// * **ExecuteMsg::Receive(msg)** Receives a message of type [`Cw20ReceiveMsg`] and processes
/// it depending on the received template.
///
/// * **ExecuteMsg::UpdateConfig { hub_addr }** Update parameters in the Outpost contract. Only the owner is allowed to
/// update the config

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<NeutronQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<NeutronMsg>, ContractError> {
    match msg {
        ExecuteMsg::EnableToken {
            ticker,
            name,
            image_url,
            decimals,
        } => enable_token(deps, env, info, ticker, name, image_url, decimals),
        // ExecuteMsg::Receive {
        //     ticker,
        //     amount,
        //     destination_addr,
        // } => bridge_receive(deps, env, info, ticker, amount, destination_addr),
        // ExecuteMsg::Send { destination_addr } => bridge_send(deps, env, info, destination_addr),
        ExecuteMsg::AddSigner { public_key, name } => add_signer(deps, env, info, public_key, name),
        ExecuteMsg::RemoveSigner { public_key } => remove_signer(deps, env, info, public_key),
        ExecuteMsg::UpdateConfig {
            bridge_ibc_channel,
            ibc_timeout_seconds,
        } => update_config(deps, env, info, bridge_ibc_channel, ibc_timeout_seconds),
        ExecuteMsg::ProposeNewOwner { owner, expires_in } => {
            let config = CONFIG.load(deps.storage)?;
            propose_new_owner(
                deps,
                info,
                env,
                owner,
                expires_in,
                config.owner,
                OWNERSHIP_PROPOSAL,
            )
            .map_err(Into::into)
        }
        ExecuteMsg::DropOwnershipProposal {} => {
            let config: Config = CONFIG.load(deps.storage)?;
            drop_ownership_proposal(deps, info, config.owner, OWNERSHIP_PROPOSAL)
                .map_err(Into::into)
        }
        ExecuteMsg::ClaimOwnership {} => {
            claim_ownership(deps, info, env, OWNERSHIP_PROPOSAL, |deps, new_owner| {
                CONFIG
                    .update::<_, StdError>(deps.storage, |mut v| {
                        v.owner = new_owner;
                        Ok(v)
                    })
                    .map(|_| ())
            })
            .map_err(Into::into)
        }
    }
}

/// Enable the bridging of a CFT-20 token
///
/// If this token doesn't have a corresponding TokenFactory token one will
/// be created using the information provided.
///
/// If the token already has a
/// TokenFactory token and is currently enabled, no action is taken.
///
/// Lastly, if the token has a TokenFactory token but is currently disabled,
/// it will be enabled again without any further changes
fn enable_token(
    deps: DepsMut<NeutronQuery>,
    env: Env,
    info: MessageInfo,
    ticker: String,
    name: String,
    image_url: String,
    decimals: u32,
) -> Result<Response<NeutronMsg>, ContractError> {
    // If we already have this token, return an error
    if TOKEN_MAPPING.has(deps.storage, &ticker) {
        return Err(ContractError::TokenAlreadyExists { ticker });
    }

    // If not, create the denom and set the metadata
    let create_denom_msg = SubMsg::reply_on_success(
        MsgCreateDenom {
            sender: env.contract.address.to_string(),
            subdenom: ticker.clone(),
        },
        ReplyIds::InstantiateNewDenom as u64,
    );

    let metadata = TokenMetadata {
        ticker,
        name,
        image_url,
        decimals,
    };
    TOKEN_METADATA.save(deps.storage, &metadata)?;

    Ok(Response::new().add_submessage(create_denom_msg))
}

/// The entry point to the contract for processing replies from submessages.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match ReplyIds::try_from(msg.id)? {
        ReplyIds::InstantiateNewDenom => {
            let MsgCreateDenomResponse { new_token_denom } = msg.result.try_into()?;

            let metadata = TOKEN_METADATA.load(deps.storage)?;

            let denom_metadata_msg = MsgSetDenomMetadata {
                sender: env.contract.address.to_string(),
                metadata: Some(Metadata {
                    symbol: metadata.ticker.clone(),
                    name: metadata.name,
                    base: new_token_denom.clone(),
                    display: metadata.ticker.clone(),
                    denom_units: vec![
                        DenomUnit {
                            denom: new_token_denom.clone(),
                            exponent: 0,
                            aliases: vec![],
                        },
                        DenomUnit {
                            denom: metadata.ticker.clone(),
                            exponent: metadata.decimals,
                            aliases: vec![],
                        },
                    ],
                    description: format!(
                        "{} is an Asteroid CFT-20 token bridged from the Cosmos Hub",
                        metadata.ticker
                    ),
                    uri: metadata.image_url,
                    uri_hash: "".to_string(),
                }),
            };

            // Save the mapping of TICKER <> DENOM both ways to ease lookups
            // in both directions
            TOKEN_MAPPING.save(deps.storage, &metadata.ticker, &new_token_denom)?;
            TOKEN_MAPPING.save(deps.storage, &new_token_denom, &metadata.ticker)?;
            TOKEN_METADATA.remove(deps.storage);

            Ok(Response::new().add_message(denom_metadata_msg))
        }
    }
}

fn bridge_receive(
    deps: DepsMut<NeutronQuery>,
    env: Env,
    info: MessageInfo,
    ticker: String,
    amount: Uint128,
    destination_addr: String,
) -> Result<Response<NeutronMsg>, ContractError> {
    // TODO
    // On receive, check if the signature for this tranfer is valid, if not, reject

    // Check the amount sent, if 0, reject
    if amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }
    // Check destination address, if invalid, reject
    if deps.api.addr_validate(&destination_addr).is_err() {
        return Err(ContractError::InvalidDestinationAddr {});
    }

    // Check the ticker, if it doesn't exist activate needs to be called first
    if !TOKEN_MAPPING.has(deps.storage, &ticker) {
        return Err(ContractError::TokenDoesNotExist { ticker });
    }

    let tokenfactory_denom = TOKEN_MAPPING.load(deps.storage, &ticker)?;

    // If ticker already exists, mint new tokens to the destination
    let coins_to_mint = coin(amount.u128(), tokenfactory_denom);

    // TokenFactory can only mint to the admin for now
    let mint_msg = MsgMint {
        sender: env.contract.address.to_string(),
        amount: Some(coins_to_mint.clone().into()),
        mint_to_address: env.contract.address.to_string(),
    };

    // Once minted to self, transfer to destination
    let mint_transfer = BankMsg::Send {
        to_address: destination_addr,
        amount: vec![coins_to_mint],
    };

    Ok(Response::default()
        .add_message(mint_msg)
        .add_message(mint_transfer)
        .add_attribute("bridge_receive", ticker))
}

fn bridge_send(
    deps: DepsMut<NeutronQuery>,
    env: Env,
    info: MessageInfo,
    destination_addr: String,
) -> Result<Response<NeutronMsg>, ContractError> {
    // Check the tokens sent, map back to CFT-20 tokens
    // Send IBC message with token destination

    // Only a single coin is allowed to be sent
    let bridging_coin = one_coin(&info)?;

    // Check the mapping for this token, fail if no mapping exists
    let cft20_denom = TOKEN_MAPPING.load(deps.storage, &bridging_coin.denom)?;

    // Contruct the IBC memo message to return X of denom on the Hub

    // Also burn the tokens
    let burn_msg = MsgBurn {
        sender: env.contract.address.to_string(),
        burn_from_address: env.contract.address.to_string(),
        amount: Some(bridging_coin.clone().into()),
    };

    // Set timeout, 10 minutes
    let ibc_timeout_timestamp = env.block.time.plus_seconds(600);

    let fee = min_ntrn_ibc_fee(
        query_min_ibc_fee(deps.as_ref())
            .map_err(|err| StdError::generic_err(err.to_string()))?
            .min_fee,
    );

    // CosmosMsg::Ibc(IbcMsg::Transfer { channel_id: (), to_address: (), amount: (), timeout: () })

    // osmosis_std::types::ibc::applications::transfer::v1::MsgTransfer {

    // }

    // let ibc_transfer = IbcMsg::Transfer {
    //     channel_id: "channel-0".to_string(),
    //     to_address: "cosmos1_bridge".to_string(),
    //     amount: coin(1u128, "untrn"),
    //     timeout: IbcTimeout::with_timestamp(ibc_timeout_timestamp),
    //     // memo: Some("burn".to_string()),
    // };

    let ibc_transfer = NeutronMsg::IbcTransfer {
        source_port: "transfer".to_string(),
        source_channel: "channel-0".to_string(),
        sender: env.contract.address.to_string(),
        receiver: destination_addr,
        token: coin(1u128, "untrn"),
        timeout_height: RequestPacketTimeoutHeight {
            revision_number: None,
            revision_height: None,
        },
        // Neutron expects nanoseconds
        // https://github.com/neutron-org/neutron/blob/303d764b57d871749fcf7d59a67b5d3078779258/proto/transfer/v1/tx.proto#L39-L42
        timeout_timestamp: ibc_timeout_timestamp.nanos(),
        memo: format!("mint {} token {}", bridging_coin.amount, cft20_denom),
        fee: fee.clone(),
    };

    let response = Response::new()
        .add_message(burn_msg)
        .add_message(ibc_transfer)
        .add_attribute("bridge_send", "something?")
        .add_attribute("fee", format!("{:?}", fee));

    Ok(response)
}

/// Add a signer for verification
fn add_signer(
    deps: DepsMut<NeutronQuery>,
    _env: Env,
    info: MessageInfo,
    public_key: String,
    name: String,
) -> Result<Response<NeutronMsg>, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only owner can update the config
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if SIGNERS.has(deps.storage, &public_key) {
        return Err(ContractError::KeyAlreadyLoaded {});
    }

    SIGNERS.save(deps.storage, &public_key, &name)?;

    Ok(Response::default())
}

/// Remove a signer from verification
fn remove_signer(
    deps: DepsMut<NeutronQuery>,
    _env: Env,
    info: MessageInfo,
    public_key: String,
) -> Result<Response<NeutronMsg>, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only owner can update the config
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if SIGNERS.has(deps.storage, &public_key) {
        return Err(ContractError::KeyNotLoaded {});
    }

    SIGNERS.remove(deps.storage, &public_key);

    Ok(Response::default())
}

/// Update the Outpost config
fn update_config(
    deps: DepsMut<NeutronQuery>,
    env: Env,
    info: MessageInfo,
    bridge_ibc_channel: Option<String>,
    ibc_timeout_seconds: Option<u64>,
) -> Result<Response<NeutronMsg>, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    // Only owner can update the config
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(ibc_timeout_seconds) = ibc_timeout_seconds {
        if !(MIN_IBC_TIMEOUT_SECONDS..=MAX_IBC_TIMEOUT_SECONDS).contains(&ibc_timeout_seconds) {
            return Err(ContractError::InvalidIBCTimeout {
                timeout: ibc_timeout_seconds,
                min: MIN_IBC_TIMEOUT_SECONDS,
                max: MAX_IBC_TIMEOUT_SECONDS,
            });
        }
        config.ibc_timeout_seconds = ibc_timeout_seconds;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

fn min_ntrn_ibc_fee(fee: IbcFee) -> IbcFee {
    IbcFee {
        recv_fee: fee.recv_fee,
        ack_fee: fee
            .ack_fee
            .into_iter()
            .filter(|a| a.denom == FEE_DENOM)
            .collect(),
        timeout_fee: fee
            .timeout_fee
            .into_iter()
            .filter(|a| a.denom == FEE_DENOM)
            .collect(),
    }
}

// #[cfg(test)]
// mod tests {

//     use super::*;

//     use cosmwasm_std::{testing::mock_info, IbcMsg, ReplyOn, SubMsg, Uint128, Uint64};

//     use crate::{
//         contract::instantiate,
//         mock::{mock_all, setup_channel, HUB, OWNER, VXASTRO_TOKEN, XASTRO_TOKEN},
//         query::query,
//     };
//     use astroport_governance::interchain::{Hub, ProposalSnapshot};

//     // Test Cases:
//     //
//     // Expect Success
//     //      - An unstake IBC message is emitted
//     //
//     // Expect Error
//     //      - No xASTRO is sent to the contract
//     //      - The funds sent to the contract is not xASTRO
//     //      - The Hub address and channel isn't set
//     //
//     #[test]
//     fn unstake() {
//         let (mut deps, env, info) = mock_all(OWNER);

//         let user = "user";
//         let user_funds = Uint128::from(1000u128);
//         let ibc_timeout_seconds = 10u64;

//         instantiate(
//             deps.as_mut(),
//             env.clone(),
//             info,
//             astroport_governance::outpost::InstantiateMsg {
//                 owner: OWNER.to_string(),
//                 xastro_token_addr: XASTRO_TOKEN.to_string(),
//                 vxastro_token_addr: VXASTRO_TOKEN.to_string(),
//                 hub_addr: HUB.to_string(),
//                 ibc_timeout_seconds: 10,
//             },
//         )
//         .unwrap();

//         // Set up valid Hub
//         setup_channel(deps.as_mut(), env.clone());

//         // Update config with new channel
//         execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(OWNER, &[]),
//             astroport_governance::outpost::ExecuteMsg::UpdateConfig {
//                 hub_addr: None,
//                 hub_channel: Some("channel-3".to_string()),
//                 ibc_timeout_seconds: None,
//             },
//         )
//         .unwrap();

//         // Attempt to unstake with an incorrect token
//         let err = execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info("not_xastro", &[]),
//             astroport_governance::outpost::ExecuteMsg::Receive(Cw20ReceiveMsg {
//                 sender: user.to_string(),
//                 amount: user_funds,
//                 msg: to_binary(&astroport_governance::outpost::Cw20HookMsg::Unstake {}).unwrap(),
//             }),
//         )
//         .unwrap_err();

//         assert_eq!(err, ContractError::Unauthorized {});

//         // Attempt to unstake correctly
//         let res = execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(XASTRO_TOKEN, &[]),
//             astroport_governance::outpost::ExecuteMsg::Receive(Cw20ReceiveMsg {
//                 sender: user.to_string(),
//                 amount: user_funds,
//                 msg: to_binary(&astroport_governance::outpost::Cw20HookMsg::Unstake {}).unwrap(),
//             }),
//         )
//         .unwrap();

//         // Build the expected message
//         let ibc_message = to_binary(&Hub::Unstake {
//             receiver: user.to_string(),
//             amount: user_funds,
//         })
//         .unwrap();

//         // We should have two messages
//         assert_eq!(res.messages.len(), 2);

//         // First message must be the burn of the amount of xASTRO sent
//         assert_eq!(
//             res.messages[0],
//             SubMsg {
//                 id: 0,
//                 gas_limit: None,
//                 reply_on: ReplyOn::Never,
//                 msg: WasmMsg::Execute {
//                     contract_addr: XASTRO_TOKEN.to_string(),
//                     msg: to_binary(&Cw20ExecuteMsg::Burn { amount: user_funds }).unwrap(),
//                     funds: vec![],
//                 }
//                 .into(),
//             }
//         );

//         // Second message must be the IBC unstake
//         assert_eq!(
//             res.messages[1],
//             SubMsg {
//                 id: 0,
//                 gas_limit: None,
//                 reply_on: ReplyOn::Never,
//                 msg: IbcMsg::SendPacket {
//                     channel_id: "channel-3".to_string(),
//                     data: ibc_message,
//                     timeout: env.block.time.plus_seconds(ibc_timeout_seconds).into(),
//                 }
//                 .into(),
//             }
//         );
//     }

//     // Test Cases:
//     //
//     // Expect Success
//     //      - The config is updated
//     //
//     // Expect Error
//     //      - When the config is updated by a non-owner
//     //
//     #[test]
//     fn update_config() {
//         let (mut deps, env, info) = mock_all(OWNER);

//         instantiate(
//             deps.as_mut(),
//             env.clone(),
//             info,
//             astroport_governance::outpost::InstantiateMsg {
//                 owner: OWNER.to_string(),
//                 xastro_token_addr: XASTRO_TOKEN.to_string(),
//                 vxastro_token_addr: VXASTRO_TOKEN.to_string(),
//                 hub_addr: HUB.to_string(),
//                 ibc_timeout_seconds: 10,
//             },
//         )
//         .unwrap();

//         setup_channel(deps.as_mut(), env.clone());

//         // Attempt to update the hub address by a non-owner
//         let err = execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info("not_owner", &[]),
//             astroport_governance::outpost::ExecuteMsg::UpdateConfig {
//                 hub_addr: Some("new_hub".to_string()),
//                 hub_channel: None,
//                 ibc_timeout_seconds: None,
//             },
//         )
//         .unwrap_err();
//         assert_eq!(err, ContractError::Unauthorized {});

//         let config = query(
//             deps.as_ref(),
//             env.clone(),
//             astroport_governance::outpost::QueryMsg::Config {},
//         )
//         .unwrap();

//         // Ensure the config set during instantiation is still there
//         assert_eq!(
//             config,
//             to_binary(&astroport_governance::outpost::Config {
//                 owner: Addr::unchecked(OWNER),
//                 xastro_token_addr: Addr::unchecked(XASTRO_TOKEN),
//                 vxastro_token_addr: Addr::unchecked(VXASTRO_TOKEN),
//                 hub_addr: HUB.to_string(),
//                 hub_channel: None,
//                 ibc_timeout_seconds: 10,
//             })
//             .unwrap()
//         );

//         // Attempt to update the hub address by the owner
//         execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(OWNER, &[]),
//             astroport_governance::outpost::ExecuteMsg::UpdateConfig {
//                 hub_addr: Some("new_owner_hub".to_string()),
//                 hub_channel: None,
//                 ibc_timeout_seconds: None,
//             },
//         )
//         .unwrap();

//         let config = query(
//             deps.as_ref(),
//             env.clone(),
//             astroport_governance::outpost::QueryMsg::Config {},
//         )
//         .unwrap();

//         // Ensure the config set after the update is correct
//         // Once a new Hub is set, the Hub channel is cleared to allow a new
//         // connection
//         assert_eq!(
//             config,
//             to_binary(&astroport_governance::outpost::Config {
//                 owner: Addr::unchecked(OWNER),
//                 xastro_token_addr: Addr::unchecked(XASTRO_TOKEN),
//                 vxastro_token_addr: Addr::unchecked(VXASTRO_TOKEN),
//                 hub_addr: "new_owner_hub".to_string(),
//                 hub_channel: None,
//                 ibc_timeout_seconds: 10,
//             })
//             .unwrap()
//         );

//         // Update the hub channel
//         execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(OWNER, &[]),
//             astroport_governance::outpost::ExecuteMsg::UpdateConfig {
//                 hub_addr: None,
//                 hub_channel: Some("channel-15".to_string()),
//                 ibc_timeout_seconds: None,
//             },
//         )
//         .unwrap();

//         let config = query(
//             deps.as_ref(),
//             env.clone(),
//             astroport_governance::outpost::QueryMsg::Config {},
//         )
//         .unwrap();

//         // Ensure the config set after the update is correct
//         // Once a new Hub is set, the Hub channel is cleared to allow a new
//         // connection
//         assert_eq!(
//             config,
//             to_binary(&astroport_governance::outpost::Config {
//                 owner: Addr::unchecked(OWNER),
//                 xastro_token_addr: Addr::unchecked(XASTRO_TOKEN),
//                 vxastro_token_addr: Addr::unchecked(VXASTRO_TOKEN),
//                 hub_addr: "new_owner_hub".to_string(),
//                 hub_channel: Some("channel-15".to_string()),
//                 ibc_timeout_seconds: 10,
//             })
//             .unwrap()
//         );

//         // Update the IBC timeout
//         execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(OWNER, &[]),
//             astroport_governance::outpost::ExecuteMsg::UpdateConfig {
//                 hub_addr: None,
//                 hub_channel: None,
//                 ibc_timeout_seconds: Some(35),
//             },
//         )
//         .unwrap();

//         let config = query(
//             deps.as_ref(),
//             env,
//             astroport_governance::outpost::QueryMsg::Config {},
//         )
//         .unwrap();

//         // Ensure the config set after the update is correct
//         // Once a new Hub is set, the Hub channel is cleared to allow a new
//         // connection
//         assert_eq!(
//             config,
//             to_binary(&astroport_governance::outpost::Config {
//                 owner: Addr::unchecked(OWNER),
//                 xastro_token_addr: Addr::unchecked(XASTRO_TOKEN),
//                 vxastro_token_addr: Addr::unchecked(VXASTRO_TOKEN),
//                 hub_addr: "new_owner_hub".to_string(),
//                 hub_channel: Some("channel-15".to_string()),
//                 ibc_timeout_seconds: 35,
//             })
//             .unwrap()
//         );
//     }

//     // Test Cases:
//     //
//     // Expect Success
//     //      - A proposal query is emitted when the proposal is not in the cache
//     //      - A vote is emitted when the proposal is in the cache
//     //
//     // Expect Error
//     //      - User has no voting power at the time of the proposal
//     //
//     #[test]
//     fn vote_on_proposal() {
//         let (mut deps, env, info) = mock_all(OWNER);

//         let proposal_id = 1u64;
//         let user = "user";
//         let voting_power = 1000u64;
//         let ibc_timeout_seconds = 10u64;

//         instantiate(
//             deps.as_mut(),
//             env.clone(),
//             info,
//             astroport_governance::outpost::InstantiateMsg {
//                 owner: OWNER.to_string(),
//                 xastro_token_addr: XASTRO_TOKEN.to_string(),
//                 vxastro_token_addr: VXASTRO_TOKEN.to_string(),
//                 hub_addr: HUB.to_string(),
//                 ibc_timeout_seconds,
//             },
//         )
//         .unwrap();

//         // Set up valid Hub
//         setup_channel(deps.as_mut(), env.clone());

//         // Update config with new channel
//         execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(OWNER, &[]),
//             astroport_governance::outpost::ExecuteMsg::UpdateConfig {
//                 hub_addr: None,
//                 hub_channel: Some("channel-3".to_string()),
//                 ibc_timeout_seconds: None,
//             },
//         )
//         .unwrap();

//         // Cast a vote with no proposal in the cache
//         let res = execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(user, &[]),
//             astroport_governance::outpost::ExecuteMsg::CastAssemblyVote {
//                 proposal_id: 1,
//                 vote: astroport_governance::assembly::ProposalVoteOption::For,
//             },
//         )
//         .unwrap();

//         // Wrap the query
//         let ibc_message = to_binary(&Hub::QueryProposal { id: proposal_id }).unwrap();

//         // Ensure a query is emitted
//         assert_eq!(
//             res.messages[0],
//             SubMsg {
//                 id: 0,
//                 gas_limit: None,
//                 reply_on: ReplyOn::Never,
//                 msg: IbcMsg::SendPacket {
//                     channel_id: "channel-3".to_string(),
//                     data: ibc_message,
//                     timeout: env.block.time.plus_seconds(ibc_timeout_seconds).into(),
//                 }
//                 .into(),
//             }
//         );

//         // Add a proposal to the cache
//         PROPOSALS_CACHE
//             .save(
//                 &mut deps.storage,
//                 proposal_id,
//                 &ProposalSnapshot {
//                     id: Uint64::from(proposal_id),
//                     start_time: 1689939457,
//                 },
//             )
//             .unwrap();

//         // Cast a vote with a proposal in the cache
//         let res = execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(user, &[]),
//             astroport_governance::outpost::ExecuteMsg::CastAssemblyVote {
//                 proposal_id,
//                 vote: astroport_governance::assembly::ProposalVoteOption::For,
//             },
//         )
//         .unwrap();

//         // Build the expected message
//         let ibc_message = to_binary(&Hub::CastAssemblyVote {
//             proposal_id,
//             voter: Addr::unchecked(user),
//             vote_option: astroport_governance::assembly::ProposalVoteOption::For,
//             voting_power: Uint128::from(voting_power),
//         })
//         .unwrap();

//         // We should only have 1 message
//         assert_eq!(res.messages.len(), 1);

//         // Ensure a vote is emitted
//         assert_eq!(
//             res.messages[0],
//             SubMsg {
//                 id: 0,
//                 gas_limit: None,
//                 reply_on: ReplyOn::Never,
//                 msg: IbcMsg::SendPacket {
//                     channel_id: "channel-3".to_string(),
//                     data: ibc_message,
//                     timeout: env.block.time.plus_seconds(ibc_timeout_seconds).into(),
//                 }
//                 .into(),
//             }
//         );

//         // Cast a vote on a proposal already voted on
//         let err = execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(user, &[]),
//             astroport_governance::outpost::ExecuteMsg::CastAssemblyVote {
//                 proposal_id,
//                 vote: astroport_governance::assembly::ProposalVoteOption::For,
//             },
//         )
//         .unwrap_err();

//         assert_eq!(err, ContractError::AlreadyVoted {});

//         // Check that we can query the vote
//         let vote_data = query(
//             deps.as_ref(),
//             env,
//             astroport_governance::outpost::QueryMsg::ProposalVoted {
//                 proposal_id,
//                 user: user.to_string(),
//             },
//         )
//         .unwrap();

//         assert_eq!(vote_data, to_binary(&ProposalVoteOption::For).unwrap());
//     }

//     // Test Cases:
//     //
//     // Expect Success
//     //      - An emissions vote is emitted is the user has voting power
//     //
//     // Expect Error
//     //      - User has no voting power
//     //
//     #[test]
//     fn vote_on_emissions() {
//         let (mut deps, env, info) = mock_all(OWNER);

//         let user = "user";
//         let votes = vec![("pool".to_string(), 10000u16)];
//         let voting_power = 1000u64;
//         let ibc_timeout_seconds = 10u64;

//         instantiate(
//             deps.as_mut(),
//             env.clone(),
//             info,
//             astroport_governance::outpost::InstantiateMsg {
//                 owner: OWNER.to_string(),
//                 xastro_token_addr: XASTRO_TOKEN.to_string(),
//                 vxastro_token_addr: VXASTRO_TOKEN.to_string(),
//                 hub_addr: HUB.to_string(),
//                 ibc_timeout_seconds,
//             },
//         )
//         .unwrap();

//         // Set up valid Hub
//         setup_channel(deps.as_mut(), env.clone());

//         // Update config with new channel
//         execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(OWNER, &[]),
//             astroport_governance::outpost::ExecuteMsg::UpdateConfig {
//                 hub_addr: None,
//                 hub_channel: Some("channel-3".to_string()),
//                 ibc_timeout_seconds: None,
//             },
//         )
//         .unwrap();

//         // Cast a vote on emissions
//         let res = execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(user, &[]),
//             astroport_governance::outpost::ExecuteMsg::CastEmissionsVote {
//                 votes: votes.clone(),
//             },
//         )
//         .unwrap();

//         // Build the expected message
//         let ibc_message = to_binary(&Hub::CastEmissionsVote {
//             voter: Addr::unchecked(user),
//             votes,
//             voting_power: Uint128::from(voting_power),
//         })
//         .unwrap();

//         // We should only have 1 message
//         assert_eq!(res.messages.len(), 1);

//         // Ensure a vote is emitted
//         assert_eq!(
//             res.messages[0],
//             SubMsg {
//                 id: 0,
//                 gas_limit: None,
//                 reply_on: ReplyOn::Never,
//                 msg: IbcMsg::SendPacket {
//                     channel_id: "channel-3".to_string(),
//                     data: ibc_message,
//                     timeout: env.block.time.plus_seconds(ibc_timeout_seconds).into(),
//                 }
//                 .into(),
//             }
//         );
//     }

//     // Test Cases:
//     //
//     // Expect Success
//     //      - The kick message is forwarded
//     //
//     // Expect Error
//     //      - When the sender is not the vxASTRO contract
//     //
//     #[test]
//     fn kick_unlocked() {
//         let (mut deps, env, info) = mock_all(OWNER);

//         let user = "user";
//         let ibc_timeout_seconds = 10u64;

//         instantiate(
//             deps.as_mut(),
//             env.clone(),
//             info,
//             astroport_governance::outpost::InstantiateMsg {
//                 owner: OWNER.to_string(),
//                 xastro_token_addr: XASTRO_TOKEN.to_string(),
//                 vxastro_token_addr: VXASTRO_TOKEN.to_string(),
//                 hub_addr: HUB.to_string(),
//                 ibc_timeout_seconds,
//             },
//         )
//         .unwrap();

//         // Set up valid Hub
//         setup_channel(deps.as_mut(), env.clone());

//         // Update config with new channel
//         execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(OWNER, &[]),
//             astroport_governance::outpost::ExecuteMsg::UpdateConfig {
//                 hub_addr: None,
//                 hub_channel: Some("channel-3".to_string()),
//                 ibc_timeout_seconds: None,
//             },
//         )
//         .unwrap();

//         // Kick a user as another user, not allowed
//         let err = execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(user, &[]),
//             astroport_governance::outpost::ExecuteMsg::KickUnlocked {
//                 user: Addr::unchecked(user),
//             },
//         )
//         .unwrap_err();

//         assert_eq!(err, ContractError::Unauthorized {});

//         // Kick a user as the vxASTRO contract
//         let res = execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(VXASTRO_TOKEN, &[]),
//             astroport_governance::outpost::ExecuteMsg::KickUnlocked {
//                 user: Addr::unchecked(user),
//             },
//         )
//         .unwrap();

//         // Build the expected message
//         let ibc_message = to_binary(&Hub::KickUnlockedVoter {
//             voter: Addr::unchecked(user),
//         })
//         .unwrap();

//         // We should only have 1 message
//         assert_eq!(res.messages.len(), 1);

//         // Ensure a kick is emitted
//         assert_eq!(
//             res.messages[0],
//             SubMsg {
//                 id: 0,
//                 gas_limit: None,
//                 reply_on: ReplyOn::Never,
//                 msg: IbcMsg::SendPacket {
//                     channel_id: "channel-3".to_string(),
//                     data: ibc_message,
//                     timeout: env.block.time.plus_seconds(ibc_timeout_seconds).into(),
//                 }
//                 .into(),
//             }
//         );
//     }

//     // Test Cases:
//     //
//     // Expect Success
//     //      - The kick message is forwarded
//     //
//     // Expect Error
//     //      - When the sender is not the vxASTRO contract
//     //
//     #[test]
//     fn kick_blacklisted() {
//         let (mut deps, env, info) = mock_all(OWNER);

//         let user = "user";
//         let ibc_timeout_seconds = 10u64;

//         instantiate(
//             deps.as_mut(),
//             env.clone(),
//             info,
//             astroport_governance::outpost::InstantiateMsg {
//                 owner: OWNER.to_string(),
//                 xastro_token_addr: XASTRO_TOKEN.to_string(),
//                 vxastro_token_addr: VXASTRO_TOKEN.to_string(),
//                 hub_addr: HUB.to_string(),
//                 ibc_timeout_seconds,
//             },
//         )
//         .unwrap();

//         // Set up valid Hub
//         setup_channel(deps.as_mut(), env.clone());

//         // Update config with new channel
//         execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(OWNER, &[]),
//             astroport_governance::outpost::ExecuteMsg::UpdateConfig {
//                 hub_addr: None,
//                 hub_channel: Some("channel-3".to_string()),
//                 ibc_timeout_seconds: None,
//             },
//         )
//         .unwrap();

//         // Kick a user as another user, not allowed
//         let err = execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(user, &[]),
//             astroport_governance::outpost::ExecuteMsg::KickBlacklisted {
//                 user: Addr::unchecked(user),
//             },
//         )
//         .unwrap_err();

//         assert_eq!(err, ContractError::Unauthorized {});

//         // Kick a user as the vxASTRO contract
//         let res = execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(VXASTRO_TOKEN, &[]),
//             astroport_governance::outpost::ExecuteMsg::KickBlacklisted {
//                 user: Addr::unchecked(user),
//             },
//         )
//         .unwrap();

//         // Build the expected message
//         let ibc_message = to_binary(&Hub::KickBlacklistedVoter {
//             voter: Addr::unchecked(user),
//         })
//         .unwrap();

//         // We should only have 1 message
//         assert_eq!(res.messages.len(), 1);

//         // Ensure a kick is emitted
//         assert_eq!(
//             res.messages[0],
//             SubMsg {
//                 id: 0,
//                 gas_limit: None,
//                 reply_on: ReplyOn::Never,
//                 msg: IbcMsg::SendPacket {
//                     channel_id: "channel-3".to_string(),
//                     data: ibc_message,
//                     timeout: env.block.time.plus_seconds(ibc_timeout_seconds).into(),
//                 }
//                 .into(),
//             }
//         );
//     }

//     // Test Cases:
//     //
//     // Expect Success
//     //      - The kick message is forwarded
//     //
//     // Expect Error
//     //      - When the sender is not the vxASTRO contract
//     //
//     #[test]
//     fn withdraw_funds() {
//         let (mut deps, env, info) = mock_all(OWNER);

//         let user = "user";
//         let ibc_timeout_seconds = 10u64;

//         instantiate(
//             deps.as_mut(),
//             env.clone(),
//             info,
//             astroport_governance::outpost::InstantiateMsg {
//                 owner: OWNER.to_string(),
//                 xastro_token_addr: XASTRO_TOKEN.to_string(),
//                 vxastro_token_addr: VXASTRO_TOKEN.to_string(),
//                 hub_addr: HUB.to_string(),
//                 ibc_timeout_seconds,
//             },
//         )
//         .unwrap();

//         // Set up valid Hub
//         setup_channel(deps.as_mut(), env.clone());

//         // Update config with new channel
//         execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(OWNER, &[]),
//             astroport_governance::outpost::ExecuteMsg::UpdateConfig {
//                 hub_addr: None,
//                 hub_channel: Some("channel-3".to_string()),
//                 ibc_timeout_seconds: None,
//             },
//         )
//         .unwrap();

//         // Withdraw stuck funds from the Hub
//         let res = execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(user, &[]),
//             astroport_governance::outpost::ExecuteMsg::WithdrawHubFunds {},
//         )
//         .unwrap();

//         // Build the expected message
//         let ibc_message = to_binary(&Hub::WithdrawFunds {
//             user: Addr::unchecked(user),
//         })
//         .unwrap();

//         // We should only have 1 message
//         assert_eq!(res.messages.len(), 1);

//         // Ensure a withdrawal is emitted
//         assert_eq!(
//             res.messages[0],
//             SubMsg {
//                 id: 0,
//                 gas_limit: None,
//                 reply_on: ReplyOn::Never,
//                 msg: IbcMsg::SendPacket {
//                     channel_id: "channel-3".to_string(),
//                     data: ibc_message,
//                     timeout: env.block.time.plus_seconds(ibc_timeout_seconds).into(),
//                 }
//                 .into(),
//             }
//         );
//     }
// }

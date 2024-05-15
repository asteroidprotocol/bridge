use cosmwasm_schema::serde::de::DeserializeOwned;
use cosmwasm_std::{
    coins,
    testing::{MockApi, MockStorage},
    to_json_binary, Addr, Api, BankMsg, Binary, BlockInfo, ChannelResponse, CustomMsg, CustomQuery,
    Empty, IbcChannel, IbcEndpoint, IbcMsg, IbcOrder, IbcQuery, Querier, Storage, SubMsgResponse,
};
use cw_multi_test::{
    App, AppResponse, BankKeeper, BankSudo, CosmosRouter, DistributionKeeper, FailingModule,
    GovFailingModule, Ibc, Module, StakeKeeper, Stargate, StargateMsg, StargateQuery, SudoMsg,
    WasmKeeper,
};

use anyhow::{Ok, Result as AnyResult};
use osmosis_std::types::osmosis::tokenfactory::v1beta1::{
    MsgBurn, MsgCreateDenom, MsgCreateDenomResponse, MsgMint, MsgSetBeforeSendHook,
    MsgSetDenomMetadata, MsgSetDenomMetadataResponse,
};

pub type StargateApp<ExecC = Empty, QueryC = Empty> = App<
    BankKeeper,
    MockApi,
    MockStorage,
    FailingModule<ExecC, QueryC, Empty>,
    WasmKeeper<ExecC, QueryC>,
    StakeKeeper,
    DistributionKeeper,
    MockIbc,
    GovFailingModule,
    MockStargate,
>;

#[derive(Default)]
pub struct MockIbc {}

impl Module for MockIbc {
    type ExecT = IbcMsg;
    type QueryT = IbcQuery;
    type SudoT = Empty;

    fn execute<ExecC, QueryC>(
        &self,
        _api: &dyn cosmwasm_std::Api,
        _storage: &mut dyn cosmwasm_std::Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &cosmwasm_std::BlockInfo,
        _sender: cosmwasm_std::Addr,
        _msg: Self::ExecT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: std::fmt::Debug
            + Clone
            + PartialEq
            + schemars::JsonSchema
            + serde::de::DeserializeOwned
            + 'static,
        QueryC: cosmwasm_std::CustomQuery + serde::de::DeserializeOwned + 'static,
    {
        Ok(AppResponse::default())
    }

    fn sudo<ExecC, QueryC>(
        &self,
        _api: &dyn cosmwasm_std::Api,
        _storage: &mut dyn cosmwasm_std::Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &cosmwasm_std::BlockInfo,
        _msg: Self::SudoT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: std::fmt::Debug
            + Clone
            + PartialEq
            + schemars::JsonSchema
            + serde::de::DeserializeOwned
            + 'static,
        QueryC: cosmwasm_std::CustomQuery + serde::de::DeserializeOwned + 'static,
    {
        Ok(AppResponse::default())
    }

    fn query(
        &self,
        _api: &dyn cosmwasm_std::Api,
        _storage: &dyn cosmwasm_std::Storage,
        _querier: &dyn cosmwasm_std::Querier,
        _block: &cosmwasm_std::BlockInfo,
        request: Self::QueryT,
    ) -> AnyResult<Binary> {
        match request {
            IbcQuery::Channel {
                channel_id,
                port_id,
            } => {
                if (channel_id == "channel-0" || channel_id == "channel-9")
                    && port_id == Some("transfer".to_string())
                {
                    return Ok(to_json_binary(&ChannelResponse {
                        channel: Some(IbcChannel::new(
                            IbcEndpoint {
                                port_id: "transfer".to_string(),
                                channel_id: channel_id.clone(),
                            },
                            IbcEndpoint {
                                port_id: "transfer".to_string(),
                                channel_id,
                            },
                            IbcOrder::Unordered,
                            "ics20-version",
                            "connection-0",
                        )),
                    })?);
                }
            }
            _ => unimplemented!(),
        }

        Ok(Binary::default())
    }
}

impl Ibc for MockIbc {}

#[derive(Default)]
pub struct MockStargate {}

impl Stargate for MockStargate {}

impl Module for MockStargate {
    type ExecT = StargateMsg;
    type QueryT = StargateQuery;
    type SudoT = Empty;

    fn execute<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        msg: Self::ExecT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: CustomMsg + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        let StargateMsg {
            type_url, value, ..
        } = msg;

        match type_url.as_str() {
            MsgCreateDenom::TYPE_URL => {
                let tf_msg: MsgCreateDenom = value.try_into()?;
                let sender_address = tf_msg.sender.to_string();
                let submsg_response = SubMsgResponse {
                    events: vec![],
                    data: Some(
                        MsgCreateDenomResponse {
                            new_token_denom: format!(
                                "factory/{}/{}",
                                sender_address, tf_msg.subdenom
                            ),
                        }
                        .into(),
                    ),
                };
                Ok(submsg_response.into())
            }
            MsgMint::TYPE_URL => {
                let tf_msg: MsgMint = value.try_into()?;
                let mint_coins = tf_msg
                    .amount
                    .expect("Empty amount in tokenfactory MsgMint!");
                #[cfg(not(any(feature = "injective", feature = "sei")))]
                let to_address = tf_msg.mint_to_address.to_string();
                #[cfg(any(feature = "injective", feature = "sei"))]
                let to_address = sender.to_string();
                let bank_sudo = BankSudo::Mint {
                    to_address,
                    amount: coins(mint_coins.amount.parse()?, mint_coins.denom),
                };
                router.sudo(api, storage, block, bank_sudo.into())
            }
            MsgBurn::TYPE_URL => {
                let tf_msg: MsgBurn = value.try_into()?;
                let burn_coins = tf_msg
                    .amount
                    .expect("Empty amount in tokenfactory MsgBurn!");
                let burn_msg = BankMsg::Burn {
                    amount: coins(burn_coins.amount.parse()?, burn_coins.denom),
                };
                router.execute(
                    api,
                    storage,
                    block,
                    Addr::unchecked(sender),
                    burn_msg.into(),
                )
            }
            MsgSetBeforeSendHook::TYPE_URL => {
                let before_hook_msg: MsgSetBeforeSendHook = value.try_into()?;
                let msg = BankSudo::SetHook {
                    contract_addr: before_hook_msg.cosmwasm_address,
                    denom: before_hook_msg.denom,
                };
                router.sudo(api, storage, block, SudoMsg::Bank(msg))
            }
            MsgSetDenomMetadata::TYPE_URL => {
                let _tf_msg: MsgSetDenomMetadata = value.try_into()?;
                let submsg_response = SubMsgResponse {
                    events: vec![],
                    data: Some(MsgSetDenomMetadataResponse {}.into()),
                };
                Ok(submsg_response.into())
            }
            _ => Err(anyhow::anyhow!(
                "Unexpected exec msg {type_url} from {sender:?}",
            )),
        }
    }
    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        _request: Self::QueryT,
    ) -> AnyResult<Binary> {
        Ok(Binary::default())
    }
    fn sudo<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _msg: Self::SudoT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: CustomMsg + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        unimplemented!("Sudo not implemented")
    }
}

#[cfg(test)]
use std::marker::PhantomData;

use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    coins, from_json, to_json_binary, ChannelResponse, Coin, IbcChannel, IbcEndpoint, IbcOrder,
    IbcQuery, OwnedDeps, Querier, QuerierResult, QueryRequest, SystemError, SystemResult,
};
use neutron_sdk::bindings::msg::IbcFee;
use neutron_sdk::bindings::query::NeutronQuery;
use neutron_sdk::query::min_ibc_fee::MinIbcFeeResponse;

use crate::types::FEE_DENOM;

pub fn mock_neutron_dependencies(
    balances: &[(&str, &[Coin])],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier, NeutronQuery> {
    let custom_querier = WasmMockQuerier::new(MockQuerier::new(balances));

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: PhantomData,
    }
}

/// WasmMockQuerier will respond to requests from the custom querier,
/// providing responses to the contracts
pub struct WasmMockQuerier {
    base: MockQuerier<NeutronQuery>,
}
impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely
        let request: QueryRequest<NeutronQuery> = match from_json(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<NeutronQuery>) -> QuerierResult {
        match &request {
            QueryRequest::Custom(NeutronQuery::MinIbcFee {}) => {
                let response = MinIbcFeeResponse {
                    min_fee: IbcFee {
                        recv_fee: vec![],
                        ack_fee: coins(100_000, FEE_DENOM),
                        timeout_fee: coins(100_000, FEE_DENOM),
                    },
                };
                SystemResult::Ok(to_json_binary(&response).into())
            }
            QueryRequest::Ibc(IbcQuery::Channel { .. }) => {
                let response = ChannelResponse {
                    channel: Some(IbcChannel::new(
                        IbcEndpoint {
                            port_id: "transfer".to_string(),
                            channel_id: "channel-0".to_string(),
                        },
                        IbcEndpoint {
                            port_id: "transfer".to_string(),
                            channel_id: "channel-0".to_string(),
                        },
                        IbcOrder::Unordered,
                        "version",
                        "connection-0",
                    )),
                };
                SystemResult::Ok(to_json_binary(&response).into())
            }
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<NeutronQuery>) -> Self {
        WasmMockQuerier { base }
    }
}

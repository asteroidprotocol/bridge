// use std::marker::PhantomData;

// use crate::types::FEE_DENOM;
// use cosmwasm_std::{
//     coins,
//     testing::{MockApi, MockQuerier, MockStorage},
//     to_json_binary, ContractResult, OwnedDeps, SystemResult,
// };
// use neutron_sdk::{
//     bindings::{msg::IbcFee, query::NeutronQuery},
//     query::min_ibc_fee::MinIbcFeeResponse,
// };

// pub fn mock_neutron_dependencies(
// ) -> OwnedDeps<MockStorage, MockApi, MockQuerier<NeutronQuery>, NeutronQuery> {
//     let neutron_custom_handler = |request: &NeutronQuery| {
//         let contract_result: ContractResult<_> = match request {
//             NeutronQuery::MinIbcFee {} => to_json_binary(&MinIbcFeeResponse {
//                 min_fee: IbcFee {
//                     recv_fee: vec![],
//                     ack_fee: coins(10000, FEE_DENOM),
//                     timeout_fee: coins(10000, FEE_DENOM),
//                 },
//             })
//             .into(),
//             _ => unimplemented!("Unsupported query request: {:?}", request),
//         };
//         SystemResult::Ok(contract_result)
//     };

//     OwnedDeps {
//         storage: MockStorage::default(),
//         api: MockApi::default(),
//         querier: MockQuerier::new(&[]).with_custom_handler(neutron_custom_handler),
//         custom_query_type: PhantomData,
//     }
// }

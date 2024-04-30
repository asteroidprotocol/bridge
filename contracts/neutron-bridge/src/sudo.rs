use cosmwasm_std::{entry_point, DepsMut, Env, Response, StdError};
use neutron_sdk::sudo::msg::TransferSudoMsg;

use crate::error::ContractError;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(_deps: DepsMut, _env: Env, _msg: TransferSudoMsg) -> Result<Response, ContractError> {
    // Neutron requires sudo endpoint to be implemented, however, we can't use
    // it for tracking failures as not enough information is provided
    // Instead, the query
    // > neutrond query contractmanager failures [contract-address] provides
    // failure IDs that can be used to retry the failed transactions via
    // ExecuteMsg::RetrySend{failure_id: u64}
    // Ok(Response::new())
    // TODO: This section needs to be fixed
    Err(ContractError::Std(StdError::generic_err(
        "failures to be handled via sudo".to_string(),
    )))
}

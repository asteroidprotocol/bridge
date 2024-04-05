use cosmwasm_std::{entry_point, DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;
use neutron_sdk::bindings::msg::NeutronMsg;
use neutron_sdk::bindings::query::NeutronQuery;

use crate::error::ContractError;
use crate::msg::{InstantiateMsg, MigrateMsg};
use crate::state::CONFIG;
use crate::types::{Config, MAX_IBC_TIMEOUT_SECONDS, MIN_IBC_TIMEOUT_SECONDS};

/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "asteroid-bridge";
/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Instantiates the contract, storing the config.
/// Returns a `Response` object on successful execution or a `ContractError` on failure.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<NeutronQuery>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<NeutronMsg>, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if !(MIN_IBC_TIMEOUT_SECONDS..=MAX_IBC_TIMEOUT_SECONDS).contains(&msg.ibc_timeout_seconds) {
        return Err(ContractError::InvalidIBCTimeout {
            timeout: msg.ibc_timeout_seconds,
            min: MIN_IBC_TIMEOUT_SECONDS,
            max: MAX_IBC_TIMEOUT_SECONDS,
        });
    }

    // TODO: Validate the bridge IBC channel

    let config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        signer_threshold: msg.signer_threshold,
        bridge_chain_id: msg.bridge_chain_id,
        bridge_ibc_channel: msg.bridge_ibc_channel,
        ibc_timeout_seconds: msg.ibc_timeout_seconds,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

/// Migrates the contract to a new version.
#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Err(ContractError::MigrationError {})
    Ok(Response::default())
}

// #[cfg(test)]
// mod tests {

//     use super::*;

//     use crate::{
//         contract::instantiate,
//         mock::{mock_all, HUB, OWNER, VXASTRO_TOKEN, XASTRO_TOKEN},
//     };

//     // Test Cases:
//     //
//     // Expect Success
//     //      - Invalid IBC timeouts are rejected
//     //
//     #[test]
//     fn invalid_ibc_timeout() {
//         let (mut deps, env, info) = mock_all(OWNER);

//         // Test MAX + 1
//         let ibc_timeout_seconds = MAX_IBC_TIMEOUT_SECONDS + 1;
//         let err = instantiate(
//             deps.as_mut(),
//             env.clone(),
//             info.clone(),
//             astroport_governance::outpost::InstantiateMsg {
//                 owner: OWNER.to_string(),
//                 xastro_token_addr: XASTRO_TOKEN.to_string(),
//                 vxastro_token_addr: VXASTRO_TOKEN.to_string(),
//                 hub_addr: HUB.to_string(),
//                 ibc_timeout_seconds,
//             },
//         )
//         .unwrap_err();

//         assert_eq!(
//             err,
//             ContractError::InvalidIBCTimeout {
//                 timeout: ibc_timeout_seconds,
//                 min: MIN_IBC_TIMEOUT_SECONDS,
//                 max: MAX_IBC_TIMEOUT_SECONDS
//             }
//         );

//         // Test MIN - 1
//         let ibc_timeout_seconds = MIN_IBC_TIMEOUT_SECONDS - 1;
//         let err = instantiate(
//             deps.as_mut(),
//             env,
//             info,
//             astroport_governance::outpost::InstantiateMsg {
//                 owner: OWNER.to_string(),
//                 xastro_token_addr: XASTRO_TOKEN.to_string(),
//                 vxastro_token_addr: VXASTRO_TOKEN.to_string(),
//                 hub_addr: HUB.to_string(),
//                 ibc_timeout_seconds,
//             },
//         )
//         .unwrap_err();

//         assert_eq!(
//             err,
//             ContractError::InvalidIBCTimeout {
//                 timeout: ibc_timeout_seconds,
//                 min: MIN_IBC_TIMEOUT_SECONDS,
//                 max: MAX_IBC_TIMEOUT_SECONDS
//             }
//         );
//     }
// }

use cosmwasm_std::{entry_point, DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;

use neutron_sdk::bindings::msg::NeutronMsg;
use neutron_sdk::bindings::query::NeutronQuery;

use crate::error::ContractError;
use crate::helpers::validate_channel;
use crate::msg::{InstantiateMsg, MigrateMsg};
use crate::state::CONFIG;
use crate::types::{Config, MAX_IBC_TIMEOUT_SECONDS, MIN_IBC_TIMEOUT_SECONDS};

/// Contract name that is used for migration
const CONTRACT_NAME: &str = "asteroid-bridge";
/// Contract version that is used for migration
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Instantiates the bridge contract, storing the config.
/// Returns a `Response` object on successful execution or a `ContractError` on failure.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<NeutronQuery>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<NeutronMsg>, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // The bridge IBC channel must be specified, that is, the channel used
    // to send information back to the source chain
    if msg.bridge_ibc_channel.is_empty() {
        return Err(ContractError::InvalidConfiguration {
            reason: "The bridge IBC channel must be specified".to_string(),
        });
    }

    // Ensure the IBC channel exists with transfer port
    // Unlike regular IBC token transfers where the channel is important, in
    // this bridge the channel is used to send information back to the source
    // chain but has no bearing on the denom of a token
    validate_channel(deps.querier, &msg.bridge_ibc_channel)?;

    // The source chain ID must be specified, that is, the chain ID of the
    // source chain, not the chain ID where this contract is deployed
    if msg.bridge_chain_id.is_empty() {
        return Err(ContractError::InvalidConfiguration {
            reason: "The source chain ID must be specified".to_string(),
        });
    }

    // Ensure valid IBC timeouts are set
    if !(MIN_IBC_TIMEOUT_SECONDS..=MAX_IBC_TIMEOUT_SECONDS).contains(&msg.ibc_timeout_seconds) {
        return Err(ContractError::InvalidIBCTimeout {
            timeout: msg.ibc_timeout_seconds,
            min: MIN_IBC_TIMEOUT_SECONDS,
            max: MAX_IBC_TIMEOUT_SECONDS,
        });
    }

    let config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        bridge_chain_id: msg.bridge_chain_id.clone(),
        bridge_ibc_channel: msg.bridge_ibc_channel.clone(),
        ibc_timeout_seconds: msg.ibc_timeout_seconds,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default()
        .add_attribute("action", "instantiate")
        .add_attribute("bridge_chain_id", msg.bridge_chain_id)
        .add_attribute("bridge_ibc_channel", msg.bridge_ibc_channel))
}

/// Migrates the contract to a new version, storing the new contract version.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    let contract_version = cw2::get_contract_version(deps.storage)?;

    match contract_version.contract.as_ref() {
        "asteroid-bridge" => match contract_version.version.as_ref() {
            "1.0.1" => {
                let mut config = CONFIG.load(deps.storage)?;

                // Allow changing the source chain ID in case the source chain
                // undergoes an upgrade that changes the chain ID
                if let Some(bridge_chain_id) = msg.bridge_chain_id {
                    if bridge_chain_id.is_empty() {
                        return Err(ContractError::InvalidConfiguration {
                            reason: "The source chain ID must be specified".to_string(),
                        });
                    }
                    config.bridge_chain_id = bridge_chain_id;
                }

                CONFIG.save(deps.storage, &config)?;
            }
            _ => return Err(ContractError::MigrationError {}),
        },
        _ => return Err(ContractError::MigrationError {}),
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("previous_contract_name", &contract_version.contract)
        .add_attribute("previous_contract_version", &contract_version.version)
        .add_attribute("new_contract_name", CONTRACT_NAME)
        .add_attribute("new_contract_version", CONTRACT_VERSION))
}

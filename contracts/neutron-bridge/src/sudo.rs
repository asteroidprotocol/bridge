use cosmwasm_std::{entry_point, DepsMut, Env, Response};
use neutron_sdk::{bindings::msg::NeutronMsg, sudo::msg::TransferSudoMsg};

use crate::{error::ContractError, helpers::build_mint_messages, state::BRIDGE_INFLIGHT};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(
    deps: DepsMut,
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
                .add_attribute("action", "response")
                .add_attribute("request", format!("response done: {:?}", sequence_id)))
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
                .add_attribute("action", "response")
                .add_attribute("request", format!("response error: {:?}", sequence_id)))
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
                .add_attribute("action", "response")
                .add_attribute("request", format!("response timeout: {:?}", sequence_id)))
        }
    }
}

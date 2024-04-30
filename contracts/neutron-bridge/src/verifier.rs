use base64::{engine::general_purpose, Engine as _};
use cosmwasm_std::{Deps, Order};
use neutron_sdk::bindings::query::NeutronQuery;

use crate::{
    error::ContractError,
    state::{CONFIG, SIGNERS},
    // types::Verifier,
};

/// Verify the signatures against the current loaded public keys
/// Once we reach the valid threshold, we return Ok
/// If we don't have enough valid signatures, we return Err
pub fn verify_signatures(
    deps: Deps<NeutronQuery>,
    message: &[u8],
    signatures: &[String],
) -> Result<(), ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // If no signatures were sent, fail the verification
    if signatures.is_empty() {
        return Err(ContractError::ThresholdNotMet {});
    }

    // If duplicate signatures are sent, fail the verification
    let mut unique_signatures = signatures.to_vec();
    unique_signatures.sort();
    unique_signatures.dedup();
    if unique_signatures.len() != signatures.len() {
        return Err(ContractError::DuplicateSignatures {});
    }

    // If the number of unique signatures are less than the threshold, fail the verification
    if unique_signatures.len() < config.signer_threshold.into() {
        return Err(ContractError::ThresholdNotMet {});
    }

    // Load the current allowed public keys
    let allowed_keys = SIGNERS.keys(deps.storage, None, None, Order::Ascending);

    let mut verified_signatures = 0;

    // Decode signatures from base64
    let decoded_signatures: Result<Vec<_>, _> = unique_signatures
        .iter()
        .map(|sig| general_purpose::STANDARD.decode(sig))
        .collect();
    let decoded_signatures = decoded_signatures?;

    // Verify the signatures against the loaded keys
    // This requires iterating over the signatures and the loaded keys
    // and thus we should not keep too many keys loaded
    //
    // While there is a possbility for this to be unbounded, the number of keys
    // will be small. The decision to use this method vs
    // sending keys with their signatures was made to simplify interactions
    // with the contract
    for loaded_key in allowed_keys {
        let allowed_key = loaded_key?;

        for signature in &decoded_signatures {
            let is_valid = deps.api.ed25519_verify(message, signature, &allowed_key)?;
            if is_valid {
                verified_signatures += 1;
                if verified_signatures >= config.signer_threshold {
                    return Ok(());
                }
                // We can move on to the next key and signatures
                break;
            }
        }
    }
    // If we reach this point, we did not have enough valid signatures
    Err(ContractError::ThresholdNotMet {})
}

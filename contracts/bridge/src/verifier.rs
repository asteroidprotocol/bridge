use base64::{engine::general_purpose, Engine as _};
use cosmwasm_std::{Deps, DepsMut};
use neutron_sdk::bindings::query::NeutronQuery;

use crate::{
    error::ContractError,
    state::{CONFIG, SIGNERS},
    types::Verifier,
};

/// Verify the message and signatures against the current loaded public keys
/// Once we reach the valid threshold, we return Ok
/// If we don't have enough valid signatures, we return Err
pub fn verify_signatures(
    deps: Deps<NeutronQuery>,
    message: &[u8],
    verifiers: &[Verifier],
) -> Result<(), ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // If no signatures were sent, fail the verification
    if verifiers.is_empty() {
        return Err(ContractError::NoSigners {});
    }

    // If we have no signers loaded, we can't verify anything
    if SIGNERS.is_empty(deps.storage) {
        return Err(ContractError::NoSigners {});
    }

    // Verify the signatures
    let mut verified_signatures = 0;
    for verifier in verifiers {
        // Verify that the keys sent as the signers are loaded
        let public_key =
            match general_purpose::STANDARD.decode(verifier.public_key_base64.as_bytes()) {
                Ok(bytes) => bytes,
                Err(e) => panic!("Failed to decode public key base64: {}", e),
            };
        if !SIGNERS.has(deps.storage, &public_key) {
            return Err(ContractError::VerifierNotLoaded {
                public_key_base64: verifier.public_key_base64.to_string(),
            });
        }

        let signature = general_purpose::STANDARD.decode(&verifier.signature_base64)?;

        // If the verifier is loaded, we can check the signature. If it is valid
        // we increment the verified_signatures counter
        let is_valid = deps.api.ed25519_verify(message, &signature, &public_key)?;
        if !is_valid {
            return Err(ContractError::ThresholdNotMet {});
        }

        // If it fails, return immediately
        verified_signatures += 1;
        // If we have enough valid signatures, we don't need to check the rest
        if verified_signatures >= config.signer_threshold {
            return Ok(());
        }
    }

    // If we reach here, we don't have enough valid signatures and thus
    // consider the message invalid
    Err(ContractError::ThresholdNotMet {})
}

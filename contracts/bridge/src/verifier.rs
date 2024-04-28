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
        return Err(ContractError::NoSigners {});
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

// #[cfg(test)]
// mod tests {

//     use crate::error::ContractError;
//     use crate::msg::QueryMsg;
//     use crate::state::{CONFIG, SIGNERS};
//     use crate::tests::mock_neutron_dependencies;
//     use crate::types::Verifier;
//     use crate::verifier::verify_signatures;
//     use base64::{engine::general_purpose, Engine as _};
//     use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
//     use cosmwasm_std::{
//         coins, from_binary, to_binary, Addr, Api, Binary, Env, MessageInfo, OwnedDeps, Storage,
//         Uint128, WasmQuery,
//     };

//     #[test]
//     fn test_valid_signature() {
//         let mut deps = mock_neutron_dependencies();
//         let info = mock_info(&String::from("anyone"), &[]);
//         let env = mock_env();

//         let attestation = format!(
//             // source_chain_id, transaction_hash, ticker, amount
//             "{}{}{}{}{}{}{}",
//             "gaialocal-1",
//             "C4B49EE668189C7724AE5B8492867C65A7FF6DE3EB4F443C42B8DD48F6630CC1",
//             "LOCALROIDS",
//             "10000000",
//             "neutronlocal-1",
//             "neutron1m0z0kk0qqug74n9u9ul23e28x5fszr628h20xwt6jywjpp64xn4qatgvm0",
//             "neutron1vrmfyhxjlpg32e68f5tg7qn9uftyn68u70trzs"
//         );

//         let public_key1 = "b577zulJVqWfXiip7ydZrvMgp2SzfR+IXhH7vkUjr+Y="; // Base64-encoded public key
//         let signature1 = "ZRNhbFe89DteWUy9es98IqGmqUoy5e0WXoBJbOAG85x+ehUIr1TQO/lD54SFrxla8Us3464xz6Rl87l8tzFcDQ=="; // Base64-encoded signature

//         // let public_key2 = "..."; // Base64-encoded public key
//         // let signature2 = "..."; // Base64-encoded signature

//         let verifiers = vec![
//             Verifier {
//                 public_key_base64: public_key1.to_string(),
//                 signature_base64: signature1.to_string(),
//             },
//             // Verifier {
//             //     public_key_base64: public_key2.to_string(),
//             //     signature_base64: signature2.to_string(),
//             // },
//         ];

//         // Test with valid signatures
//         let res = verify_signatures(deps.as_ref(), attestation.as_bytes(), &verifiers);
//         assert!(res.is_ok());

//         // // Test with invalid signature
//         // let invalid_signature = "..."; // Base64-encoded invalid signature
//         // let verifiers = vec![
//         //     Verifier {
//         //         public_key_base64: public_key1.to_string(),
//         //         signature_base64: signature1.to_string(),
//         //     },
//         //     Verifier {
//         //         public_key_base64: public_key2.to_string(),
//         //         signature_base64: invalid_signature.to_string(),
//         //     },
//         // ];

//         // let res = crate::verifier::verify_signatures(deps.as_ref(), message, &verifiers);
//         // assert_eq!(res.unwrap_err(), ContractError::ThresholdNotMet {});
//     }
// }

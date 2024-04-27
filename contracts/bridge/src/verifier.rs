use base64::{engine::general_purpose, Engine as _};
use cosmwasm_std::Deps;
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

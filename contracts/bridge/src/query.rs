use crate::state::{CONFIG, DISABLED_TOKENS, TOKEN_MAPPING};
use crate::types::{QuerySignersResponse, QueryTokensResponse};
use crate::{msg::QueryMsg, state::SIGNERS};

use base64::{engine::general_purpose, Engine as _};
use cosmwasm_std::{entry_point, to_json_binary, Binary, Deps, Env, Order, StdError, StdResult};
use cw_storage_plus::Bound;
use neutron_sdk::bindings::query::NeutronQuery;

// Settings for pagination.
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

/// Expose available contract queries.
///
/// ## Queries
/// * **QueryMsg::Config {}** Returns the config of the Bridge
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<NeutronQuery>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::Signers {} => {
            let signers_result: Result<Vec<_>, cosmwasm_std::StdError> = SIGNERS
                .range(deps.storage, None, None, Order::Ascending)
                .map(|result| {
                    result.map(|(key, value)| (general_purpose::STANDARD.encode(key), value))
                })
                .collect(); // This collects the results into a single Result type

            // Now you need to handle the result of the entire operation
            match signers_result {
                Ok(signers) => to_json_binary(&QuerySignersResponse { signers }),
                Err(e) => Err(e), // Propagate the error
            }
        }
        QueryMsg::Tokens { start_after, limit } => {
            to_json_binary(&query_all_tokens(deps, start_after, limit)?)
        }
        QueryMsg::DisabledTokens { start_after, limit } => {
            to_json_binary(&query_disabled_tokens(deps, start_after, limit)?)
        }
        QueryMsg::TestVerifySignature {
            public_key_base64,
            signature_base64,
            attestation,
        } => test_verify_signature(deps, public_key_base64, signature_base64, attestation),
    }
}

fn test_verify_signature(
    deps: Deps<NeutronQuery>,
    public_key_base64: String,
    signature_base64: String,
    attestation: String,
) -> StdResult<Binary> {
    let attestation_bytes = attestation.as_bytes();

    // Decode the base64 encoded signature
    let signature = match general_purpose::STANDARD.decode(signature_base64.as_bytes()) {
        Ok(bytes) => bytes,
        Err(e) => panic!("Failed to decode signature: {}", e),
    };

    // Decode the base64 encoded public key
    let public_key = match general_purpose::STANDARD.decode(public_key_base64.as_bytes()) {
        Ok(bytes) => bytes,
        Err(e) => panic!("Failed to decode public key: {}", e),
    };

    // let public_key_bytes: [u8; PUBLIC_KEY_LENGTH] = public_key.try_into().unwrap();

    // let verifying_key = VerifyingKey::from_bytes(&public_key_bytes).unwrap();

    // let signature_bytes: [u8; 64] = signature.try_into().unwrap();

    // let sig = Signature::from_bytes(&signature_bytes);

    // // TODO Return proper error
    // verifying_key
    //     .verify_strict(attestation_bytes, &sig)
    //     .unwrap();

    // println!("Signature verified: {:?}", result);

    // Verify the signature
    // let res = ed25519_verify(attestation_bytes, &signature, &public_key).unwrap();
    let res = deps
        .api
        .ed25519_verify(attestation_bytes, &signature, &public_key)
        .unwrap();
    to_json_binary(&res)
}

// // Handler for the query that lists tokens with pagination
// fn query_list_tokens(
//     deps: Deps,
//     start_after: Option<String>,
//     limit: Option<u32>,
// ) -> StdResult<Binary> {
//     let start_bound = start_after.map(|start| Bound::exclusive(start.as_bytes()));
//     let limit = limit.unwrap_or(10) as usize; // Default to 10 items if no limit is provided

//     let tokens: Vec<_> = TOKEN_MAPPING
//         .range(deps.storage, start_bound, None, Order::Ascending)
//         .take(limit) // Take only up to `limit` items
//         .map(|item| {
//             let (key, value) = item?;
//             Ok((String::from_utf8(key.into())?, value))
//         })
//         .collect::<StdResult<Vec<(String, String)>>>()?;

//     // Convert the tokens vector to binary using to_binary
//     to_json_binary(&tokens)
// }

pub fn query_all_tokens(
    deps: Deps<NeutronQuery>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<QueryTokensResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start_bound = start_after.as_deref().map(Bound::exclusive);

    let tokens = TOKEN_MAPPING
        .keys(deps.storage, start_bound, None, Order::Ascending)
        .take(limit)
        .map(|key_result| key_result.map_err(StdError::from))
        .collect::<StdResult<Vec<String>>>()?;

    Ok(QueryTokensResponse { tokens })
}

pub fn query_disabled_tokens(
    deps: Deps<NeutronQuery>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<QueryTokensResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start_bound = start_after.as_deref().map(Bound::exclusive);

    let tokens = DISABLED_TOKENS
        .keys(deps.storage, start_bound, None, Order::Ascending)
        .take(limit)
        .map(|key_result| key_result.map_err(StdError::from))
        .collect::<StdResult<Vec<String>>>()?;

    Ok(QueryTokensResponse { tokens })
}

// #[cfg(test)]
// mod tests {

//     use super::*;

//     use crate::{
//         contract::instantiate, msg::InstantiateMsg, query::query, tests::mock_neutron_dependencies,
//     };
//     use cosmwasm_std::testing::{mock_env, mock_info};
//     // use neutron_sdk::{
//     //     bindings::{msg::IbcFee, query::NeutronQuery},
//     //     query::min_ibc_fee::MinIbcFeeResponse,
//     // };

//     // Test Cases:
//     //
//     // Expect Success
//     //      - Can verify that the signature is value
//     #[test]
//     fn query_test_signature() {
//         // let (mut deps, env, info) = mock_all(OWNER);

//         let mut deps = mock_neutron_dependencies();
//         let info = mock_info(&String::from("anyone"), &[]);
//         let env = mock_env();

//         let owner = "owner";
//         let ibc_timeout_seconds = 10u64;
//         let bridge_ibc_channel = "channel-0";
//         let bridge_chain_id = "gaia-1";

//         instantiate(
//             deps.as_mut(),
//             env.clone(),
//             info,
//             InstantiateMsg {
//                 owner: owner.to_string(),
//                 signer_threshold: 1,
//                 bridge_chain_id: bridge_chain_id.to_string(),
//                 bridge_ibc_channel: bridge_ibc_channel.to_string(),
//                 ibc_timeout_seconds,
//             },
//         )
//         .unwrap();

//         // Build attestation
//         // parsedURN.ChainID + transactionModel.Hash + tokenModel.Ticker + fmt.Sprintf("%d", amount) + remoteChainId + remoteContract + receiverAddress
//         // gaialocal-1 + 8AD3C9E8F69B86DD2EEBC3C5C0BD329F80BA25502F7EE27CDEDD8AD65AB6FBF4 + T1 + 3000000 + neutronlocal-1 + neutron1234 + neutron98765
//         let message_str = "gaialocal-18AD3C9E8F69B86DD2EEBC3C5C0BD329F80BA25502F7EE27CDEDD8AD65AB6FBF4T13000000neutronlocal-1neutron1234neutron98765";
//         let signature_base64 = "Zkk8EhCkNdGoYiFkYXvv6KHOo/V+2nFt07LmwsY7MJaoZQMyD6uLFljrEC/RYmTKiKcmyVgfEc0m9IlZvRHPCw==";
//         let public_key_base64 = "b577zulJVqWfXiip7ydZrvMgp2SzfR+IXhH7vkUjr+Y=";

//         // Test if the contract can correctly verify the information
//         let verified_signature = query(
//             deps.as_ref(),
//             env,
//             QueryMsg::TestVerifySignature {
//                 public_key_base64: public_key_base64.to_string(),
//                 signature_base64: signature_base64.to_string(),
//                 attestation: message_str.to_string(),
//             },
//         )
//         .unwrap();

//         assert_eq!(verified_signature, to_json_binary(&true).unwrap());
//     }
// }

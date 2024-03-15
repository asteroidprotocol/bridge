use crate::msg::QueryMsg;
use crate::state::CONFIG;

use cosmwasm_crypto::ed25519_verify;
use cosmwasm_std::{entry_point, to_json_binary, Binary, Deps, Env, StdResult};
use data_encoding::BASE64;

/// Expose available contract queries.
///
/// ## Queries
/// * **QueryMsg::Config {}** Returns the config of the Bridge
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::TestVerifySignature {
            public_key_base64,
            signature_base64,
            attestation,
        } => test_verify_signature(public_key_base64, signature_base64, attestation),
    }
}

fn test_verify_signature(
    public_key_base64: String,
    signature_base64: String,
    attestation: String,
) -> StdResult<Binary> {
    let attestation_bytes = attestation.as_bytes();

    // Decode the base64 encoded signature
    let signature = match BASE64.decode(signature_base64.as_bytes()) {
        Ok(bytes) => bytes,
        Err(e) => panic!("Failed to decode signature: {}", e),
    };

    // Decode the base64 encoded public key
    let public_key = match BASE64.decode(public_key_base64.as_bytes()) {
        Ok(bytes) => bytes,
        Err(e) => panic!("Failed to decode public key: {}", e),
    };

    // Verify the signature
    let res = ed25519_verify(attestation_bytes, &signature, &public_key).unwrap();
    to_json_binary(&res)
}

#[cfg(test)]
mod tests {

    use super::*;

    use crate::{contract::instantiate, msg::InstantiateMsg, query::query};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    // Test Cases:
    //
    // Expect Success
    //      - Can verify that the signature is value
    #[test]
    fn query_test_signature() {
        // let (mut deps, env, info) = mock_all(OWNER);

        let mut deps = mock_dependencies();
        let info = mock_info(&String::from("anyone"), &[]);
        let env = mock_env();

        let owner = "owner";
        let ibc_timeout_seconds = 10u64;
        let bridge_ibc_channel = "channel-0";

        instantiate(
            deps.as_mut(),
            env.clone(),
            info,
            InstantiateMsg {
                owner: owner.to_string(),
                bridge_ibc_channel: bridge_ibc_channel.to_string(),
                ibc_timeout_seconds,
            },
        )
        .unwrap();

        // Build attestation
        // parsedURN.ChainID + transactionModel.Hash + tokenModel.Ticker + fmt.Sprintf("%d", amount) + remoteChainId + remoteContract + receiverAddress
        // gaialocal-1 + 8AD3C9E8F69B86DD2EEBC3C5C0BD329F80BA25502F7EE27CDEDD8AD65AB6FBF4 + T1 + 3000000 + neutronlocal-1 + neutron1234 + neutron98765
        let message_str = "gaialocal-18AD3C9E8F69B86DD2EEBC3C5C0BD329F80BA25502F7EE27CDEDD8AD65AB6FBF4T13000000neutronlocal-1neutron1234neutron98765";
        let signature_base64 = "Zkk8EhCkNdGoYiFkYXvv6KHOo/V+2nFt07LmwsY7MJaoZQMyD6uLFljrEC/RYmTKiKcmyVgfEc0m9IlZvRHPCw==";
        let public_key_base64 = "b577zulJVqWfXiip7ydZrvMgp2SzfR+IXhH7vkUjr+Y=";

        // Test if the contract can correctly verify the information
        let verified_signature = query(
            deps.as_ref(),
            env,
            QueryMsg::TestVerifySignature {
                public_key_base64: public_key_base64.to_string(),
                signature_base64: signature_base64.to_string(),
                attestation: message_str.to_string(),
            },
        )
        .unwrap();

        assert_eq!(verified_signature, to_json_binary(&true).unwrap());
    }
}

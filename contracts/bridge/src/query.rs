use crate::msg::QueryMsg;
use crate::state::CONFIG;

use base64::{engine::general_purpose, Engine as _};
use cosmwasm_std::{entry_point, to_json_binary, Binary, Deps, DepsMut, Env, StdResult};
use ed25519_dalek::{Signature, VerifyingKey, PUBLIC_KEY_LENGTH};
use neutron_sdk::bindings::query::NeutronQuery;

/// Expose available contract queries.
///
/// ## Queries
/// * **QueryMsg::Config {}** Returns the config of the Bridge
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<NeutronQuery>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&CONFIG.load(deps.storage)?),
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

#[cfg(test)]
mod tests {

    use std::marker::PhantomData;

    use super::*;

    use crate::{contract::instantiate, msg::InstantiateMsg, query::query, types::FEE_DENOM};
    use cosmwasm_std::{
        coins, from_binary,
        testing::{mock_env, mock_info, MockApi, MockQuerier, MockStorage},
        to_binary, ContractResult, OwnedDeps, SystemResult,
    };
    use neutron_sdk::{
        bindings::{msg::IbcFee, query::NeutronQuery},
        query::min_ibc_fee::MinIbcFeeResponse,
    };

    fn mock_neutron_dependencies(
    ) -> OwnedDeps<MockStorage, MockApi, MockQuerier<NeutronQuery>, NeutronQuery> {
        let neutron_custom_handler = |request: &NeutronQuery| {
            let contract_result: ContractResult<_> = match request {
                NeutronQuery::MinIbcFee {} => to_binary(&MinIbcFeeResponse {
                    min_fee: IbcFee {
                        recv_fee: vec![],
                        ack_fee: coins(10000, FEE_DENOM),
                        timeout_fee: coins(10000, FEE_DENOM),
                    },
                })
                .into(),
                _ => unimplemented!("Unsupported query request: {:?}", request),
            };
            SystemResult::Ok(contract_result)
        };

        OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: MockQuerier::new(&[]).with_custom_handler(neutron_custom_handler),
            custom_query_type: PhantomData,
        }
    }

    // Test Cases:
    //
    // Expect Success
    //      - Can verify that the signature is value
    #[test]
    fn query_test_signature() {
        // let (mut deps, env, info) = mock_all(OWNER);

        let mut deps = mock_neutron_dependencies();
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
                signer_threshold: 1,
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

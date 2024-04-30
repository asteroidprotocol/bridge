use asteroid_neutron_bridge::contract::instantiate;
use asteroid_neutron_bridge::error::ContractError;
use asteroid_neutron_bridge::execute::{execute, reply};
use asteroid_neutron_bridge::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use asteroid_neutron_bridge::query::query;
use asteroid_neutron_bridge::types::{
    Config, QuerySignersResponse, QueryTokensResponse, TokenMetadata, MAX_IBC_TIMEOUT_SECONDS,
    MIN_IBC_TIMEOUT_SECONDS, MIN_SIGNER_THRESHOLD,
};
use astroport_test::cw_multi_test::{AppBuilder, Contract, ContractWrapper, Executor};
use astroport_test::modules::stargate::{MockStargate, StargateApp};
use cosmwasm_std::{Addr, Coin, Uint128};
use neutron_sdk::bindings::msg::NeutronMsg;
use neutron_sdk::bindings::query::NeutronQuery;

type NeutronApp = StargateApp<NeutronMsg, NeutronQuery>;

const VALID_SIGNER_1: &str = "b577zulJVqWfXiip7ydZrvMgp2SzfR+IXhH7vkUjr+Y=";
const VALID_SIGNER_2: &str = "vXRMhQtQNezXhdvYe1xlHYysGaEAJH2WwnV8Fvuuttw=";

// Signatures for TESTTOKEN with 6 decimals
const SIGNATURE_1: &str =
    "OU5aYIcdVHNVFNcg+MLT9uYVfkNHjTN8Pzg7lHmni5AuCC0ln78lJQnCRi8XxaPaxQYrm3TY+2+LeOU6H9j0DQ==";
const SIGNATURE_2: &str =
    "r3pfcIod2/49HHTOC+QRcVuccg2nOqSZsCNulv+McYFsEOPX7TN3PFscdVfavaGmb3mqdM6vF5italUVrJH3DA==";

// Signatures for bridging 1000 TESTTOKEN
const BRIDGE_SIGNATURE_1: &str =
    "ZwoqbZxvNaz06/0ZO+M7g0Ygf5YRKkWYNcm/yD+wYQ43N9/9i5xiSHxMhOo0wttNf5NP/T7Rrlv1Sp3K8qyiCw==";
const BRIDGE_SIGNATURE_2: &str =
    "+Y5UhcFimBzBnJX8BIFZPR2DjUp3DaYVRF81osV/qx8E4gDWk3z1EtUsLX3oITTld0lc12IQGdpuFcCWDAMVAQ==";

fn mock_app(owner: &Addr, coins: Vec<Coin>) -> NeutronApp {
    AppBuilder::new_custom()
        // .with_stargate(StargateKeeper::default())
        // .with_custom(NeutronMockModule::new())
        .with_stargate(MockStargate::default())
        // .with_custom(custom)
        // .with_custom(NeutronMockModule::new())
        // .with_wasm::<FailingModule<NeutronMsg, NeutronQuery, Empty>, WasmKeeper<_, _>>(
        //     WasmKeeper::new(),
        // )
        .build(|router, _, storage| {
            // initialization moved to App construction
            router.bank.init_balance(storage, owner, coins).unwrap()
        })
}

fn bridge_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    Box::new(ContractWrapper::new(execute, instantiate, query).with_reply(reply))
}

#[test]
fn test_instantiate() {
    let owner = Addr::unchecked("owner");
    let mut app = mock_app(
        &owner,
        vec![Coin {
            denom: "untrn".to_string(),
            amount: Uint128::from(1000000u64),
        }],
    );
    let contract_code = app.store_code(bridge_contract());

    // Valid configuration
    let bridge_address = app
        .instantiate_contract(
            contract_code,
            owner.clone(),
            &InstantiateMsg {
                owner: owner.to_string(),
                signer_threshold: 2,
                ibc_timeout_seconds: 10,
                bridge_ibc_channel: "channel-1".to_string(),
                bridge_chain_id: "localgaia-1".to_string(),
            },
            &[],
            "Asteroid Bridge",
            None,
        )
        .unwrap();

    // Query to check all the values were set
    let response: Config = app
        .wrap()
        .query_wasm_smart(bridge_address, &QueryMsg::Config {})
        .unwrap();

    assert_eq!(response.signer_threshold, 2);
    assert_eq!(response.bridge_chain_id, "localgaia-1");
    assert_eq!(response.bridge_ibc_channel, "channel-1");
    assert_eq!(response.ibc_timeout_seconds, 10);

    // Test invalid configurations
    let err = app
        .instantiate_contract(
            contract_code,
            owner.clone(),
            &InstantiateMsg {
                owner: owner.to_string(),
                signer_threshold: 0,
                ibc_timeout_seconds: 10,
                bridge_ibc_channel: "channel-1".to_string(),
                bridge_chain_id: "localgaia-1".to_string(),
            },
            &[],
            "Asteroid Bridge",
            None,
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidConfiguration {
            reason: format!(
                "Invalid signer threshold, the minimum is {}",
                MIN_SIGNER_THRESHOLD
            )
        }
    );

    let err = app
        .instantiate_contract(
            contract_code,
            owner.clone(),
            &InstantiateMsg {
                owner: owner.to_string(),
                signer_threshold: 2,
                ibc_timeout_seconds: MIN_IBC_TIMEOUT_SECONDS - 1,
                bridge_ibc_channel: "channel-1".to_string(),
                bridge_chain_id: "localgaia-1".to_string(),
            },
            &[],
            "Asteroid Bridge",
            None,
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidIBCTimeout {
            timeout: MIN_IBC_TIMEOUT_SECONDS - 1,
            min: MIN_IBC_TIMEOUT_SECONDS,
            max: MAX_IBC_TIMEOUT_SECONDS,
        }
    );

    let err = app
        .instantiate_contract(
            contract_code,
            owner.clone(),
            &InstantiateMsg {
                owner: owner.to_string(),
                signer_threshold: 2,
                ibc_timeout_seconds: MAX_IBC_TIMEOUT_SECONDS + 1,
                bridge_ibc_channel: "channel-1".to_string(),
                bridge_chain_id: "localgaia-1".to_string(),
            },
            &[],
            "Asteroid Bridge",
            None,
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidIBCTimeout {
            timeout: MAX_IBC_TIMEOUT_SECONDS + 1,
            min: MIN_IBC_TIMEOUT_SECONDS,
            max: MAX_IBC_TIMEOUT_SECONDS,
        }
    );

    let err = app
        .instantiate_contract(
            contract_code,
            owner.clone(),
            &InstantiateMsg {
                owner: owner.to_string(),
                signer_threshold: 2,
                ibc_timeout_seconds: 10,
                bridge_ibc_channel: "".to_string(),
                bridge_chain_id: "localgaia-1".to_string(),
            },
            &[],
            "Asteroid Bridge",
            None,
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidConfiguration {
            reason: "The bridge IBC channel must be specified".to_string()
        }
    );

    let err = app
        .instantiate_contract(
            contract_code,
            owner.clone(),
            &InstantiateMsg {
                owner: owner.to_string(),
                signer_threshold: 2,
                ibc_timeout_seconds: 10,
                bridge_ibc_channel: "channel-0".to_string(),
                bridge_chain_id: "".to_string(),
            },
            &[],
            "Asteroid Bridge",
            None,
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidConfiguration {
            reason: "The source chain ID must be specified".to_string()
        }
    );
}

#[test]
fn test_add_signer() {
    let owner = Addr::unchecked("owner");
    let not_owner = Addr::unchecked("not_owner");
    let mut app = mock_app(&owner, vec![]);
    let contract_code = app.store_code(bridge_contract());

    let bridge_address = app
        .instantiate_contract(
            contract_code,
            owner.clone(),
            &InstantiateMsg {
                owner: owner.to_string(),
                signer_threshold: 2,
                ibc_timeout_seconds: 10,
                bridge_ibc_channel: "channel-0".to_string(),
                bridge_chain_id: "localgaia-1".to_string(),
            },
            &[],
            "Asteroid Bridge",
            None,
        )
        .unwrap();

    // Add invalid signers
    let err = app
        .execute_contract(
            owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::AddSigner {
                name: "signer".to_string(),
                public_key_base64: "invalid_key".to_string(),
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidConfiguration {
            reason: "Key could not be decoded".to_string()
        }
    );

    // Add a valid signer
    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::AddSigner {
            name: "signer".to_string(),
            public_key_base64: VALID_SIGNER_1.to_string(),
        },
        &[],
    )
    .unwrap();

    // Add a duplicate signer
    let err = app
        .execute_contract(
            owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::AddSigner {
                name: "duplicate-signer".to_string(),
                public_key_base64: VALID_SIGNER_1.to_string(),
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidConfiguration {
            reason: "The public key has already been loaded".to_string()
        }
    );

    // Attempt to add a signer without being the owner
    let err = app
        .execute_contract(
            not_owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::AddSigner {
                name: "duplicate-signer".to_string(),
                public_key_base64: VALID_SIGNER_1.to_string(),
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::Unauthorized {}
    );

    // Query to check new signer was added
    let response: QuerySignersResponse = app
        .wrap()
        .query_wasm_smart(&bridge_address, &QueryMsg::Signers {})
        .unwrap();

    assert_eq!(response.signers.len(), 1);
}

#[test]
fn test_remove_signer() {
    let owner = Addr::unchecked("owner");
    let not_owner = Addr::unchecked("not_owner");
    let mut app = mock_app(&owner, vec![]);
    let contract_code = app.store_code(bridge_contract());

    let bridge_address = app
        .instantiate_contract(
            contract_code,
            owner.clone(),
            &InstantiateMsg {
                owner: owner.to_string(),
                signer_threshold: 2,
                ibc_timeout_seconds: 10,
                bridge_ibc_channel: "channel-0".to_string(),
                bridge_chain_id: "localgaia-1".to_string(),
            },
            &[],
            "Asteroid Bridge",
            None,
        )
        .unwrap();

    // Add a valid signer
    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::AddSigner {
            name: "signer".to_string(),
            public_key_base64: VALID_SIGNER_1.to_string(),
        },
        &[],
    )
    .unwrap();

    // Remove an unknown signer
    let err = app
        .execute_contract(
            owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::RemoveSigner {
                public_key_base64: "aW52YWxpZC1zaWduZXI=".to_string(),
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidConfiguration {
            reason: "Key to remove doesn't exist".to_string()
        }
    );

    // Remove an unknown signer
    let err = app
        .execute_contract(
            owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::RemoveSigner {
                public_key_base64: "invalid_key".to_string(),
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidConfiguration {
            reason: "Key could not be decoded".to_string()
        }
    );

    // Attempt to remove a signer without being the owner
    let err = app
        .execute_contract(
            not_owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::RemoveSigner {
                public_key_base64: "aW52YWxpZC1zaWduZXI=".to_string(),
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::Unauthorized {}
    );

    // Remove a known signer
    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::RemoveSigner {
            public_key_base64: VALID_SIGNER_1.to_string(),
        },
        &[],
    )
    .unwrap();

    // Ensure signer was removed
    let response: QuerySignersResponse = app
        .wrap()
        .query_wasm_smart(&bridge_address, &QueryMsg::Signers {})
        .unwrap();

    assert_eq!(response.signers.len(), 0);
}

#[test]
fn test_update_config() {
    let owner = Addr::unchecked("owner");
    let not_owner = Addr::unchecked("not_owner");
    let mut app = mock_app(&owner, vec![]);
    let contract_code = app.store_code(bridge_contract());

    let bridge_address = app
        .instantiate_contract(
            contract_code,
            owner.clone(),
            &InstantiateMsg {
                owner: owner.to_string(),
                signer_threshold: 2,
                ibc_timeout_seconds: 10,
                bridge_ibc_channel: "channel-0".to_string(),
                bridge_chain_id: "localgaia-1".to_string(),
            },
            &[],
            "Asteroid Bridge",
            None,
        )
        .unwrap();

    // Attempt to update config without being the owner
    let err = app
        .execute_contract(
            not_owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::UpdateConfig {
                signer_threshold: Some(1),
                bridge_chain_id: None,
                bridge_ibc_channel: None,
                ibc_timeout_seconds: None,
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::Unauthorized {}
    );

    // Attempt to update config with invalid signer threshold
    let err = app
        .execute_contract(
            owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::UpdateConfig {
                signer_threshold: Some(MIN_SIGNER_THRESHOLD - 1),
                bridge_chain_id: None,
                bridge_ibc_channel: None,
                ibc_timeout_seconds: None,
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidConfiguration {
            reason: format!(
                "Invalid signer threshold, the minimum is {}",
                MIN_SIGNER_THRESHOLD
            )
        }
    );

    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::UpdateConfig {
            signer_threshold: Some(MIN_SIGNER_THRESHOLD + 1),
            bridge_chain_id: None,
            bridge_ibc_channel: None,
            ibc_timeout_seconds: None,
        },
        &[],
    )
    .unwrap();

    // Attempt blank chain ID
    let err = app
        .execute_contract(
            owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::UpdateConfig {
                signer_threshold: None,
                bridge_chain_id: Some("".to_string()),
                bridge_ibc_channel: None,
                ibc_timeout_seconds: None,
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidConfiguration {
            reason: "The source chain ID must be specified".to_string()
        }
    );

    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::UpdateConfig {
            signer_threshold: None,
            bridge_chain_id: Some("newgaia-1".to_string()),
            bridge_ibc_channel: None,
            ibc_timeout_seconds: None,
        },
        &[],
    )
    .unwrap();

    // Attempt blank ibc channel
    let err = app
        .execute_contract(
            owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::UpdateConfig {
                signer_threshold: None,
                bridge_chain_id: None,
                bridge_ibc_channel: Some("".to_string()),
                ibc_timeout_seconds: None,
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidConfiguration {
            reason: "The bridge IBC channel must be specified".to_string()
        }
    );

    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::UpdateConfig {
            signer_threshold: None,
            bridge_chain_id: None,
            bridge_ibc_channel: Some("channel-9".to_string()),
            ibc_timeout_seconds: None,
        },
        &[],
    )
    .unwrap();

    // Attempt invalid IBC timeout
    let err = app
        .execute_contract(
            owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::UpdateConfig {
                signer_threshold: None,
                bridge_chain_id: None,
                bridge_ibc_channel: None,
                ibc_timeout_seconds: Some(MIN_IBC_TIMEOUT_SECONDS - 1),
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidIBCTimeout {
            timeout: MIN_IBC_TIMEOUT_SECONDS - 1,
            min: MIN_IBC_TIMEOUT_SECONDS,
            max: MAX_IBC_TIMEOUT_SECONDS,
        }
    );

    let err = app
        .execute_contract(
            owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::UpdateConfig {
                signer_threshold: None,
                bridge_chain_id: None,
                bridge_ibc_channel: None,
                ibc_timeout_seconds: Some(MAX_IBC_TIMEOUT_SECONDS + 1),
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidIBCTimeout {
            timeout: MAX_IBC_TIMEOUT_SECONDS + 1,
            min: MIN_IBC_TIMEOUT_SECONDS,
            max: MAX_IBC_TIMEOUT_SECONDS,
        }
    );

    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::UpdateConfig {
            signer_threshold: None,
            bridge_chain_id: None,
            bridge_ibc_channel: None,
            ibc_timeout_seconds: Some(MIN_IBC_TIMEOUT_SECONDS + 1),
        },
        &[],
    )
    .unwrap();

    // Query to check all the new values were set
    let response: Config = app
        .wrap()
        .query_wasm_smart(&bridge_address, &QueryMsg::Config {})
        .unwrap();

    assert_eq!(response.signer_threshold, MIN_SIGNER_THRESHOLD + 1);
    assert_eq!(response.bridge_chain_id, "newgaia-1");
    assert_eq!(response.bridge_ibc_channel, "channel-9");
    assert_eq!(response.ibc_timeout_seconds, MIN_IBC_TIMEOUT_SECONDS + 1);
}

#[test]
fn test_link_token() {
    let owner = Addr::unchecked("owner");
    let mut app = mock_app(&owner, vec![]);
    let contract_code = app.store_code(bridge_contract());

    let bridge_address = app
        .instantiate_contract(
            contract_code,
            owner.clone(),
            &InstantiateMsg {
                owner: owner.to_string(),
                signer_threshold: 2,
                ibc_timeout_seconds: 10,
                bridge_ibc_channel: "channel-0".to_string(),
                bridge_chain_id: "localgaia-1".to_string(),
            },
            &[],
            "Asteroid Bridge",
            None,
        )
        .unwrap();

    // Add a valid signer
    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::AddSigner {
            name: "signer1".to_string(),
            public_key_base64: VALID_SIGNER_1.to_string(),
        },
        &[],
    )
    .unwrap();

    // Add a second valid signer
    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::AddSigner {
            name: "signer2".to_string(),
            public_key_base64: VALID_SIGNER_2.to_string(),
        },
        &[],
    )
    .unwrap();

    // Signatures for TESTTOKEN with 6 decimals
    let signature_1 =
        "OU5aYIcdVHNVFNcg+MLT9uYVfkNHjTN8Pzg7lHmni5AuCC0ln78lJQnCRi8XxaPaxQYrm3TY+2+LeOU6H9j0DQ=="
            .to_string();
    let signature_2 =
        "r3pfcIod2/49HHTOC+QRcVuccg2nOqSZsCNulv+McYFsEOPX7TN3PFscdVfavaGmb3mqdM6vF5italUVrJH3DA=="
            .to_string();

    // Duplicate signatures
    let err = app
        .execute_contract(
            owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::LinkToken {
                source_chain_id: "localgaia-1".to_string(),
                token: TokenMetadata {
                    ticker: "TESTTOKEN".to_string(),
                    name: "TestToken".to_string(),
                    image_url: "https://example.com".to_string(),
                    decimals: 6,
                },
                signatures: vec![signature_1.clone(), signature_1.clone()],
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::DuplicateSignatures {}
    );

    // Invalid signatures
    let err = app
        .execute_contract(
            owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::LinkToken {
                source_chain_id: "localgaia-1".to_string(),
                token: TokenMetadata {
                    ticker: "NOT_TESTTOKEN".to_string(),
                    name: "TestToken".to_string(),
                    image_url: "https://example.com".to_string(),
                    decimals: 6,
                },
                signatures: vec![signature_1.clone(), signature_2.clone()],
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::ThresholdNotMet {}
    );

    // Below threshold signatures
    let err = app
        .execute_contract(
            owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::LinkToken {
                source_chain_id: "localgaia-1".to_string(),
                token: TokenMetadata {
                    ticker: "NOT_TESTTOKEN".to_string(),
                    name: "TestToken".to_string(),
                    image_url: "https://example.com".to_string(),
                    decimals: 6,
                },
                signatures: vec![signature_1.clone()],
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::ThresholdNotMet {}
    );

    // No signatures
    let err = app
        .execute_contract(
            owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::LinkToken {
                source_chain_id: "localgaia-1".to_string(),
                token: TokenMetadata {
                    ticker: "NOT_TESTTOKEN".to_string(),
                    name: "TestToken".to_string(),
                    image_url: "https://example.com".to_string(),
                    decimals: 6,
                },
                signatures: vec![],
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::ThresholdNotMet {}
    );

    // Valid signatures
    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::LinkToken {
            source_chain_id: "localgaia-1".to_string(),
            token: TokenMetadata {
                ticker: "TESTTOKEN".to_string(),
                name: "TestToken".to_string(),
                image_url: "https://example.com".to_string(),
                decimals: 6,
            },
            signatures: vec![signature_1.clone(), signature_2.clone()],
        },
        &[],
    )
    .unwrap();

    // Ensure the token was actually set up correctly
    // Query to check all the new values were set
    let response: QueryTokensResponse = app
        .wrap()
        .query_wasm_smart(
            &bridge_address,
            &QueryMsg::Tokens {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    // We should have two tokens listed in this response as we add a mapping for
    // CFT-20 <> TokenFactory and TokenFactory <> CFT-20
    assert_eq!(response.tokens.len(), 2);
    assert_eq!(response.tokens[0], "TESTTOKEN");

    // Attempt to add a duplicate
    let err = app
        .execute_contract(
            owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::LinkToken {
                source_chain_id: "localgaia-1".to_string(),
                token: TokenMetadata {
                    ticker: "TESTTOKEN".to_string(),
                    name: "TestToken".to_string(),
                    image_url: "https://example.com".to_string(),
                    decimals: 6,
                },
                signatures: vec![signature_1, signature_2],
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::TokenAlreadyExists {
            ticker: "TESTTOKEN".to_string()
        }
    );
}

#[test]
fn test_enable_disable_token() {
    let owner = Addr::unchecked("owner");
    let not_owner = Addr::unchecked("not_owner");
    let mut app = mock_app(&owner, vec![]);
    let contract_code = app.store_code(bridge_contract());

    let bridge_address = app
        .instantiate_contract(
            contract_code,
            owner.clone(),
            &InstantiateMsg {
                owner: owner.to_string(),
                signer_threshold: 2,
                ibc_timeout_seconds: 10,
                bridge_ibc_channel: "channel-0".to_string(),
                bridge_chain_id: "localgaia-1".to_string(),
            },
            &[],
            "Asteroid Bridge",
            None,
        )
        .unwrap();

    // Add a valid signer
    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::AddSigner {
            name: "signer1".to_string(),
            public_key_base64: VALID_SIGNER_1.to_string(),
        },
        &[],
    )
    .unwrap();

    // Add a second valid signer
    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::AddSigner {
            name: "signer2".to_string(),
            public_key_base64: VALID_SIGNER_2.to_string(),
        },
        &[],
    )
    .unwrap();

    // Signatures for TESTTOKEN with 6 decimals
    let signature_1 =
        "OU5aYIcdVHNVFNcg+MLT9uYVfkNHjTN8Pzg7lHmni5AuCC0ln78lJQnCRi8XxaPaxQYrm3TY+2+LeOU6H9j0DQ=="
            .to_string();
    let signature_2 =
        "r3pfcIod2/49HHTOC+QRcVuccg2nOqSZsCNulv+McYFsEOPX7TN3PFscdVfavaGmb3mqdM6vF5italUVrJH3DA=="
            .to_string();

    // Valid signatures
    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::LinkToken {
            source_chain_id: "localgaia-1".to_string(),
            token: TokenMetadata {
                ticker: "TESTTOKEN".to_string(),
                name: "TestToken".to_string(),
                image_url: "https://example.com".to_string(),
                decimals: 6,
            },
            signatures: vec![signature_1.clone(), signature_2.clone()],
        },
        &[],
    )
    .unwrap();

    // Ensure the token was actually set up correctly
    // Query to check all the new values were set
    let response: QueryTokensResponse = app
        .wrap()
        .query_wasm_smart(
            &bridge_address,
            &QueryMsg::Tokens {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    // We should have two tokens listed in this response as we add a mapping for
    // CFT-20 <> TokenFactory and TokenFactory <> CFT-20
    assert_eq!(response.tokens.len(), 2);
    assert_eq!(response.tokens[0], "TESTTOKEN");

    // Enable a token that wasn't disabled
    let err = app
        .execute_contract(
            owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::EnableToken {
                ticker: "TESTTOKEN".to_string(),
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidConfiguration {
            reason: "This token is not disabled".to_string()
        }
    );

    // Disable a token from wrong account
    let err = app
        .execute_contract(
            not_owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::DisableToken {
                ticker: "TESTTOKEN".to_string(),
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::Unauthorized {}
    );

    // Disable invalid token
    let err = app
        .execute_contract(
            owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::DisableToken {
                ticker: "NOT_TESTTOKEN".to_string(),
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::TokenDoesNotExist {
            ticker: "NOT_TESTTOKEN".to_string()
        }
    );

    // Disable token
    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::DisableToken {
            ticker: "TESTTOKEN".to_string(),
        },
        &[],
    )
    .unwrap();

    // Query to check if it was disabled
    let response: QueryTokensResponse = app
        .wrap()
        .query_wasm_smart(
            &bridge_address,
            &QueryMsg::DisabledTokens {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(response.tokens.len(), 1);

    // Enable a token from wrong account
    let err = app
        .execute_contract(
            not_owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::EnableToken {
                ticker: "TESTTOKEN".to_string(),
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::Unauthorized {}
    );

    // Enable token
    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::EnableToken {
            ticker: "TESTTOKEN".to_string(),
        },
        &[],
    )
    .unwrap();

    // Query to check if it was enabled
    let response: QueryTokensResponse = app
        .wrap()
        .query_wasm_smart(
            &bridge_address,
            &QueryMsg::DisabledTokens {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(response.tokens.len(), 0);

    // TODO: Test bridging with disabled token
}

#[test]
fn test_bridge_receive() {
    let owner = Addr::unchecked("owner");
    let not_owner = Addr::unchecked("not_owner");
    let mut app = mock_app(&owner, vec![]);
    let contract_code = app.store_code(bridge_contract());

    let bridge_address = app
        .instantiate_contract(
            contract_code,
            owner.clone(),
            &InstantiateMsg {
                owner: owner.to_string(),
                signer_threshold: 2,
                ibc_timeout_seconds: 10,
                bridge_ibc_channel: "channel-0".to_string(),
                bridge_chain_id: "localgaia-1".to_string(),
            },
            &[],
            "Asteroid Bridge",
            None,
        )
        .unwrap();

    // Add a valid signer
    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::AddSigner {
            name: "signer1".to_string(),
            public_key_base64: VALID_SIGNER_1.to_string(),
        },
        &[],
    )
    .unwrap();

    // Add a second valid signer
    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::AddSigner {
            name: "signer2".to_string(),
            public_key_base64: VALID_SIGNER_2.to_string(),
        },
        &[],
    )
    .unwrap();

    // Receive token not linked yet
    let err = app
        .execute_contract(
            not_owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::Receive {
                source_chain_id: "localgaia-1".to_string(),
                transaction_hash: "TXHASH1".to_string(),
                ticker: "TESTTOKEN".to_string(),
                amount: Uint128::from(1000u64),
                destination_addr: "user1".to_string(),
                signatures: vec![
                    BRIDGE_SIGNATURE_1.to_string().to_string().clone(),
                    BRIDGE_SIGNATURE_2.to_string().to_string().clone(),
                ],
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::TokenDoesNotExist {
            ticker: "TESTTOKEN".to_string()
        }
    );

    // Valid signatures
    app.execute_contract(
        not_owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::LinkToken {
            source_chain_id: "localgaia-1".to_string(),
            token: TokenMetadata {
                ticker: "TESTTOKEN".to_string(),
                name: "TestToken".to_string(),
                image_url: "https://example.com".to_string(),
                decimals: 6,
            },
            signatures: vec![
                SIGNATURE_1.to_string().clone(),
                SIGNATURE_2.to_string().clone(),
            ],
        },
        &[],
    )
    .unwrap();

    // Ensure the token was actually set up correctly
    // Query to check all the new values were set
    let response: QueryTokensResponse = app
        .wrap()
        .query_wasm_smart(
            &bridge_address,
            &QueryMsg::Tokens {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    // We should have two tokens listed in this response as we add a mapping for
    // CFT-20 <> TokenFactory and TokenFactory <> CFT-20
    assert_eq!(response.tokens.len(), 2);
    assert_eq!(response.tokens[0], "TESTTOKEN");

    // Receive token with no signatures
    let err = app
        .execute_contract(
            not_owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::Receive {
                source_chain_id: "localgaia-1".to_string(),
                transaction_hash: "TXHASH1".to_string(),
                ticker: "TESTTOKEN".to_string(),
                amount: Uint128::from(1000u64),
                destination_addr: "user1".to_string(),
                signatures: vec![],
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::ThresholdNotMet {}
    );

    // Receive token with invalid signature for the amount
    let err = app
        .execute_contract(
            not_owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::Receive {
                source_chain_id: "localgaia-1".to_string(),
                transaction_hash: "TXHASH1".to_string(),
                ticker: "TESTTOKEN".to_string(),
                amount: Uint128::from(10000u64),
                destination_addr: "user1".to_string(),
                signatures: vec![
                    BRIDGE_SIGNATURE_1.to_string().clone(),
                    BRIDGE_SIGNATURE_2.to_string().to_string().clone(),
                ],
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::ThresholdNotMet {}
    );

    // Receive zero tokens
    let err = app
        .execute_contract(
            not_owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::Receive {
                source_chain_id: "localgaia-1".to_string(),
                transaction_hash: "TXHASH1".to_string(),
                ticker: "TESTTOKEN".to_string(),
                amount: Uint128::from(0u64),
                destination_addr: "user1".to_string(),
                signatures: vec![
                    BRIDGE_SIGNATURE_1.to_string().clone(),
                    BRIDGE_SIGNATURE_2.to_string().clone(),
                ],
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::ZeroAmount {}
    );

    // Validate that the user has no TESTTOKEN balance
    let res = app.wrap().query_all_balances("user1").unwrap();
    assert_eq!(res.len(), 0);

    // Valid bridge transaction
    app.execute_contract(
        not_owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::Receive {
            source_chain_id: "localgaia-1".to_string(),
            transaction_hash: "TXHASH1".to_string(),
            ticker: "TESTTOKEN".to_string(),
            amount: Uint128::from(1000u64),
            destination_addr: "user1".to_string(),
            signatures: vec![
                BRIDGE_SIGNATURE_1.to_string().clone(),
                BRIDGE_SIGNATURE_2.to_string().clone(),
            ],
        },
        &[],
    )
    .unwrap();

    // Assert that the user received the testtoken
    let res = app.wrap().query_all_balances("user1").unwrap();
    res.iter().for_each(|coin| {
        if coin.denom == "factory/contract0/TESTTOKEN" {
            assert_eq!(coin.amount, Uint128::from(1000u64));
        }
    });

    // Replay the same transaction
    let err = app
        .execute_contract(
            not_owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::Receive {
                source_chain_id: "localgaia-1".to_string(),
                transaction_hash: "TXHASH1".to_string(),
                ticker: "TESTTOKEN".to_string(),
                amount: Uint128::from(1000u64),
                destination_addr: "user1".to_string(),
                signatures: vec![
                    BRIDGE_SIGNATURE_1.to_string().clone(),
                    BRIDGE_SIGNATURE_2.to_string().clone(),
                ],
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::TransactionAlreadyHandled {
            transaction_hash: "TXHASH1".to_string()
        }
    );

    // Try invalid destination address
    let err = app
        .execute_contract(
            not_owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::Receive {
                source_chain_id: "localgaia-1".to_string(),
                transaction_hash: "TXHASH1".to_string(),
                ticker: "TESTTOKEN".to_string(),
                amount: Uint128::from(1000u64),
                destination_addr: "".to_string(),
                signatures: vec![
                    BRIDGE_SIGNATURE_1.to_string().clone(),
                    BRIDGE_SIGNATURE_2.to_string().clone(),
                ],
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidDestinationAddr {}
    );

    // Disable TESTTOKEN
    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::DisableToken {
            ticker: "TESTTOKEN".to_string(),
        },
        &[],
    )
    .unwrap();

    // Attempt to bridge a disabled token
    let err = app
        .execute_contract(
            not_owner.clone(),
            bridge_address.clone(),
            &ExecuteMsg::Receive {
                source_chain_id: "localgaia-1".to_string(),
                transaction_hash: "TXHASH1".to_string(),
                ticker: "TESTTOKEN".to_string(),
                amount: Uint128::from(1000u64),
                destination_addr: "user1".to_string(),
                signatures: vec![
                    BRIDGE_SIGNATURE_1.to_string().clone(),
                    BRIDGE_SIGNATURE_2.to_string().clone(),
                ],
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::TokenDisabled {
            ticker: "TESTTOKEN".to_string()
        }
    );
}

// TODO The Full bridge send test is currently not available due to custom Neutron queries
// that need to be implemented
#[test]
fn test_bridge_send() {
    let owner = Addr::unchecked("owner");
    let not_owner = Addr::unchecked("not_owner");
    let user1 = Addr::unchecked("user1");
    let mut app = mock_app(
        &user1,
        vec![
            Coin {
                denom: "untrn".to_string(),
                amount: Uint128::from(1000000u64),
            },
            Coin {
                denom: "uatom".to_string(),
                amount: Uint128::from(1000000u64),
            },
        ],
    );
    let contract_code = app.store_code(bridge_contract());

    let bridge_address = app
        .instantiate_contract(
            contract_code,
            owner.clone(),
            &InstantiateMsg {
                owner: owner.to_string(),
                signer_threshold: 2,
                ibc_timeout_seconds: 10,
                bridge_ibc_channel: "channel-0".to_string(),
                bridge_chain_id: "localgaia-1".to_string(),
            },
            &[],
            "Asteroid Bridge",
            None,
        )
        .unwrap();

    // Add a valid signer
    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::AddSigner {
            name: "signer1".to_string(),
            public_key_base64: VALID_SIGNER_1.to_string(),
        },
        &[],
    )
    .unwrap();

    // Add a second valid signer
    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::AddSigner {
            name: "signer2".to_string(),
            public_key_base64: VALID_SIGNER_2.to_string(),
        },
        &[],
    )
    .unwrap();

    // Link token
    app.execute_contract(
        not_owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::LinkToken {
            source_chain_id: "localgaia-1".to_string(),
            token: TokenMetadata {
                ticker: "TESTTOKEN".to_string(),
                name: "TestToken".to_string(),
                image_url: "https://example.com".to_string(),
                decimals: 6,
            },
            signatures: vec![
                SIGNATURE_1.to_string().clone(),
                SIGNATURE_2.to_string().clone(),
            ],
        },
        &[],
    )
    .unwrap();

    // Valid bridge transaction
    app.execute_contract(
        not_owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::Receive {
            source_chain_id: "localgaia-1".to_string(),
            transaction_hash: "TXHASH1".to_string(),
            ticker: "TESTTOKEN".to_string(),
            amount: Uint128::from(1000u64),
            destination_addr: "user1".to_string(),
            signatures: vec![
                BRIDGE_SIGNATURE_1.to_string().clone(),
                BRIDGE_SIGNATURE_2.to_string().clone(),
            ],
        },
        &[],
    )
    .unwrap();

    // Assert that the user received the testtoken
    let res = app.wrap().query_all_balances("user1").unwrap();
    res.iter().for_each(|coin| {
        if coin.denom == "factory/contract0/TESTTOKEN" {
            assert_eq!(coin.amount, Uint128::from(1000u64));
        }
    });

    // Check the total supply of the token
    let res = app
        .wrap()
        .query_supply("factory/contract0/TESTTOKEN".to_string())
        .unwrap();
    assert_eq!(res.amount, Uint128::from(1000u64));

    // Send incorrect tokens
    let err = app
        .execute_contract(
            user1.clone(),
            bridge_address.clone(),
            &ExecuteMsg::Send {
                destination_addr: "cosmos1hubaddress".to_string(),
            },
            &[
                // Coin {
                //     denom: "factory/contract0/TESTTOKEN".to_string(),
                //     amount: Uint128::from(1u64),
                // },
                Coin {
                    denom: "untrn".to_string(),
                    amount: Uint128::from(1u64),
                },
            ],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidFunds {}
    );

    let err = app
        .execute_contract(
            user1.clone(),
            bridge_address.clone(),
            &ExecuteMsg::Send {
                destination_addr: "cosmos1hubaddress".to_string(),
            },
            &[
                Coin {
                    denom: "factory/contract0/TESTTOKEN".to_string(),
                    amount: Uint128::from(1u64),
                },
                // Coin {
                //     denom: "untrn".to_string(),
                //     amount: Uint128::from(1u64),
                // },
            ],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidFunds {}
    );

    let err = app
        .execute_contract(
            user1.clone(),
            bridge_address.clone(),
            &ExecuteMsg::Send {
                destination_addr: "cosmos1hubaddress".to_string(),
            },
            &[
                Coin {
                    denom: "uatom".to_string(),
                    amount: Uint128::from(1u64),
                },
                Coin {
                    denom: "untrn".to_string(),
                    amount: Uint128::from(1u64),
                },
            ],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidFunds {}
    );

    // Disable the token
    app.execute_contract(
        owner.clone(),
        bridge_address.clone(),
        &ExecuteMsg::DisableToken {
            ticker: "TESTTOKEN".to_string(),
        },
        &[],
    )
    .unwrap();

    // Try bridging disabled token
    let err = app
        .execute_contract(
            user1.clone(),
            bridge_address.clone(),
            &ExecuteMsg::Send {
                destination_addr: "cosmos1hubaddress".to_string(),
            },
            &[
                Coin {
                    denom: "factory/contract0/TESTTOKEN".to_string(),
                    amount: Uint128::from(1u64),
                },
                Coin {
                    denom: "untrn".to_string(),
                    amount: Uint128::from(1u64),
                },
            ],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::TokenDisabled {
            ticker: "TESTTOKEN".to_string()
        }
    );

    // TODO: The remaining part of this test will be fixed soon

    // TODO: Ensure the user balance was updated

    // Ensure that the total supply was reduced
    // let res = app
    //     .wrap()
    //     .query_supply("factory/contract0/TESTTOKEN".to_string())
    //     .unwrap();
    // assert_eq!(res.amount, Uint128::from(999u64));
}

// pub struct NeutronMockModule {}

// impl NeutronMockModule {
//     pub fn new() -> Self {
//         Self {}
//     }
// }

// impl Module for NeutronMockModule {
//     type ExecT = NeutronMsg;
//     type QueryT = NeutronQuery;
//     type SudoT = Empty;

//     fn execute<ExecC, QueryC>(
//         &self,
//         api: &dyn Api,
//         storage: &mut dyn Storage,
//         router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
//         block: &BlockInfo,
//         _sender: Addr,
//         msg: Self::ExecT,
//     ) -> AnyResult<AppResponse>
//     where
//         ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
//         QueryC: CustomQuery + DeserializeOwned + 'static,
//     {
//         unimplemented!("not implemented")
//     }

//     fn sudo<ExecC, QueryC>(
//         &self,
//         _api: &dyn Api,
//         _storage: &mut dyn Storage,
//         _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
//         _block: &BlockInfo,
//         _msg: Self::SudoT,
//     ) -> AnyResult<AppResponse>
//     where
//         ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
//         QueryC: CustomQuery + DeserializeOwned + 'static,
//     {
//         unimplemented!("not implemented")
//     }

//     fn query(
//         &self,
//         _api: &dyn Api,
//         _storage: &dyn Storage,
//         _querier: &dyn Querier,
//         _block: &BlockInfo,
//         request: Self::QueryT,
//     ) -> AnyResult<Binary> {
//         match request {
//             NeutronQuery::MinIbcFee {} => Ok(to_json_binary(&MinIbcFeeResponse {
//                 min_fee: IbcFee {
//                     ack_fee: [Coin {
//                         denom: "untrn".to_string(),
//                         amount: 1u128.into(),
//                     }]
//                     .to_vec(),
//                     recv_fee: [Coin {
//                         denom: "untrn".to_string(),
//                         amount: 1u128.into(),
//                     }]
//                     .to_vec(),
//                     timeout_fee: [Coin {
//                         denom: "untrn".to_string(),
//                         amount: 1u128.into(),
//                     }]
//                     .to_vec(),
//                 },
//             })?),
//             // InjectiveQuery::SpotMarket { market_id } => {
//             //     // let markets = self.markets.borrow();
//             //     // if let Some((base_denom, quote_denom)) = markets.get(&market_id) {
//             //     //     // TODO: save min_quantity_tick_size and min_price_tick_size somewhere if needed
//             //     //     // as currently they are hardcoded
//             //     //     Ok(to_json_binary(&SpotMarketResponse {
//             //     //         market: Some(SpotMarket {
//             //     //             ticker: base_denom.to_string() + "/" + quote_denom,
//             //     //             market_id,
//             //     //             min_quantity_tick_size: 1000000000000000u128.into(), // from the real INJ/USDT market, 0.001 INJ
//             //     //             base_denom: base_denom.clone(),
//             //     //             quote_denom: quote_denom.clone(),
//             //     //             status: Default::default(),
//             //     //             min_price_tick_size: f64_to_dec::<Decimal256>(0.000000000000001)
//             //     //                 .conv()?, // 0.000000000000001
//             //     //             maker_fee_rate: Default::default(),
//             //     //             taker_fee_rate: Default::default(),
//             //     //             relayer_fee_share_rate: Default::default(),
//             //     //         }),
//             //     //     })?)
//             //     // } else {
//             //     //     Ok(to_json_binary(&SpotMarketResponse { market: None })?)
//             //     // }
//             // }
//             _ => unimplemented!("not implemented"),
//         }
//     }
// }

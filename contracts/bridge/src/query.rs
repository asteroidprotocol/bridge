use crate::msg::QueryMsg;
use crate::state::CONFIG;
use cosmwasm_crypto::ed25519_verify;
use cosmwasm_std::{entry_point, to_json_binary, to_json_string, Binary, Deps, Env, StdResult};
use osmosis_std::types::cosmos::crypto::ed25519;

/// Expose available contract queries.
///
/// ## Queries
/// * **QueryMsg::Config {}** Returns the config of the Bridge
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::TestSignature {
            public_key,
            signature,
            attestation,
        } => test_signature(public_key, signature, attestation),
    }
}

fn test_signature(public_key: String, signature: String, attestation: String) -> StdResult<Binary> {
    println!("\n\n\n===========\nCheck signature!");

    // TODO: Continue here
    ed25519_verify();

    println!("\n===========\n\n\n\n");
    to_json_binary("String")
}

#[cfg(test)]
mod tests {

    use super::*;

    use cosmwasm_std::{testing::mock_info, StdError, Uint64};

    use crate::{contract::instantiate, execute::execute, msg::InstantiateMsg, query::query};

    use cosmwasm_std::testing::{mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage};
    // Test Cases:
    //
    // Expect Success
    //      - Can query for a vote already cast
    //
    // Expect Error
    //      - Must fail if the vote doesn't exist
    //
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

        // Check that we can query the vote that was cast
        let verified_signature = query(
            deps.as_ref(),
            env,
            QueryMsg::TestSignature {
                public_key: "key".to_string(),
                signature: "signature".to_string(),
                attestation: "attest".to_string(),
            },
        )
        .unwrap();

        assert_eq!(verified_signature, to_json_binary(&"success").unwrap());
    }
}

// #[cfg(test)]
// mod tests {

//     use super::*;

//     use cosmwasm_std::{testing::mock_info, StdError, Uint64};

//     use crate::{
//         contract::instantiate,
//         execute::execute,
//         mock::{mock_all, setup_channel, HUB, OWNER, VXASTRO_TOKEN, XASTRO_TOKEN},
//         query::query,
//         state::PROPOSALS_CACHE,
//     };
//     use astroport_governance::{assembly::ProposalVoteOption, interchain::ProposalSnapshot};

//     // Test Cases:
//     //
//     // Expect Success
//     //      - Can query for a vote already cast
//     //
//     // Expect Error
//     //      - Must fail if the vote doesn't exist
//     //
//     #[test]
//     fn query_votes() {
//         let (mut deps, env, info) = mock_all(OWNER);

//         let proposal_id = 1u64;
//         let user = "user";
//         let ibc_timeout_seconds = 10u64;

//         instantiate(
//             deps.as_mut(),
//             env.clone(),
//             info,
//             astroport_governance::outpost::InstantiateMsg {
//                 owner: OWNER.to_string(),
//                 xastro_token_addr: XASTRO_TOKEN.to_string(),
//                 vxastro_token_addr: VXASTRO_TOKEN.to_string(),
//                 hub_addr: HUB.to_string(),
//                 ibc_timeout_seconds,
//             },
//         )
//         .unwrap();

//         // Set up valid Hub
//         setup_channel(deps.as_mut(), env.clone());

//         // Update config with new channel
//         execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(OWNER, &[]),
//             astroport_governance::outpost::ExecuteMsg::UpdateConfig {
//                 hub_addr: None,
//                 hub_channel: Some("channel-3".to_string()),
//                 ibc_timeout_seconds: None,
//             },
//         )
//         .unwrap();

//         // Add a proposal to the cache
//         PROPOSALS_CACHE
//             .save(
//                 &mut deps.storage,
//                 proposal_id,
//                 &ProposalSnapshot {
//                     id: Uint64::from(proposal_id),
//                     start_time: 1689939457,
//                 },
//             )
//             .unwrap();

//         // Cast a vote with a proposal in the cache
//         execute(
//             deps.as_mut(),
//             env.clone(),
//             mock_info(user, &[]),
//             astroport_governance::outpost::ExecuteMsg::CastAssemblyVote {
//                 proposal_id,
//                 vote: astroport_governance::assembly::ProposalVoteOption::For,
//             },
//         )
//         .unwrap();

//         // Check that we can query the vote that was cast
//         let vote_data = query(
//             deps.as_ref(),
//             env.clone(),
//             astroport_governance::outpost::QueryMsg::ProposalVoted {
//                 proposal_id,
//                 user: user.to_string(),
//             },
//         )
//         .unwrap();

//         assert_eq!(vote_data, to_binary(&ProposalVoteOption::For).unwrap());

//         // Check that we receive an error when querying a vote that doesn't exist
//         let err = query(
//             deps.as_ref(),
//             env,
//             astroport_governance::outpost::QueryMsg::ProposalVoted {
//                 proposal_id,
//                 user: "other_user".to_string(),
//             },
//         )
//         .unwrap_err();

//         assert_eq!(
//             err,
//             StdError::NotFound {
//                 kind: "astroport_governance::assembly::ProposalVoteOption".to_string()
//             }
//         );
//     }
// }

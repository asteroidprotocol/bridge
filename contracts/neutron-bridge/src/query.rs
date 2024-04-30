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
/// * **QueryMsg::Signers {}** Returns the current signers and their public keys in base64
/// * **QueryMsg::Tokens { start_after, limit }** Returns the CFT-20 and TokenFactory tokens that can be bridged
/// * **QueryMsg::DisabledTokens { start_after, limit }** Returns the CFT-20 and TokenFactory tokens that have been disabled from bridging},
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<NeutronQuery>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::Signers {} => {
            let signers: Result<Vec<_>, StdError> = SIGNERS
                .range(deps.storage, None, None, Order::Ascending)
                .map(|result| {
                    result.map(|(key, value)| (general_purpose::STANDARD.encode(key), value))
                })
                .collect();

            match signers {
                Ok(signers) => to_json_binary(&QuerySignersResponse { signers }),
                Err(e) => Err(e),
            }
        }
        QueryMsg::Tokens { start_after, limit } => {
            to_json_binary(&query_all_tokens(deps, start_after, limit)?)
        }
        QueryMsg::DisabledTokens { start_after, limit } => {
            to_json_binary(&query_disabled_tokens(deps, start_after, limit)?)
        }
    }
}

/// Queries all tokens that have been added to the bridge
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

/// Queries all disabled tokens
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

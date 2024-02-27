use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

use crate::state::Config;

/// Holds the parameters used for creating a Hub contract
#[cw_serde]
pub struct InstantiateMsg {
    /// The contract owner
    pub owner: String,
}

/// The contract migration message
/// We currently take no arguments for migrations
#[cw_serde]
pub struct MigrateMsg {}

/// Describes the execute messages available in the contract
#[cw_serde]
pub enum ExecuteMsg {
    /// Enable a CFT-20 token to be bridged
    Activate {
        /// The ticker of the CFT-20 token
        ticker: String,
        /// The name of the CFT-20 token
        name: String,
        /// The amount of decimals this CFT-20 uses
        decimals: u32,
        // // TODO: Signature and checking data
    },
    /// Receive CFT-20 token message from the Hub
    Receive {
        /// The ticker of the CFT-20 token
        ticker: String,
        /// The amount of CFT-20 tokens
        amount: Uint128,
        /// The destination address to transfer the CFT-20-equivalent to
        destination_addr: String,
        // // TODO: Signature and checking data
    },
    /// Send CFT-20 token back to the Hub
    Send {
        /// The destination address to transfer the CFT-20-equivalent to
        destination_addr: String,
        // // TODO: Signature and checking data
    },
    UpdateConfig {
        /// The timeout in seconds for IBC packets
        ibc_timeout_seconds: Option<u64>,
    },
}

/// Describes the query messages available in the contract
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the config of the Hub
    #[returns(Config)]
    Config {},
}

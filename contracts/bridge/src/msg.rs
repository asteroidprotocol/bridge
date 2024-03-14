use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

use crate::types::Config;

/// Holds the parameters used for creating a Hub contract
#[cw_serde]
pub struct InstantiateMsg {
    /// The contract owner
    pub owner: String,
    /// The IBC channel to the Cosmos Hub
    pub bridge_ibc_channel: String,
    /// The timeout in seconds for IBC packets
    pub ibc_timeout_seconds: u64,
}

/// The contract migration message
/// We currently take no arguments for migrations
#[cw_serde]
pub struct MigrateMsg {}

/// Describes the execute messages available in the contract
#[cw_serde]
pub enum ExecuteMsg {
    /// Enable a CFT-20 token to be bridged
    EnableToken {
        /// The ticker of the CFT-20 token
        ticker: String,
        /// The name of the CFT-20 token
        name: String,
        /// The URL to the CFT-20 token's image
        image_url: String,
        /// The amount of decimals this CFT-20 uses
        decimals: u32,
    },
    // Disable a token from being bridged
    // DisableToken {
    //     /// The ticker of the CFT-20 token
    //     ticker: String,
    // },
    // /// Receive CFT-20 token message from the Hub
    // Receive {
    //     /// The ticker of the CFT-20 token
    //     ticker: String,
    //     /// The amount of CFT-20 tokens
    //     amount: Uint128,
    //     /// The destination address to transfer the CFT-20-equivalent to
    //     destination_addr: String,
    //     // // TODO: Signature and checking data
    // },
    // /// Send CFT-20 token back to the Hub
    // Send {
    //     /// The destination address to transfer the CFT-20-equivalent to
    //     destination_addr: String,
    //     // // TODO: Signature and checking data
    // },
    /// Adds a signer to the allowed list for signature verification
    AddSigner { public_key: String, name: String },
    /// Remove a signer from the allowed list for signature verification
    RemoveSigner { public_key: String },
    UpdateConfig {
        /// The new IBC channel to the Cosmos Hub to use
        bridge_ibc_channel: Option<String>,
        /// The timeout in seconds for IBC packets
        ibc_timeout_seconds: Option<u64>,
    },
    /// Propose a new owner for the contract
    ProposeNewOwner { owner: String, expires_in: u64 },
    /// Remove the ownership transfer proposal
    DropOwnershipProposal {},
    /// Claim contract ownership
    ClaimOwnership {},
}

/// Describes the query messages available in the contract
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the config of the Bridge
    #[returns(Config)]
    Config {},

    #[returns(String)]
    TestSignature {
        public_key: String,
        signature: String,
        attestation: String,
    }, // /// Returns the allowed signers for signature verification
       // Signers {},
       // /// Returns all the tokens that have been enabled for bridging
       // Tokens {},
}

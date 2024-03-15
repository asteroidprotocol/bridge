use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

use crate::types::{Config, TokenMetadata, Verifier};

/// Holds the parameters used for creating a Hub contract
#[cw_serde]
pub struct InstantiateMsg {
    /// The contract owner
    pub owner: String,
    /// The threshold of signers needed to confirm a message
    pub signer_threshold: u8,
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
    /// Link a CFT-20 token to be bridged
    LinkToken {
        /// The metadata of the CFT-20 token
        token: TokenMetadata,
        /// The signatures of from the verifying parties
        verifiers: Vec<Verifier>,
    },
    // Enable a previously disabled token to being bridged again
    EnableToken {
        /// The ticker of the CFT-20 token
        ticker: String,
    },
    // Disable a token from being bridged
    DisableToken {
        /// The ticker of the CFT-20 token
        ticker: String,
    },
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
    AddSigner {
        /// The public key in base64. This is the raw key without the ASN.1
        /// structure, that is, the last 32 bytes from the DER-encoded public key
        public_key_base64: String,
        /// A simple human name for the owner of the public key
        name: String,
    },
    /// Remove a signer from the allowed list for signature verification
    RemoveSigner {
        /// The public key in base64 to remove. This is the same key added using
        /// AddSigner
        public_key_base64: String,
    },
    UpdateConfig {
        /// The new threshold of signers needed to confirm a message
        signer_threshold: Option<u8>,
        /// The new IBC channel to the Cosmos Hub to use
        bridge_ibc_channel: Option<String>,
        /// The timeout in seconds for IBC packets
        ibc_timeout_seconds: Option<u64>,
    },
    /// Propose a new owner for the contract
    ProposeNewOwner {
        /// The owner being proposed
        owner: String,
        /// Time in seconds for the proposal to expire
        expires_in: u64,
    },
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
    /// Test whether a signature is valid given the public key and attestation
    #[returns(String)]
    TestVerifySignature {
        public_key_base64: String,
        signature_base64: String,
        attestation: String,
    }, // /// Returns the allowed signers for signature verification
       // Signers {},
       // /// Returns all the tokens that have been enabled for bridging
       // Tokens {},
}

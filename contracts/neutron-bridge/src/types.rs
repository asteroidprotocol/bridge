use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};

// Minimum IBC timeout is 5 seconds
pub const MIN_IBC_TIMEOUT_SECONDS: u64 = 5;
// Maximum IBC timeout is 1 hour
pub const MAX_IBC_TIMEOUT_SECONDS: u64 = 60 * 60;

pub const FEE_DENOM: &str = "untrn";
// Signer threshold can't be less than this value
pub const MIN_SIGNER_THRESHOLD: u8 = 2;
// The reply ID for the instantiate_denom reply when linking a token
pub const INSTANTIATE_DENOM_REPLY_ID: u64 = 1;
// The reply ID for IBC transfer to capture the channel and sequence
pub const IBC_REPLY_HANDLER_ID: u64 = 2;

#[cw_serde]
pub struct Config {
    /// The owner's address
    pub owner: Addr,
    /// The chain ID this bridge is connected to
    pub bridge_chain_id: String,
    /// The channel used to communicate with the Hub
    pub bridge_ibc_channel: String,
    /// The timeout in seconds for IBC packets
    pub ibc_timeout_seconds: u64,
}

#[cw_serde]
pub struct TokenMetadata {
    /// The ticker of the CFT-20 token
    pub ticker: String,
    /// The name of the CFT-20 token
    pub name: String,
    /// The URL to the CFT-20 token's image
    pub image_url: String,
    /// The amount of decimals this CFT-20 uses
    pub decimals: u32,
}

#[cw_serde]
pub struct QuerySignersResponse {
    /// The signers currently loaded, the format is
    /// (base64 public key, name)
    pub signers: Vec<(String, String)>,
}

#[cw_serde]
pub struct QueryTokensResponse {
    /// The list of token denoms allowed in bridging
    pub tokens: Vec<String>,
}

#[cw_serde]
pub struct BridgingAsset {
    // pub channel_id: String,
    // pub sequence: u64,
    pub sender: Addr,
    pub funds: Coin,
}

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, IbcTimeout};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Minimum IBC timeout is 5 seconds
pub const MIN_IBC_TIMEOUT_SECONDS: u64 = 5;
// Maximum IBC timeout is 1 hour
pub const MAX_IBC_TIMEOUT_SECONDS: u64 = 60 * 60;

pub const FEE_DENOM: &str = "untrn";

#[cw_serde]
pub struct Config {
    /// The owner's address
    pub owner: Addr,
    /// The threshold of signers needed to confirm a message
    pub signer_threshold: u8,
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

/// The structure to hold verification information
#[cw_serde]
pub struct Verifier {
    pub public_key_base64: String,
    pub signature_base64: String,
}

#[cw_serde]
pub struct QuerySignersResponse {
    pub signers: Vec<(String, String)>,
}

#[cw_serde]
pub struct QueryTokensResponse {
    pub tokens: Vec<String>,
}

/// These are messages in the IBC lifecycle. Only usable by IBC-enabled contracts
/// (contracts that directly speak the IBC protocol via 6 entry points)
#[non_exhaustive]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CustomIbcMsg {
    /// Sends bank tokens owned by the contract to the given address on another chain.
    /// The channel must already be established between the ibctransfer module on this chain
    /// and a matching module on the remote chain.
    /// We cannot select the port_id, this is whatever the local chain has bound the ibctransfer
    /// module to.
    TransferWithMemo {
        /// existing channel to send the tokens over
        channel_id: String,
        /// address on the remote chain to receive these tokens
        to_address: String,
        /// packet data only supports one coin
        /// https://github.com/cosmos/cosmos-sdk/blob/v0.40.0/proto/ibc/applications/transfer/v1/transfer.proto#L11-L20
        amount: Coin,
        /// when packet times out, measured on remote chain
        timeout: IbcTimeout,
        /// optional memo to include
        memo: Option<String>,
    },
}

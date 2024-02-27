use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    /// The owner's address
    pub owner: Addr,
}

#[cw_serde]
pub struct TokenMetadata {
    /// The ticker of the CFT-20 token
    pub ticker: String,
    /// The name of the CFT-20 token
    pub name: String,
    /// The amount of decimals this CFT-20 uses
    pub decimals: u32,
}

/// Store the contract config
pub const CONFIG: Item<Config> = Item::new("config");

// Token Mapping is kept in a map of
// CFT-20 Ticker -> TokenFactory denom as well as the reverse
// TokenFactory denom -> CFT-20 Ticker
pub const TOKEN_MAPPING: Map<&str, String> = Map::new("token_mapping");

/// Store the token metadata when the denom is created via Reply
pub const TOKEN_METADATA: Item<TokenMetadata> = Item::new("token_metadata");

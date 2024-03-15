use cw_storage_plus::{Item, Map};

use crate::types::{Config, TokenMetadata};

use astroport::common::OwnershipProposal;

/// Store the contract config
pub const CONFIG: Item<Config> = Item::new("config");

// TODO: The public key should be stored as u8 to avoid decoding it the whole time, we can always convert to b64 on query/checks
/// The public keys of the allowed signers of bridge messages used to confirm
/// signature
/// It holds <public key, name> to help identify specific keys
pub const SIGNERS: Map<&[u8], String> = Map::new("signers");

// Token Mapping is kept in a map of
// CFT-20 Ticker -> TokenFactory denom as well as the reverse
// TokenFactory denom -> CFT-20 Ticker
pub const TOKEN_MAPPING: Map<&str, String> = Map::new("token_mapping");

/// Store the disabled tokens
pub const DISABLED_TOKENS: Map<&str, bool> = Map::new("disabled_tokens");

/// Store the token metadata when the denom is created via Reply
pub const TOKEN_METADATA: Item<TokenMetadata> = Item::new("token_metadata");

/// Contains a proposal to change contract ownership
pub const OWNERSHIP_PROPOSAL: Item<OwnershipProposal> = Item::new("ownership_proposal");

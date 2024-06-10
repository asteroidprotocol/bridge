use cw_storage_plus::{Item, Map};

use crate::types::{BridgingAsset, Config, TokenMetadata};

use astroport::common::OwnershipProposal;

/// Store the contract config
pub const CONFIG: Item<Config> = Item::new("config");

/// The public keys of the allowed signers of bridge messages used to confirm
/// signature. The public key is stored in the format required during bridging
/// It holds <public key, name> to help identify specific keys
pub const SIGNERS: Map<&[u8], String> = Map::new("signers");

// Token Mapping is kept in a map of
// CFT-20 Ticker -> TokenFactory denom as well as the reverse
// TokenFactory denom -> CFT-20 Ticker
pub const TOKEN_MAPPING: Map<&str, String> = Map::new("token_mapping");

/// Store the disabled tokens
pub const DISABLED_TOKENS: Map<&str, bool> = Map::new("disabled_tokens");

/// Store the transactions we've processed
pub const HANDLED_TRANSACTIONS: Map<&str, bool> = Map::new("handled_transactions");

/// Store the token metadata when the denom is created via Reply
pub const TOKEN_METADATA: Item<TokenMetadata> = Item::new("token_metadata");

/// Contains a proposal to change contract ownership
pub const OWNERSHIP_PROPOSAL: Item<OwnershipProposal> = Item::new("ownership_proposal");

/// Holds the bridging assets that are currently in flight
pub const BRIDGE_INFLIGHT: Map<(&str, u64), BridgingAsset> = Map::new("bridge_inflight");

/// Temporary storage for the payload of the current bridge message for handling replies
pub const BRIDGE_CURRENT_PAYLOAD: Item<BridgingAsset> = Item::new("bridge_current_payload");

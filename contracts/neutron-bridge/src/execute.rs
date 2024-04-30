use astroport::common::{claim_ownership, drop_ownership_proposal, propose_new_owner};
use base64::{engine::general_purpose, Engine as _};
use cosmwasm_std::{coin, entry_point, BankMsg, Coin, Reply, StdError, SubMsg, Uint128};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use ed25519_dalek::{VerifyingKey, PUBLIC_KEY_LENGTH};

use neutron_sdk::bindings::msg::{IbcFee, NeutronMsg};
use neutron_sdk::bindings::query::NeutronQuery;
use neutron_sdk::query::min_ibc_fee::query_min_ibc_fee;
use neutron_sdk::sudo::msg::RequestPacketTimeoutHeight;
use osmosis_std::types::cosmos::bank::v1beta1::{DenomUnit, Metadata};
use osmosis_std::types::osmosis::tokenfactory::v1beta1::{
    MsgBurn, MsgCreateDenom, MsgCreateDenomResponse, MsgMint, MsgSetDenomMetadata,
};

use crate::msg::ExecuteMsg;
use crate::state::{
    DISABLED_TOKENS, HANDLED_TRANSACTIONS, OWNERSHIP_PROPOSAL, SIGNERS, TOKEN_MAPPING,
    TOKEN_METADATA,
};
use crate::types::{
    Config, TokenMetadata, FEE_DENOM, INSTANTIATE_DENOM_REPLY_ID, MAX_IBC_TIMEOUT_SECONDS,
    MIN_IBC_TIMEOUT_SECONDS, MIN_SIGNER_THRESHOLD,
};
use crate::verifier::verify_signatures;
use crate::{error::ContractError, state::CONFIG};

/// Exposes all the execute functions available in the contract
///
/// ## Executable Messages
/// * **ExecuteMsg::LinkToken { source_chain_id, token,signatures } ** Link and enable a CFT-20 token to be bridged
/// * **ExecuteMsg::EnableToken { ticker}** Enable a previously disabled token to being bridged again
/// * **ExecuteMsg::DisableToken { ticker }** Disable a token from being bridged
/// * **ExecuteMsg::Receive { source_chain_id, transaction_hash, ticker, amount, destination_addr, signatures }** Receive CFT-20 token message from the Hub
/// * **ExecuteMsg::Send { destination_addr }** Send CFT-20 token back to the Hub
/// * **ExecuteMsg::RetrySend { failure_id }** Retry a failed IBC transaction, the failure IDs can be retrieved using
/// * **ExecuteMsg::AddSigner { public_key_base64, name }** Adds a signer to the allowed list for signature verification
/// * **ExecuteMsg::RemoveSigner { public_key_base64 }** Remove a signer from the allowed list for signature verification
/// * **ExecuteMsg::UpdateConfig { signer_threshold, bridge_chain_id, bridge_ibc_channel, ibc_timeout_seconds }** Update the contract config
/// * **ExecuteMsg::ProposeNewOwner { owner, expires_in }** Propose a new owner for the contract
/// * **ExecuteMsg::DropOwnershipProposal {}** Remove the ownership transfer proposal
/// * **ExecuteMsg::ClaimOwnership {}** Claim contract ownership
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<NeutronQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<NeutronMsg>, ContractError> {
    match msg {
        ExecuteMsg::LinkToken {
            source_chain_id,
            token,
            signatures,
        } => link_token(deps, env, source_chain_id, token, signatures),
        ExecuteMsg::EnableToken { ticker } => enable_token(deps, env, info, ticker),
        ExecuteMsg::DisableToken { ticker } => disable_token(deps, env, info, ticker),
        ExecuteMsg::Receive {
            source_chain_id,
            transaction_hash,
            ticker,
            amount,
            destination_addr,
            signatures,
        } => bridge_receive(
            deps,
            env,
            source_chain_id,
            transaction_hash,
            ticker,
            amount,
            destination_addr,
            signatures,
        ),
        ExecuteMsg::Send { destination_addr } => bridge_send(deps, env, info, destination_addr),
        ExecuteMsg::RetrySend { failure_id } => retry_send(failure_id),
        ExecuteMsg::AddSigner {
            public_key_base64,
            name,
        } => add_signer(deps, info, name, public_key_base64),
        ExecuteMsg::RemoveSigner { public_key_base64 } => {
            remove_signer(deps, env, info, public_key_base64)
        }
        ExecuteMsg::UpdateConfig {
            signer_threshold,
            bridge_chain_id,
            bridge_ibc_channel,
            ibc_timeout_seconds,
        } => update_config(
            deps,
            info,
            signer_threshold,
            bridge_chain_id,
            bridge_ibc_channel,
            ibc_timeout_seconds,
        ),
        ExecuteMsg::ProposeNewOwner { owner, expires_in } => {
            let config = CONFIG.load(deps.storage)?;
            propose_new_owner(
                deps,
                info,
                env,
                owner,
                expires_in,
                config.owner,
                OWNERSHIP_PROPOSAL,
            )
            .map_err(Into::into)
        }
        ExecuteMsg::DropOwnershipProposal {} => {
            let config: Config = CONFIG.load(deps.storage)?;
            drop_ownership_proposal(deps, info, config.owner, OWNERSHIP_PROPOSAL)
                .map_err(Into::into)
        }
        ExecuteMsg::ClaimOwnership {} => {
            claim_ownership(deps, info, env, OWNERSHIP_PROPOSAL, |deps, new_owner| {
                CONFIG
                    .update::<_, StdError>(deps.storage, |mut v| {
                        v.owner = new_owner;
                        Ok(v)
                    })
                    .map(|_| ())
            })
            .map_err(Into::into)
        }
    }
}

/// The entry point to the contract for processing replies from submessages.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    deps: DepsMut<NeutronQuery>,
    env: Env,
    msg: Reply,
) -> Result<Response<NeutronMsg>, ContractError> {
    match msg.id {
        INSTANTIATE_DENOM_REPLY_ID => {
            let MsgCreateDenomResponse { new_token_denom } = msg.result.try_into()?;

            let metadata = TOKEN_METADATA.load(deps.storage)?;

            // TODO: Add back setting denom metadata after figuring out test
            let denom_metadata_msg = MsgSetDenomMetadata {
                sender: env.contract.address.to_string(),
                metadata: Some(Metadata {
                    symbol: metadata.ticker.clone(),
                    name: metadata.name,
                    base: new_token_denom.clone(),
                    display: metadata.ticker.clone(),
                    denom_units: vec![
                        DenomUnit {
                            denom: new_token_denom.clone(),
                            exponent: 0,
                            aliases: vec![],
                        },
                        DenomUnit {
                            denom: metadata.ticker.clone(),
                            exponent: metadata.decimals,
                            aliases: vec![],
                        },
                    ],
                    description: format!(
                        "{} is an Asteroid CFT-20 token bridged from the Cosmos Hub",
                        metadata.ticker
                    ),
                    uri: metadata.image_url,
                    uri_hash: "".to_string(),
                }),
            };

            // Save the mapping of TICKER <> DENOM both ways to ease lookups
            // in both directions
            TOKEN_MAPPING.save(deps.storage, &metadata.ticker, &new_token_denom)?;
            TOKEN_MAPPING.save(deps.storage, &new_token_denom, &metadata.ticker)?;
            TOKEN_METADATA.remove(deps.storage);

            Ok(Response::new()
                // TODO: Add back setting denom metadata after figuring out test
                // .add_message(denom_metadata_msg)
                .add_attribute("action", "set_denom_metadata")
                .add_attribute("ticker", metadata.ticker))
        }
        _ => Err(ContractError::InvalidReplyId { id: msg.id }),
    }
}

/// Enable the bridging of a CFT-20 token
///
/// If this token doesn't have a corresponding TokenFactory token one will
/// be created using the information provided.
fn link_token(
    deps: DepsMut<NeutronQuery>,
    env: Env,
    source_chain_id: String,
    token: TokenMetadata,
    signatures: Vec<String>,
) -> Result<Response<NeutronMsg>, ContractError> {
    // If we already have this token, return an error
    if TOKEN_MAPPING.has(deps.storage, &token.ticker) {
        return Err(ContractError::TokenAlreadyExists {
            ticker: token.ticker,
        });
    }

    // Build the attestation message to verify the token information
    // The format is {source_chain_id}{ticker}{decimals}{chain_id}{contract_address}
    // cosmoshub-4ticker8neutron-1neutron1xxxxx
    let attestation = format!(
        "{}{}{}{}{}",
        source_chain_id, token.ticker, token.decimals, env.block.chain_id, env.contract.address
    );

    // Verify with current keys
    verify_signatures(deps.as_ref(), attestation.as_bytes(), &signatures)?;

    // If not, create the denom and set the metadata
    let create_denom_msg = SubMsg::reply_on_success(
        MsgCreateDenom {
            sender: env.contract.address.to_string(),
            subdenom: token.ticker.clone(),
        },
        INSTANTIATE_DENOM_REPLY_ID,
    );

    TOKEN_METADATA.save(deps.storage, &token)?;

    Ok(Response::new().add_submessage(create_denom_msg))
}

/// Enable a token for bridging if it was previously disabled
fn enable_token(
    deps: DepsMut<NeutronQuery>,
    _env: Env,
    info: MessageInfo,
    ticker: String,
) -> Result<Response<NeutronMsg>, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only owner can update the config
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    // If this token isn't in the disabled list, return an error
    if !DISABLED_TOKENS.has(deps.storage, &ticker) {
        return Err(ContractError::InvalidConfiguration {
            reason: "This token is not disabled".to_string(),
        });
    }

    DISABLED_TOKENS.remove(deps.storage, &ticker);

    Ok(Response::new()
        .add_attribute("action", "enable_token")
        .add_attribute("ticker", ticker))
}

/// Disable a token for bridging
fn disable_token(
    deps: DepsMut<NeutronQuery>,
    _env: Env,
    info: MessageInfo,
    ticker: String,
) -> Result<Response<NeutronMsg>, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only owner can update the config
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    // If this token doesn't exist, return an error
    if !TOKEN_MAPPING.has(deps.storage, &ticker) {
        return Err(ContractError::TokenDoesNotExist { ticker });
    }

    DISABLED_TOKENS.save(deps.storage, &ticker, &true)?;

    Ok(Response::new()
        .add_attribute("action", "disable_token")
        .add_attribute("ticker", ticker))
}

/// Receive tokens from the Hub and mint them to the destination address
fn bridge_receive(
    deps: DepsMut<NeutronQuery>,
    env: Env,
    source_chain_id: String,
    transaction_hash: String,
    ticker: String,
    amount: Uint128,
    destination_addr: String,
    signatures: Vec<String>,
) -> Result<Response<NeutronMsg>, ContractError> {
    // Check if the token is disabled
    if DISABLED_TOKENS.has(deps.storage, &ticker) {
        return Err(ContractError::TokenDisabled { ticker });
    }
    // Check the amount sent, if 0, reject
    if amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }
    // Check destination address, if invalid, reject
    if deps.api.addr_validate(&destination_addr).is_err() {
        return Err(ContractError::InvalidDestinationAddr {});
    }
    // Check the ticker, if it doesn't exist activate needs to be called first
    if !TOKEN_MAPPING.has(deps.storage, &ticker) {
        return Err(ContractError::TokenDoesNotExist { ticker });
    }
    // Check if we've processed this transaction already
    if HANDLED_TRANSACTIONS.has(deps.storage, &transaction_hash) {
        return Err(ContractError::TransactionAlreadyHandled { transaction_hash });
    }
    // Store the transaction hash to prevent replay attacks
    HANDLED_TRANSACTIONS.save(deps.storage, &transaction_hash, &true)?;

    // Build the attestation message
    let attestation = format!(
        // source_chain_id, transaction_hash, ticker, amount
        "{}{}{}{}{}{}{}",
        source_chain_id,
        transaction_hash,
        ticker,
        amount,
        env.block.chain_id,
        env.contract.address,
        destination_addr
    );

    verify_signatures(deps.as_ref(), attestation.as_bytes(), &signatures)?;

    let tokenfactory_denom = TOKEN_MAPPING.load(deps.storage, &ticker)?;

    // If ticker already exists, mint new tokens to the destination
    let coins_to_mint = coin(amount.u128(), tokenfactory_denom);

    // TokenFactory can only mint to the admin for now
    let mint_msg = MsgMint {
        sender: env.contract.address.to_string(),
        amount: Some(coins_to_mint.clone().into()),
        mint_to_address: env.contract.address.to_string(),
    };

    // Once minted to self, transfer to destination
    let mint_transfer = BankMsg::Send {
        to_address: destination_addr.clone(),
        amount: vec![coins_to_mint.clone()],
    };

    Ok(Response::default()
        .add_message(mint_msg)
        .add_message(mint_transfer)
        .add_attribute("action", "bridge_receive")
        .add_attribute("tokens", coins_to_mint.to_string())
        .add_attribute("destination", destination_addr))
}

/// Return tokens to the Hub
fn bridge_send(
    deps: DepsMut<NeutronQuery>,
    env: Env,
    info: MessageInfo,
    destination_addr: String,
) -> Result<Response<NeutronMsg>, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // The user should be sending 2 tokens, one TokenFactory token to bridge back
    // and NTRN for paying the IBC fees
    let mut fee_coin = Coin::default();
    let mut bridging_coin = Coin::default();

    // Only the bridged token and NTRN must be sent
    if info.funds.len() != 2 {
        return Err(ContractError::InvalidFunds {});
    }

    info.funds.iter().for_each(|coin| {
        if coin.denom == FEE_DENOM {
            fee_coin = coin.clone();
        }
        if TOKEN_MAPPING.has(deps.storage, &coin.denom) {
            bridging_coin = coin.clone();
        }
    });

    // If either of the coins is 0, reject
    if fee_coin.amount.is_zero() || bridging_coin.amount.is_zero() {
        return Err(ContractError::InvalidFunds {});
    }

    // Check the mapping for this token, fail if no mapping exists
    let cft20_denom = TOKEN_MAPPING.load(deps.storage, &bridging_coin.denom)?;

    // Check if the token is disabled
    // We check the CFT-20 ticker and the TokenFactory in case one is missed in the disable
    if DISABLED_TOKENS.has(deps.storage, &cft20_denom)
        || DISABLED_TOKENS.has(deps.storage, &bridging_coin.denom)
    {
        return Err(ContractError::TokenDisabled {
            ticker: cft20_denom,
        });
    }

    // Contruct the IBC memo message to return X of denom on the Hub
    // urn:bridge:gaialocal-1@v1;recv$tic=LOCALROIDS,amt=1,dst=cosmos1234,rch=neutronlocal-1,src=neutron1m857lgtjssgt0wm3crzfmt3v950vqnkqy4vep9
    let memo = format!(
        "urn:bridge:{}@v1;recv$tic={},amt={},dst={},rch={},src={}",
        config.bridge_chain_id,
        cft20_denom,
        bridging_coin.amount,
        destination_addr,
        env.block.chain_id,
        info.sender
    );

    // Burn the bridging token
    let burn_msg = MsgBurn {
        sender: env.contract.address.to_string(),
        burn_from_address: env.contract.address.to_string(),
        amount: Some(bridging_coin.clone().into()),
    };

    let fee = min_ntrn_ibc_fee(
        query_min_ibc_fee(deps.as_ref())
            .map_err(|err| StdError::generic_err(err.to_string()))?
            .min_fee,
    );

    // Calculate the total fee required
    let total_fee = fee
        .ack_fee
        .iter()
        .chain(fee.recv_fee.iter())
        .chain(fee.timeout_fee.iter())
        .filter(|a| a.denom == FEE_DENOM)
        .fold(Uint128::zero(), |acc, coin| acc + coin.amount);

    // Ensure the user sent enough to cover the fee + 1 untrn to do the actual IBC transaction
    let ibc_coin = coin(1u128, "untrn");
    if total_fee > fee_coin.amount.saturating_sub(Uint128::one()) {
        return Err(ContractError::InsufficientFunds {
            expected: total_fee.saturating_add(Uint128::one()),
        });
    }

    // Construct the IBC transfer message
    // The memo is important and enables the indexer to release the tokens on
    // the Hub's side
    let ibc_transfer = NeutronMsg::IbcTransfer {
        source_port: "transfer".to_string(),
        source_channel: config.bridge_ibc_channel,
        // TODO: Note to auditor, please also confirm that this sender address can't be spoofed on the Hub's side
        sender: env.contract.address.to_string(),
        receiver: destination_addr.clone(),
        token: ibc_coin,
        timeout_height: RequestPacketTimeoutHeight {
            revision_number: None,
            revision_height: None,
        },
        // Neutron expects nanoseconds
        // https://github.com/neutron-org/neutron/blob/303d764b57d871749fcf7d59a67b5d3078779258/proto/transfer/v1/tx.proto#L39-L42
        timeout_timestamp: env
            .block
            .time
            .plus_seconds(config.ibc_timeout_seconds)
            .nanos(),
        memo,
        fee,
    };

    let response = Response::new()
        .add_message(burn_msg)
        .add_message(ibc_transfer)
        .add_attribute("action", "bridge_send")
        .add_attribute("tokens", bridging_coin.to_string())
        .add_attribute("destination", destination_addr);

    Ok(response)
}

/// Retry a failed IBC transaction by using the chain's failure ID
fn retry_send(failure_id: u64) -> Result<Response<NeutronMsg>, ContractError> {
    let msg = NeutronMsg::submit_resubmit_failure(failure_id);
    Ok(Response::default()
        .add_message(msg)
        .add_attribute("action", "bridge_retry_send")
        .add_attribute("failure_id", failure_id.to_string()))
}

/// Add a signer to the list of allowed public keys
/// Verifies that the public key can be loaded and in the correct format
/// as well as checks for duplicate keys
fn add_signer(
    deps: DepsMut<NeutronQuery>,
    info: MessageInfo,
    name: String,
    public_key_base64: String,
) -> Result<Response<NeutronMsg>, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only owner can update the config
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    // Decode the base64 encoded public key
    let public_key = match general_purpose::STANDARD.decode(public_key_base64.as_bytes()) {
        Ok(bytes) => bytes,
        Err(_) => {
            return Err(ContractError::InvalidConfiguration {
                reason: "Key could not be decoded".to_string(),
            })
        }
    };

    // Verify that the format for the key is correct before adding it
    let public_key_bytes: [u8; PUBLIC_KEY_LENGTH] = match public_key.clone().try_into() {
        Ok(bytes) => bytes,
        Err(_) => {
            return Err(ContractError::InvalidConfiguration {
                reason: "Invalid public key length".to_string(),
            });
        }
    };
    VerifyingKey::from_bytes(&public_key_bytes)?;

    // Ensure this key isn't loaded yet
    if SIGNERS.has(deps.storage, &public_key) {
        return Err(ContractError::InvalidConfiguration {
            reason: "The public key has already been loaded".to_string(),
        });
    }

    SIGNERS.save(deps.storage, &public_key, &name)?;

    Ok(Response::default()
        .add_attribute("action", "add_signer")
        .add_attribute("name", name)
        .add_attribute("public_key", public_key_base64))
}

/// Remove a signer from the list of allowed public keys
fn remove_signer(
    deps: DepsMut<NeutronQuery>,
    _env: Env,
    info: MessageInfo,
    public_key_base64: String,
) -> Result<Response<NeutronMsg>, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only owner can update the config
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    // Decode the base64 encoded public key
    let public_key = match general_purpose::STANDARD.decode(public_key_base64.as_bytes()) {
        Ok(bytes) => bytes,
        Err(_) => {
            return Err(ContractError::InvalidConfiguration {
                reason: "Key could not be decoded".to_string(),
            })
        }
    };

    if !SIGNERS.has(deps.storage, &public_key) {
        return Err(ContractError::InvalidConfiguration {
            reason: "Key to remove doesn't exist".to_string(),
        });
    }

    SIGNERS.remove(deps.storage, &public_key);

    Ok(Response::default()
        .add_attribute("action", "remove_signer")
        .add_attribute("public_key", public_key_base64))
}

/// Update the Bridge config
fn update_config(
    deps: DepsMut<NeutronQuery>,
    info: MessageInfo,
    signer_threshold: Option<u8>,
    bridge_chain_id: Option<String>,
    bridge_ibc_channel: Option<String>,
    ibc_timeout_seconds: Option<u64>,
) -> Result<Response<NeutronMsg>, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    // Only owner can update the config
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    // Signer threshold can't be less than 2. We require at _least_ 2 valid
    // signatures before allowing the bridge to even be instantiated
    if let Some(signer_threshold) = signer_threshold {
        if signer_threshold < MIN_SIGNER_THRESHOLD {
            return Err(ContractError::InvalidConfiguration {
                reason: format!(
                    "Invalid signer threshold, the minimum is {}",
                    MIN_SIGNER_THRESHOLD
                ),
            });
        }

        config.signer_threshold = signer_threshold;
    }

    // Allow changing the source chain ID in case the source chain
    // undergoes an upgrade that changes the chain ID
    if let Some(bridge_chain_id) = bridge_chain_id {
        if bridge_chain_id.is_empty() {
            return Err(ContractError::InvalidConfiguration {
                reason: "The source chain ID must be specified".to_string(),
            });
        }
        config.bridge_chain_id = bridge_chain_id;
    }

    // Allow changing the IBC channel in case the original channel expires
    // and can't be revived
    if let Some(bridge_ibc_channel) = bridge_ibc_channel {
        if bridge_ibc_channel.is_empty() {
            return Err(ContractError::InvalidConfiguration {
                reason: "The bridge IBC channel must be specified".to_string(),
            });
        }
        config.bridge_ibc_channel = bridge_ibc_channel;
    }

    // Validate minimum and maximum IBC timeout
    if let Some(ibc_timeout_seconds) = ibc_timeout_seconds {
        if !(MIN_IBC_TIMEOUT_SECONDS..=MAX_IBC_TIMEOUT_SECONDS).contains(&ibc_timeout_seconds) {
            return Err(ContractError::InvalidIBCTimeout {
                timeout: ibc_timeout_seconds,
                min: MIN_IBC_TIMEOUT_SECONDS,
                max: MAX_IBC_TIMEOUT_SECONDS,
            });
        }
        config.ibc_timeout_seconds = ibc_timeout_seconds;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attribute("action", "update_config"))
}

/// Helper function to query the Neutron chain for the current minimum IBC fees
fn min_ntrn_ibc_fee(fee: IbcFee) -> IbcFee {
    IbcFee {
        recv_fee: fee.recv_fee,
        ack_fee: fee
            .ack_fee
            .into_iter()
            .filter(|a| a.denom == FEE_DENOM)
            .collect(),
        timeout_fee: fee
            .timeout_fee
            .into_iter()
            .filter(|a| a.denom == FEE_DENOM)
            .collect(),
    }
}
use astroport::common::{claim_ownership, drop_ownership_proposal, propose_new_owner};
use base64::{engine::general_purpose, Engine as _};
use cosmwasm_std::{
    coin, entry_point, BankMsg, CosmosMsg, IbcMsg, IbcTimeout, Reply, StdError, SubMsg, Uint128,
};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw_utils::one_coin;
use ed25519_dalek::{VerifyingKey, PUBLIC_KEY_LENGTH};
use neutron_sdk::bindings::query::NeutronQuery;
use neutron_sdk::query::min_ibc_fee::query_min_ibc_fee;
use neutron_sdk::sudo::msg::RequestPacketTimeoutHeight;
use osmosis_std::types::cosmos::bank::v1beta1::{DenomUnit, Metadata};
use osmosis_std::types::cosmos::crypto::ed25519;

use crate::msg::ExecuteMsg;
use crate::state::{DISABLED_TOKENS, OWNERSHIP_PROPOSAL, SIGNERS, TOKEN_MAPPING, TOKEN_METADATA};
use crate::types::{
    Config, CustomIbcMsg, TokenMetadata, Verifier, MAX_IBC_TIMEOUT_SECONDS, MIN_IBC_TIMEOUT_SECONDS,
};
use crate::verifier::verify_signatures;
use crate::{error::ContractError, state::CONFIG};

use neutron_sdk::bindings::msg::{IbcFee, NeutronMsg};

use osmosis_std::types::osmosis::tokenfactory::v1beta1::{
    MsgBurn, MsgCreateDenom, MsgCreateDenomResponse, MsgMint, MsgSetBeforeSendHook,
    MsgSetDenomMetadata,
};

/// This contract accepts only one fee denom
const FEE_DENOM: &str = "untrn";

/// A `reply` call code ID used for sub-messages.
enum ReplyIds {
    InstantiateNewDenom = 1,
}

impl TryFrom<u64> for ReplyIds {
    type Error = ContractError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(ReplyIds::InstantiateNewDenom),
            // 2 => Ok(ReplyIds::InstantiateTrackingContract),
            _ => Err(ContractError::FailedToParseReply {}),
        }
    }
}

/// Exposes all the execute functions available in the contract.
///
/// ## Execute messages
///
/// * **ExecuteMsg::Receive(msg)** Receives a message of type [`Cw20ReceiveMsg`] and processes
/// it depending on the received template.
///
/// * **ExecuteMsg::UpdateConfig { hub_addr }** Update parameters in the Outpost contract. Only the owner is allowed to
/// update the config

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<NeutronQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<NeutronMsg>, ContractError> {
    match msg {
        ExecuteMsg::LinkToken { token, verifiers } => link_token(deps, env, info, token, verifiers),
        ExecuteMsg::EnableToken { ticker } => enable_token(deps, env, info, ticker),
        ExecuteMsg::DisableToken { ticker } => disable_token(deps, env, info, ticker),
        ExecuteMsg::Receive {
            source_chain_id,
            transaction_hash,
            ticker,
            amount,
            destination_addr,
            verifiers,
        } => bridge_receive(
            deps,
            env,
            info,
            source_chain_id,
            transaction_hash,
            ticker,
            amount,
            destination_addr,
            verifiers,
        ),
        // ExecuteMsg::Send { destination_addr } => bridge_send(deps, env, info, destination_addr),
        ExecuteMsg::AddSigner {
            public_key_base64,
            name,
        } => add_signer(deps, env, info, public_key_base64, name),
        ExecuteMsg::RemoveSigner { public_key_base64 } => {
            remove_signer(deps, env, info, public_key_base64)
        }
        ExecuteMsg::UpdateConfig {
            signer_threshold,
            bridge_ibc_channel,
            ibc_timeout_seconds,
        } => update_config(
            deps,
            env,
            info,
            signer_threshold,
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

/// Enable the bridging of a CFT-20 token
///
/// If this token doesn't have a corresponding TokenFactory token one will
/// be created using the information provided.
///
/// If the token already has a
/// TokenFactory token and is currently enabled, no action is taken.
///
/// Lastly, if the token has a TokenFactory token but is currently disabled,
/// it will be enabled again without any further changes
fn link_token(
    deps: DepsMut<NeutronQuery>,
    env: Env,
    _info: MessageInfo,
    token: TokenMetadata,
    verifiers: Vec<Verifier>,
) -> Result<Response<NeutronMsg>, ContractError> {
    // If we already have this token, return an error
    if TOKEN_MAPPING.has(deps.storage, &token.ticker) {
        return Err(ContractError::TokenAlreadyExists {
            ticker: token.ticker,
        });
    }

    // // TODO: Linking a token hasn't been implemented yet
    // // TODO: Build the message to verify
    // let message = b"";

    // // Verify the message with the current loaded keys
    // verify_signatures(deps.as_ref(), message, &verifiers)?;
    // // At this point everything has been verified and we may continue

    // If not, create the denom and set the metadata
    let create_denom_msg = SubMsg::reply_on_success(
        MsgCreateDenom {
            sender: env.contract.address.to_string(),
            subdenom: token.ticker.clone(),
        },
        ReplyIds::InstantiateNewDenom as u64,
    );

    TOKEN_METADATA.save(deps.storage, &token)?;

    Ok(Response::new().add_submessage(create_denom_msg))
}

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
        return Err(ContractError::Std(StdError::generic_err(
            "This token is not disabled",
        )));
    }

    // TODO Decide if we should remove the token completely, or rather have a block list
    DISABLED_TOKENS.remove(deps.storage, &ticker);

    Ok(Response::new()
        .add_attribute("action", "enable_token")
        .add_attribute("ticker", ticker))
}

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

    // TODO Decide if we should remove the token completely, or rather have a block list
    DISABLED_TOKENS.save(deps.storage, &ticker, &true)?;

    Ok(Response::new()
        .add_attribute("action", "disable_token")
        .add_attribute("ticker", ticker))
}

/// The entry point to the contract for processing replies from submessages.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match ReplyIds::try_from(msg.id)? {
        ReplyIds::InstantiateNewDenom => {
            let MsgCreateDenomResponse { new_token_denom } = msg.result.try_into()?;

            let metadata = TOKEN_METADATA.load(deps.storage)?;

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

            Ok(Response::new().add_message(denom_metadata_msg))
        }
    }
}

// TODO: Clear up args
fn bridge_receive(
    deps: DepsMut<NeutronQuery>,
    env: Env,
    info: MessageInfo,
    source_chain_id: String,
    transaction_hash: String,
    ticker: String,
    amount: Uint128,
    destination_addr: String,
    verifiers: Vec<Verifier>,
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

    verify_signatures(deps.as_ref(), attestation.as_bytes(), &verifiers)?;

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
        to_address: destination_addr,
        amount: vec![coins_to_mint],
    };

    Ok(Response::default()
        .add_message(mint_msg)
        .add_message(mint_transfer)
        .add_attribute("bridge_receive", ticker))
}

fn bridge_send(
    deps: DepsMut<NeutronQuery>,
    env: Env,
    info: MessageInfo,
    destination_addr: String,
) -> Result<Response<NeutronMsg>, ContractError> {
    // Check the tokens sent, map back to CFT-20 tokens
    // Send IBC message with token destination

    // Only a single coin is allowed to be sent
    let bridging_coin = one_coin(&info)?;

    // Check the mapping for this token, fail if no mapping exists
    let cft20_denom = TOKEN_MAPPING.load(deps.storage, &bridging_coin.denom)?;

    // Check if the token is disabled
    if DISABLED_TOKENS.has(deps.storage, &cft20_denom) {
        return Err(ContractError::TokenDisabled {
            ticker: cft20_denom,
        });
    }

    // Contruct the IBC memo message to return X of denom on the Hub

    // Also burn the tokens
    let burn_msg = MsgBurn {
        sender: env.contract.address.to_string(),
        burn_from_address: env.contract.address.to_string(),
        amount: Some(bridging_coin.clone().into()),
    };

    // Set timeout, 10 minutes
    let ibc_timeout_timestamp = env.block.time.plus_seconds(600);

    let fee = min_ntrn_ibc_fee(
        query_min_ibc_fee(deps.as_ref())
            .map_err(|err| StdError::generic_err(err.to_string()))?
            .min_fee,
    );

    // CosmosMsg::Ibc(IbcMsg::Transfer { channel_id: (), to_address: (), amount: (), timeout: () })

    // osmosis_std::types::ibc::applications::transfer::v1::MsgTransfer {

    // }

    // let ibc_transfer = IbcMsg::Transfer {
    //     channel_id: "channel-0".to_string(),
    //     to_address: "cosmos1_bridge".to_string(),
    //     amount: coin(1u128, "untrn"),
    //     timeout: IbcTimeout::with_timestamp(ibc_timeout_timestamp),
    //     // memo: Some("burn".to_string()),
    // };

    let ibc_transfer = NeutronMsg::IbcTransfer {
        source_port: "transfer".to_string(),
        source_channel: "channel-0".to_string(),
        sender: env.contract.address.to_string(),
        receiver: destination_addr,
        token: coin(1u128, "untrn"),
        timeout_height: RequestPacketTimeoutHeight {
            revision_number: None,
            revision_height: None,
        },
        // Neutron expects nanoseconds
        // https://github.com/neutron-org/neutron/blob/303d764b57d871749fcf7d59a67b5d3078779258/proto/transfer/v1/tx.proto#L39-L42
        timeout_timestamp: ibc_timeout_timestamp.nanos(),
        memo: format!("mint {} token {}", bridging_coin.amount, cft20_denom),
        fee: fee.clone(),
    };

    let response = Response::new()
        .add_message(burn_msg)
        .add_message(ibc_transfer)
        .add_attribute("bridge_send", "something?")
        .add_attribute("fee", format!("{:?}", fee));

    Ok(response)
}

/// Add a signer for verification
fn add_signer(
    deps: DepsMut<NeutronQuery>,
    _env: Env,
    info: MessageInfo,
    public_key_base64: String,
    name: String,
) -> Result<Response<NeutronMsg>, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only owner can update the config
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    // Decode the base64 encoded public key
    let public_key = match general_purpose::STANDARD.decode(public_key_base64.as_bytes()) {
        Ok(bytes) => bytes,
        Err(e) => panic!("Failed to decode public key base64: {}", e),
    };

    // TODO Handle this correctly
    // Try loading the public key to see if it is in valid format
    let public_key_bytes: [u8; PUBLIC_KEY_LENGTH] = public_key.clone().try_into().unwrap();
    VerifyingKey::from_bytes(&public_key_bytes).unwrap();

    if SIGNERS.has(deps.storage, &public_key) {
        return Err(ContractError::KeyAlreadyLoaded {});
    }

    SIGNERS.save(deps.storage, &public_key, &name)?;

    Ok(Response::default())
}

/// Remove a signer from verification
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
        Err(e) => panic!("Failed to decode public key base64: {}", e),
    };

    if SIGNERS.has(deps.storage, &public_key) {
        return Err(ContractError::KeyNotLoaded {});
    }

    SIGNERS.remove(deps.storage, &public_key);

    Ok(Response::default())
}

/// Update the Outpost config
fn update_config(
    deps: DepsMut<NeutronQuery>,
    _env: Env,
    info: MessageInfo,
    signer_threshold: Option<u8>,
    bridge_ibc_channel: Option<String>,
    ibc_timeout_seconds: Option<u64>,
) -> Result<Response<NeutronMsg>, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    // Only owner can update the config
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

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

    // TODO Verify valid IBC channel

    if let Some(bridge_ibc_channel) = bridge_ibc_channel {
        config.bridge_ibc_channel = bridge_ibc_channel;
    }

    // TODO Verify threshold

    if let Some(signer_threshold) = signer_threshold {
        config.signer_threshold = signer_threshold;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

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

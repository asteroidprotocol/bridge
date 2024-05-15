use base64::{engine::general_purpose, Engine as _};
use cosmwasm_std::{
    BankMsg, ChannelResponse, Coin, CosmosMsg, Deps, IbcQuery, Order, QuerierWrapper,
};
use neutron_sdk::bindings::{msg::NeutronMsg, query::NeutronQuery};
use osmosis_std::types::osmosis::tokenfactory::v1beta1::MsgMint;

use crate::{error::ContractError, state::SIGNERS, types::MIN_SIGNER_THRESHOLD};

/// Verify the signatures against the current loaded public keys
/// Once we reach the valid threshold, we return Ok
/// If we don't have enough valid signatures, we return Err
pub fn verify_signatures(
    deps: Deps<NeutronQuery>,
    message: &[u8],
    signatures: &[String],
) -> Result<(), ContractError> {
    // If no signatures were sent, fail the verification
    if signatures.is_empty() {
        return Err(ContractError::ThresholdNotMet {});
    }

    // If duplicate signatures are sent, fail the verification
    let mut unique_signatures = signatures.to_vec();
    unique_signatures.sort();
    unique_signatures.dedup();
    if unique_signatures.len() != signatures.len() {
        return Err(ContractError::DuplicateSignatures {});
    }

    // Calculate the threshold based on the number of signers
    let keys = SIGNERS.keys(deps.storage, None, None, Order::Ascending);
    let majority_threshold = get_majority_threshold(keys.count());

    // If the number of unique signatures are less than the threshold, fail the verification
    if unique_signatures.len() < majority_threshold.into() {
        return Err(ContractError::ThresholdNotMet {});
    }

    // Load the current allowed public keys
    let allowed_keys = SIGNERS.keys(deps.storage, None, None, Order::Ascending);

    let mut verified_signatures = 0;

    // Decode signatures from base64
    let decoded_signatures: Result<Vec<_>, _> = unique_signatures
        .iter()
        .map(|sig| general_purpose::STANDARD.decode(sig))
        .collect();
    let decoded_signatures = decoded_signatures?;

    // Verify the signatures against the loaded keys
    // This requires iterating over the signatures and the loaded keys
    // and thus we should not keep too many keys loaded
    //
    // While there is a possbility for this to be unbounded, the number of keys
    // will be small. The decision to use this method vs
    // sending keys with their signatures was made to simplify interactions
    // with the contract
    for loaded_key in allowed_keys {
        let allowed_key = loaded_key?;

        for signature in &decoded_signatures {
            let is_valid = deps.api.ed25519_verify(message, signature, &allowed_key)?;
            if is_valid {
                verified_signatures += 1;
                if verified_signatures >= majority_threshold {
                    return Ok(());
                }
                // We can move on to the next key and signatures
                break;
            }
        }
    }
    // If we reach this point, we did not have enough valid signatures
    Err(ContractError::ThresholdNotMet {})
}

/// Construct messages to mint and transfer TokenFactory tokens
/// TokenFactory tokens must always be minted by the contract address
pub fn build_mint_messages(
    contract_address: String,
    coin: Coin,
    destination: String,
) -> Vec<CosmosMsg<NeutronMsg>> {
    // TokenFactory can only mint to the sender
    let mint_msg = MsgMint {
        sender: contract_address.clone(),
        amount: Some(coin.clone().into()),
        mint_to_address: contract_address,
    };

    // Once minted to self, transfer to destination
    let mint_transfer = BankMsg::Send {
        to_address: destination,
        amount: vec![coin.clone()],
    };

    vec![mint_msg.into(), mint_transfer.into()]
}

/// Get the majority threshold for the current amount of signers
/// If the amount if an even number, we return the threshold as half of the signers + 1
/// If the amount is an odd number, we return the threshold as half of the signers rounded up
/// to the nearest integer
/// If the threshold is less than the minimum threshold, we return the minimum threshold
pub fn get_majority_threshold(signers_count: usize) -> u8 {
    let threshold = if signers_count % 2 == 0 {
        (signers_count / 2) + 1
    } else {
        (signers_count + 1) / 2
    };

    // Ensure the threshold is not less than MIN_SIGNER_THRESHOLD
    threshold
        .try_into()
        .unwrap_or(MIN_SIGNER_THRESHOLD)
        .max(MIN_SIGNER_THRESHOLD)
}

/// Checks that the given channel and port is valid
pub fn validate_channel(
    querier: QuerierWrapper<NeutronQuery>,
    given_channel: &String,
) -> Result<(), ContractError> {
    let ChannelResponse { channel } = querier.query(
        &IbcQuery::Channel {
            channel_id: given_channel.to_string(),
            port_id: Some("transfer".to_string()),
        }
        .into(),
    )?;
    channel
        .map(|_| ())
        .ok_or_else(|| ContractError::InvalidConfiguration {
            reason: "The provided IBC channel is invalid".to_string(),
        })
}

#[cfg(test)]
mod testing {
    use super::*;

    #[test]
    fn test_threshold_calculation() {
        // Test the threshold calculation
        assert_eq!(get_majority_threshold(0), MIN_SIGNER_THRESHOLD);
        assert_eq!(get_majority_threshold(1), MIN_SIGNER_THRESHOLD);
        assert_eq!(get_majority_threshold(2), MIN_SIGNER_THRESHOLD);
        assert_eq!(get_majority_threshold(3), 2);
        assert_eq!(get_majority_threshold(4), 3);
        assert_eq!(get_majority_threshold(5), 3);
        assert_eq!(get_majority_threshold(6), 4);
        assert_eq!(get_majority_threshold(7), 4);
        assert_eq!(get_majority_threshold(8), 5);
        assert_eq!(get_majority_threshold(9), 5);
        assert_eq!(get_majority_threshold(10), 6);

        assert_eq!(get_majority_threshold(50), 26);
        assert_eq!(get_majority_threshold(51), 26);

        assert_eq!(get_majority_threshold(99), 50);
        assert_eq!(get_majority_threshold(100), 51);
    }
}

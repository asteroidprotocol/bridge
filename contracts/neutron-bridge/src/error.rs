use cosmwasm_std::{OverflowError, StdError, Uint128, VerificationError};
use ed25519_dalek::SignatureError;
use thiserror::Error;

/// This enum describes bribes contract errors
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Contract can't be migrated!")]
    MigrationError {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("The CFT-20 token '{ticker}' is already linked")]
    TokenAlreadyExists { ticker: String },

    #[error("The CFT-20 token '{ticker}' has not been linked for bridging")]
    TokenDoesNotExist { ticker: String },

    #[error("Insufficient valid signatures to confirm the message")]
    ThresholdNotMet {},

    #[error("Duplicated signatures are not allowed")]
    DuplicateSignatures {},

    #[error("This token has been disabled from bridging: {ticker}")]
    TokenDisabled { ticker: String },

    #[error("The transaction has already been handled: {transaction_hash}")]
    TransactionAlreadyHandled { transaction_hash: String },

    #[error("You can not send 0 CFT-20 tokens")]
    ZeroAmount {},

    #[error("Invalid destination address")]
    InvalidDestinationAddr {},

    #[error("Invalid IBC timeout: {timeout}, must be between {min} and {max} seconds")]
    InvalidIBCTimeout { timeout: u64, min: u64, max: u64 },

    #[error("Invalid contract configuration: {reason}")]
    InvalidConfiguration { reason: String },

    #[error("Invalid reply ID: {id}")]
    InvalidReplyId { id: u64 },

    #[error("Invalid funds, expected NTRN and bridging token to be sent together and cover bridging cost")]
    InvalidFunds {},

    #[error("Insufficient funds to cover the bridging cost, expected at least {expected} untrn")]
    InsufficientFunds { expected: Uint128 },

    #[error("Failed to handle IBC transfer response: {detail}")]
    IBCResponseFail { detail: String },
}

impl From<OverflowError> for ContractError {
    fn from(o: OverflowError) -> Self {
        StdError::from(o).into()
    }
}

impl From<VerificationError> for ContractError {
    fn from(v: VerificationError) -> Self {
        StdError::from(v).into()
    }
}

impl From<SignatureError> for ContractError {
    fn from(v: SignatureError) -> Self {
        let std_error = StdError::generic_err(format!("Signature decode error: {}", v));

        // Utilize the existing conversion from StdError to ContractError
        ContractError::from(std_error)
    }
}

impl From<base64::DecodeError> for ContractError {
    fn from(error: base64::DecodeError) -> Self {
        // Convert the base64::DecodeError to a generic StdError
        let std_error = StdError::generic_err(format!("Base64 decode error: {}", error));

        // Utilize the existing conversion from StdError to ContractError
        ContractError::from(std_error)
    }
}

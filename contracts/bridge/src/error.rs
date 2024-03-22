use base64::DecodeError;
use cosmwasm_std::{OverflowError, StdError, VerificationError};
use cw_utils::PaymentError;
use thiserror::Error;

/// This enum describes bribes contract errors
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("Contract can't be migrated!")]
    MigrationError {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("The CFT-20 token '{ticker}' is already activated")]
    TokenAlreadyExists { ticker: String },

    #[error("The CFT-20 token '{ticker}' has not been activated for bridging")]
    TokenDoesNotExist { ticker: String },

    #[error("The public key has already been added")]
    KeyAlreadyLoaded {},

    #[error("The public key is not loaded")]
    KeyNotLoaded {},

    #[error("The public key provided is not a verifier: {public_key_base64}")]
    VerifierNotLoaded { public_key_base64: String },

    #[error("Insufficient valid signatures to confirm the message")]
    ThresholdNotMet {},

    #[error("No signers have been loaded or provided by the caller")]
    NoSigners {},

    #[error("This token has been disabled from bridging: {ticker}")]
    TokenDisabled { ticker: String },

    #[error("You can not send 0 CFT-20 tokens")]
    ZeroAmount {},

    #[error("Invalid destination address")]
    InvalidDestinationAddr {},

    #[error("Failed to parse or process reply message")]
    FailedToParseReply {},

    #[error("Invalid source port {invalid}. Should be : {valid}")]
    InvalidSourcePort { invalid: String, valid: String },

    #[error("Invalid IBC timeout: {timeout}, must be between {min} and {max} seconds")]
    InvalidIBCTimeout { timeout: u64, min: u64, max: u64 },
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

impl From<base64::DecodeError> for ContractError {
    fn from(error: base64::DecodeError) -> Self {
        // Convert the base64::DecodeError to a generic StdError
        let std_error = StdError::generic_err(format!("Base64 decode error: {}", error));

        // Utilize the existing conversion from StdError to ContractError
        ContractError::from(std_error)
    }
}

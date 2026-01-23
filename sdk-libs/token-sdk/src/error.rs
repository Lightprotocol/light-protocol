//! Error types for light-token SDK.
//!
//! This module re-exports errors from light-compressed-token-sdk and defines
//! additional errors specific to high-level token operations.

// Re-export base errors from compressed-token-sdk
pub use light_compressed_token_sdk::error::{Result, TokenSdkError};
use solana_program_error::ProgramError;
use thiserror::Error;

/// Result type for light-token specific errors
pub type LightTokenResult<T> = std::result::Result<T, LightTokenError>;

/// Errors specific to high-level token operations in light-token.
/// Error codes start at 17500 to avoid conflicts with TokenSdkError (17001-17033).
#[derive(Debug, Error)]
pub enum LightTokenError {
    #[error("SPL interface required for this operation")]
    SplInterfaceRequired,
    #[error("Incomplete SPL interface configuration")]
    IncompleteSplInterface,
    #[error("Use regular SPL transfer for this operation")]
    UseRegularSplTransfer,
    #[error("Cannot determine account type")]
    CannotDetermineAccountType,
    #[error("Missing mint account")]
    MissingMintAccount,
    #[error("Missing SPL token program")]
    MissingSplTokenProgram,
    #[error("Missing SPL interface PDA")]
    MissingSplInterfacePda,
    #[error("Missing SPL interface PDA bump")]
    MissingSplInterfacePdaBump,
    #[error("SPL token program mismatch between source and destination")]
    SplTokenProgramMismatch,
    #[error("Invalid account data")]
    InvalidAccountData,
    #[error("Serialization error")]
    SerializationError,
}

impl From<LightTokenError> for ProgramError {
    fn from(e: LightTokenError) -> Self {
        ProgramError::Custom(e.into())
    }
}

impl From<LightTokenError> for u32 {
    fn from(e: LightTokenError) -> Self {
        match e {
            LightTokenError::SplInterfaceRequired => 17500,
            LightTokenError::IncompleteSplInterface => 17501,
            LightTokenError::UseRegularSplTransfer => 17502,
            LightTokenError::CannotDetermineAccountType => 17503,
            LightTokenError::MissingMintAccount => 17504,
            LightTokenError::MissingSplTokenProgram => 17505,
            LightTokenError::MissingSplInterfacePda => 17506,
            LightTokenError::MissingSplInterfacePdaBump => 17507,
            LightTokenError::SplTokenProgramMismatch => 17508,
            LightTokenError::InvalidAccountData => 17509,
            LightTokenError::SerializationError => 17510,
        }
    }
}

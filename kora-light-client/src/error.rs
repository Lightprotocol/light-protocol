//! Error types for kora-light-client.

#[derive(Debug, thiserror::Error)]
pub enum KoraLightError {
    #[error("Cannot determine account type from owner")]
    CannotDetermineAccountType,

    #[error("Insufficient balance: need {needed}, have {available}")]
    InsufficientBalance { needed: u64, available: u64 },

    #[error("No compressed accounts provided")]
    NoCompressedAccounts,

    #[error("Borsh serialization error: {0}")]
    BorshError(#[from] std::io::Error),

    #[error("Arithmetic overflow")]
    ArithmeticOverflow,

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

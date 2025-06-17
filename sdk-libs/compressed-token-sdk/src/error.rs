use thiserror::Error;

pub type Result<T> = std::result::Result<T, TokenSdkError>;

#[derive(Debug, Error)]
pub enum TokenSdkError {
    #[error("Insufficient balance")]
    InsufficientBalance,
    #[error("Serialization error")]
    SerializationError,
    #[error("CPI error: {0}")]
    CpiError(String),
    #[error("Cannot compress and decompress")]
    CannotCompressAndDecompress,
}

// Keep old error type for backwards compatibility
pub type CTokenSdkError = TokenSdkError;

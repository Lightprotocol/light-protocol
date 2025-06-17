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
    #[error("Inconsistent compress/decompress state")]
    InconsistentCompressDecompressState,
    #[error("Both compress and decompress specified")]
    BothCompressAndDecompress,
    #[error("Invalid compress/decompress amount")]
    InvalidCompressDecompressAmount,
}


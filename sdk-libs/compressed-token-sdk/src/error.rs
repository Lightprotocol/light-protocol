use light_compressed_token_types::error::LightTokenSdkTypeError;
use solana_program_error::ProgramError;
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
    #[error("Ctoken::transfer, compress, or decompress cannot be used with fn transfer(), fn compress(), fn decompress()")]
    MethodUsed,
    #[error(transparent)]
    CompressedTokenTypes(#[from] LightTokenSdkTypeError),
}

impl From<TokenSdkError> for ProgramError {
    fn from(e: TokenSdkError) -> Self {
        ProgramError::Custom(e.into())
    }
}

impl From<TokenSdkError> for u32 {
    fn from(e: TokenSdkError) -> Self {
        match e {
            TokenSdkError::InsufficientBalance => 17001,
            TokenSdkError::SerializationError => 17002,
            TokenSdkError::CpiError(_) => 17003,
            TokenSdkError::CannotCompressAndDecompress => 17004,
            TokenSdkError::InconsistentCompressDecompressState => 17005,
            TokenSdkError::BothCompressAndDecompress => 17006,
            TokenSdkError::InvalidCompressDecompressAmount => 17007,
            TokenSdkError::MethodUsed => 17008,
            TokenSdkError::CompressedTokenTypes(e) => e.into(),
        }
    }
}

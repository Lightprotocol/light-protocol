use thiserror::Error;

pub type Result<T> = std::result::Result<T, LightTokenSdkTypeError>;

#[derive(Debug, Error)]
pub enum LightTokenSdkTypeError {
    #[error("CPI accounts index out of bounds: {0}")]
    CpiAccountsIndexOutOfBounds(usize),
    #[error("Sender token account does only exist in compressed mode")]
    SenderTokenAccountDoesOnlyExistInCompressedMode,
    #[error("Decompression recipient token account does only exist in decompressed mode")]
    DecompressionRecipientTokenAccountDoesOnlyExistInDecompressedMode,
    #[error("Sol pool PDA is undefined")]
    SolPoolPdaUndefined,
    #[error("Mint is undefined for batch compress")]
    MintUndefinedForBatchCompress,
    #[error("Token pool PDA is undefined for compressed")]
    TokenPoolUndefinedForCompressed,
    #[error("Token program is undefined for compressed")]
    TokenProgramUndefinedForCompressed,
}

impl From<LightTokenSdkTypeError> for u32 {
    fn from(error: LightTokenSdkTypeError) -> Self {
        match error {
            LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(_) => 18001,
            LightTokenSdkTypeError::SenderTokenAccountDoesOnlyExistInCompressedMode => 18002,
            LightTokenSdkTypeError::DecompressionRecipientTokenAccountDoesOnlyExistInDecompressedMode => 18003,
            LightTokenSdkTypeError::SolPoolPdaUndefined => 18004,
            LightTokenSdkTypeError::MintUndefinedForBatchCompress => 18005,
            LightTokenSdkTypeError::TokenPoolUndefinedForCompressed => 18006,
            LightTokenSdkTypeError::TokenProgramUndefinedForCompressed => 18007,
        }
    }
}

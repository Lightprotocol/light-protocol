use light_account_checks::AccountError;
use light_sdk::error::LightSdkError;
use light_sdk_types::error::LightSdkTypesError;
use light_token_interface::TokenError;
use light_token_types::error::LightTokenSdkTypeError;
use light_zero_copy::errors::ZeroCopyError;
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
    #[error("Compression cannot be set twice")]
    CompressionCannotBeSetTwice,
    #[error("Inconsistent compress/decompress state")]
    InconsistentCompressDecompressState,
    #[error("Both compress and decompress specified")]
    BothCompressAndDecompress,
    #[error("Invalid compress/decompress amount")]
    InvalidCompressDecompressAmount,
    #[error("Ctoken::transfer, compress, or decompress cannot be used with fn transfer(), fn compress(), fn decompress()")]
    MethodUsed,
    #[error("DecompressedMintConfig is required for decompressed mints")]
    DecompressedMintConfigRequired,
    #[error("Invalid compress input owner")]
    InvalidCompressInputOwner,
    #[error("Account borrow failed")]
    AccountBorrowFailed,
    #[error("Invalid account data")]
    InvalidAccountData,
    #[error("Missing required CPI account")]
    MissingCpiAccount,
    #[error("Too many accounts")]
    TooManyAccounts,
    #[error("PackedAccount indices are not continuous")]
    NonContinuousIndices,
    #[error("PackedAccount index out of bounds")]
    PackedAccountIndexOutOfBounds,
    #[error("Cannot mint with decompressed mint in CPI write mode")]
    CannotMintWithDecompressedInCpiWrite,
    #[error("RentAuthorityIsNone")]
    RentAuthorityIsNone,
    #[error("Incomplete SPL interface")]
    IncompleteSplInterface,
    #[error("SPL interface required")]
    SplInterfaceRequired,
    #[error("Use regular SPL transfer")]
    UseRegularSplTransfer,
    #[error("Cannot determine account type")]
    CannotDetermineAccountType,
    #[error("MintActionMetaConfig::new_cpi_context requires cpi_context data")]
    CpiContextRequired,
    #[error("Missing mint account")]
    MissingMintAccount,
    #[error("Missing SPL token program")]
    MissingSplTokenProgram,
    #[error("Missing SPL interface PDA")]
    MissingSplInterfacePda,
    #[error("Missing SPL interface PDA bump")]
    MissingSplInterfacePdaBump,
    #[error("Invalid CPI context: first_set_context or set_context must be true")]
    InvalidCpiContext,
    #[error("No input accounts provided")]
    NoInputAccounts,
    #[error("Missing Compressible extension on Light Token account")]
    MissingCompressibleExtension,
    #[error("Invalid Token account data")]
    InvalidTokenAccount,
    #[error(transparent)]
    CompressedTokenTypes(#[from] LightTokenSdkTypeError),
    #[error(transparent)]
    TokenError(#[from] TokenError),
    #[error(transparent)]
    LightSdkError(#[from] LightSdkError),
    #[error(transparent)]
    LightSdkTypesError(#[from] LightSdkTypesError),
    #[error(transparent)]
    ZeroCopyError(#[from] ZeroCopyError),
    #[error(transparent)]
    AccountError(#[from] AccountError),
}
#[cfg(feature = "anchor")]
impl From<TokenSdkError> for anchor_lang::prelude::ProgramError {
    fn from(e: TokenSdkError) -> Self {
        ProgramError::Custom(e.into())
    }
}
#[cfg(not(feature = "anchor"))]
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
            TokenSdkError::CompressionCannotBeSetTwice => 17005,
            TokenSdkError::InconsistentCompressDecompressState => 17006,
            TokenSdkError::BothCompressAndDecompress => 17007,
            TokenSdkError::InvalidCompressDecompressAmount => 17008,
            TokenSdkError::MethodUsed => 17009,
            TokenSdkError::DecompressedMintConfigRequired => 17010,
            TokenSdkError::InvalidCompressInputOwner => 17011,
            TokenSdkError::AccountBorrowFailed => 17012,
            TokenSdkError::InvalidAccountData => 17013,
            TokenSdkError::MissingCpiAccount => 17014,
            TokenSdkError::TooManyAccounts => 17015,
            TokenSdkError::NonContinuousIndices => 17016,
            TokenSdkError::PackedAccountIndexOutOfBounds => 17017,
            TokenSdkError::CannotMintWithDecompressedInCpiWrite => 17018,
            TokenSdkError::RentAuthorityIsNone => 17019,
            TokenSdkError::SplInterfaceRequired => 17020,
            TokenSdkError::IncompleteSplInterface => 17021,
            TokenSdkError::UseRegularSplTransfer => 17022,
            TokenSdkError::CannotDetermineAccountType => 17023,
            TokenSdkError::CpiContextRequired => 17024,
            TokenSdkError::MissingMintAccount => 17025,
            TokenSdkError::MissingSplTokenProgram => 17026,
            TokenSdkError::MissingSplInterfacePda => 17027,
            TokenSdkError::MissingSplInterfacePdaBump => 17028,
            TokenSdkError::InvalidCpiContext => 17029,
            TokenSdkError::NoInputAccounts => 17030,
            TokenSdkError::MissingCompressibleExtension => 17031,
            TokenSdkError::InvalidTokenAccount => 17032,
            TokenSdkError::CompressedTokenTypes(e) => e.into(),
            TokenSdkError::TokenError(e) => e.into(),
            TokenSdkError::LightSdkTypesError(e) => e.into(),
            TokenSdkError::LightSdkError(e) => e.into(),
            TokenSdkError::ZeroCopyError(e) => e.into(),
            TokenSdkError::AccountError(e) => e.into(),
        }
    }
}

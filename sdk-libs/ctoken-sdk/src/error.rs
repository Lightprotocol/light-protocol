use light_account_checks::AccountError;
use light_token_interface::TokenError;
use light_token_types::error::LightTokenSdkTypeError;
use light_sdk::error::LightSdkError;
use light_sdk_types::error::LightSdkTypesError;
use light_zero_copy::errors::ZeroCopyError;
use solana_program_error::ProgramError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CTokenSdkError>;

#[derive(Debug, Error)]
pub enum CTokenSdkError {
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
    #[error("Missing Compressible extension on CToken account")]
    MissingCompressibleExtension,
    #[error("Invalid CToken account data")]
    InvalidCTokenAccount,
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
impl From<CTokenSdkError> for anchor_lang::prelude::ProgramError {
    fn from(e: CTokenSdkError) -> Self {
        ProgramError::Custom(e.into())
    }
}
#[cfg(not(feature = "anchor"))]
impl From<CTokenSdkError> for ProgramError {
    fn from(e: CTokenSdkError) -> Self {
        ProgramError::Custom(e.into())
    }
}

impl From<CTokenSdkError> for u32 {
    fn from(e: CTokenSdkError) -> Self {
        match e {
            CTokenSdkError::InsufficientBalance => 17001,
            CTokenSdkError::SerializationError => 17002,
            CTokenSdkError::CpiError(_) => 17003,
            CTokenSdkError::CannotCompressAndDecompress => 17004,
            CTokenSdkError::CompressionCannotBeSetTwice => 17005,
            CTokenSdkError::InconsistentCompressDecompressState => 17006,
            CTokenSdkError::BothCompressAndDecompress => 17007,
            CTokenSdkError::InvalidCompressDecompressAmount => 17008,
            CTokenSdkError::MethodUsed => 17009,
            CTokenSdkError::DecompressedMintConfigRequired => 17010,
            CTokenSdkError::InvalidCompressInputOwner => 17011,
            CTokenSdkError::AccountBorrowFailed => 17012,
            CTokenSdkError::InvalidAccountData => 17013,
            CTokenSdkError::MissingCpiAccount => 17014,
            CTokenSdkError::TooManyAccounts => 17015,
            CTokenSdkError::NonContinuousIndices => 17016,
            CTokenSdkError::PackedAccountIndexOutOfBounds => 17017,
            CTokenSdkError::CannotMintWithDecompressedInCpiWrite => 17018,
            CTokenSdkError::RentAuthorityIsNone => 17019,
            CTokenSdkError::SplInterfaceRequired => 17020,
            CTokenSdkError::IncompleteSplInterface => 17021,
            CTokenSdkError::UseRegularSplTransfer => 17022,
            CTokenSdkError::CannotDetermineAccountType => 17023,
            CTokenSdkError::CpiContextRequired => 17024,
            CTokenSdkError::MissingMintAccount => 17025,
            CTokenSdkError::MissingSplTokenProgram => 17026,
            CTokenSdkError::MissingSplInterfacePda => 17027,
            CTokenSdkError::MissingSplInterfacePdaBump => 17028,
            CTokenSdkError::InvalidCpiContext => 17029,
            CTokenSdkError::NoInputAccounts => 17030,
            CTokenSdkError::MissingCompressibleExtension => 17031,
            CTokenSdkError::InvalidCTokenAccount => 17032,
            CTokenSdkError::CompressedTokenTypes(e) => e.into(),
            CTokenSdkError::TokenError(e) => e.into(),
            CTokenSdkError::LightSdkTypesError(e) => e.into(),
            CTokenSdkError::LightSdkError(e) => e.into(),
            CTokenSdkError::ZeroCopyError(e) => e.into(),
            CTokenSdkError::AccountError(e) => e.into(),
        }
    }
}

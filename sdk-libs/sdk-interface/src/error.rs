use light_account_checks::error::AccountError;
use light_compressed_account::CompressedAccountError;
use light_hasher::HasherError;
use light_sdk_types::error::LightSdkTypesError;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum LightPdaError {
    #[error("Constraint violation")]
    ConstraintViolation,
    #[error("Borsh error.")]
    Borsh,
    #[error("Account check error: {0}")]
    AccountCheck(#[from] AccountError),
    #[error("Hasher error: {0}")]
    Hasher(#[from] HasherError),
    #[error("Missing compression_info field")]
    MissingCompressionInfo,
    #[error("Rent sponsor account does not match the expected PDA from config")]
    InvalidRentSponsor,
    #[error("Borsh IO error: {0}")]
    BorshIo(String),
    #[error("CPI accounts index out of bounds: {0}")]
    CpiAccountsIndexOutOfBounds(usize),
    #[error("Read-only accounts are not supported in write_to_cpi_context operations")]
    ReadOnlyAccountsNotSupportedInCpiContext,
    #[error("Compressed account error: {0}")]
    CompressedAccountError(#[from] CompressedAccountError),
    #[error("Account data too small")]
    AccountDataTooSmall,
    #[error("Invalid instruction data")]
    InvalidInstructionData,
    #[error("Invalid seeds")]
    InvalidSeeds,
    #[error("CPI invocation failed")]
    CpiFailed,
    #[error("Not enough account keys")]
    NotEnoughAccountKeys,
    #[error("Missing required signature")]
    MissingRequiredSignature,
}

pub type Result<T> = core::result::Result<T, LightPdaError>;

impl From<LightSdkTypesError> for LightPdaError {
    fn from(e: LightSdkTypesError) -> Self {
        match e {
            LightSdkTypesError::CpiAccountsIndexOutOfBounds(index) => {
                LightPdaError::CpiAccountsIndexOutOfBounds(index)
            }
            LightSdkTypesError::AccountError(e) => LightPdaError::AccountCheck(e),
            LightSdkTypesError::Hasher(e) => LightPdaError::Hasher(e),
            _ => LightPdaError::ConstraintViolation,
        }
    }
}

impl From<LightPdaError> for u32 {
    fn from(e: LightPdaError) -> Self {
        match e {
            LightPdaError::ConstraintViolation => 17001,
            LightPdaError::Borsh => 17002,
            LightPdaError::AccountCheck(e) => e.into(),
            LightPdaError::Hasher(e) => e.into(),
            LightPdaError::MissingCompressionInfo => 17003,
            LightPdaError::InvalidRentSponsor => 17004,
            LightPdaError::BorshIo(_) => 17005,
            LightPdaError::CpiAccountsIndexOutOfBounds(_) => 17006,
            LightPdaError::ReadOnlyAccountsNotSupportedInCpiContext => 17007,
            LightPdaError::CompressedAccountError(e) => e.into(),
            LightPdaError::AccountDataTooSmall => 17008,
            LightPdaError::InvalidInstructionData => 17009,
            LightPdaError::InvalidSeeds => 17010,
            LightPdaError::CpiFailed => 17011,
            LightPdaError::NotEnoughAccountKeys => 17012,
            LightPdaError::MissingRequiredSignature => 17013,
        }
    }
}

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
    #[cfg(feature = "solana")]
    #[error("Program error: {0}")]
    ProgramError(#[from] solana_program_error::ProgramError),
    #[error("Borsh IO error: {0}")]
    BorshIo(String),
    #[error("CPI accounts index out of bounds: {0}")]
    CpiAccountsIndexOutOfBounds(usize),
    #[error("Read-only accounts are not supported in write_to_cpi_context operations")]
    ReadOnlyAccountsNotSupportedInCpiContext,
    #[error("Compressed account error: {0}")]
    CompressedAccountError(#[from] CompressedAccountError),
}

pub type Result<T> = core::result::Result<T, LightPdaError>;

#[cfg(feature = "solana")]
impl From<LightPdaError> for solana_program_error::ProgramError {
    fn from(e: LightPdaError) -> Self {
        solana_program_error::ProgramError::Custom(u32::from(e))
    }
}

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
            #[cfg(feature = "solana")]
            LightPdaError::ProgramError(e) => u64::from(e) as u32,
            LightPdaError::BorshIo(_) => 17005,
            LightPdaError::CpiAccountsIndexOutOfBounds(_) => 17006,
            LightPdaError::ReadOnlyAccountsNotSupportedInCpiContext => 17007,
            LightPdaError::CompressedAccountError(e) => e.into(),
        }
    }
}

#[cfg(feature = "solana")]
impl From<core::cell::BorrowError> for LightPdaError {
    fn from(_e: core::cell::BorrowError) -> Self {
        LightPdaError::AccountCheck(AccountError::BorrowAccountDataFailed)
    }
}

#[cfg(feature = "solana")]
impl From<core::cell::BorrowMutError> for LightPdaError {
    fn from(_e: core::cell::BorrowMutError) -> Self {
        LightPdaError::AccountCheck(AccountError::BorrowAccountDataFailed)
    }
}

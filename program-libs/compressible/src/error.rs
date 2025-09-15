use light_hasher::HasherError;
use thiserror::Error;
#[derive(Debug, Error, PartialEq)]
pub enum CompressibleError {
    #[error("ConstraintViolation")]
    ConstraintViolation,
    #[error("FailedBorrowRentSysvar")]
    FailedBorrowRentSysvar,
    #[error("InvalidState{0}")]
    InvalidState(u8),
    #[error("Hasher error {0}")]
    HasherError(#[from] HasherError),
}

// Numberspace 19_*
impl From<CompressibleError> for u32 {
    fn from(e: CompressibleError) -> u32 {
        match e {
            CompressibleError::ConstraintViolation => 19001,
            CompressibleError::FailedBorrowRentSysvar => 19002,
            CompressibleError::InvalidState(_) => 19003,
            CompressibleError::HasherError(e) => u32::from(e),
        }
    }
}

#[cfg(feature = "solana")]
impl From<CompressibleError> for solana_program_error::ProgramError {
    fn from(e: CompressibleError) -> Self {
        solana_program_error::ProgramError::Custom(e.into())
    }
}

#[cfg(feature = "pinocchio")]
impl From<CompressibleError> for pinocchio::program_error::ProgramError {
    fn from(e: CompressibleError) -> Self {
        pinocchio::program_error::ProgramError::Custom(e.into())
    }
}

#[cfg(feature = "anchor")]
impl From<CompressibleError> for anchor_lang::prelude::ProgramError {
    fn from(e: CompressibleError) -> Self {
        anchor_lang::prelude::ProgramError::Custom(e.into())
    }
}

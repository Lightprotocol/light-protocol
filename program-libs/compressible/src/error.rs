use light_hasher::HasherError;
use thiserror::Error;
#[derive(Debug, Error, PartialEq)]
pub enum CompressibleError {
    #[error("FailedBorrowRentSysvar")]
    FailedBorrowRentSysvar,
    #[error("InvalidState{0}")]
    InvalidState(u8),
    #[error("InvalidVersion")]
    InvalidVersion,
    #[error("Hasher error {0}")]
    HasherError(#[from] HasherError),
}

// Numberspace 19_*
impl From<CompressibleError> for u32 {
    fn from(e: CompressibleError) -> u32 {
        match e {
            CompressibleError::FailedBorrowRentSysvar => 19001,
            CompressibleError::InvalidState(_) => 19002,
            CompressibleError::InvalidVersion => 19003,
            CompressibleError::HasherError(e) => u32::from(e),
        }
    }
}

#[cfg(all(feature = "solana", not(feature = "anchor")))]
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

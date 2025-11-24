use anchor_lang::prelude::{Error, ProgramError};

#[repr(u32)]
pub enum ErrorCode {
    RentRecipientMismatch,
    InvalidAuthority,
    InvalidMintAuthority,
    InvalidFeePayer,
}

impl From<ErrorCode> for ProgramError {
    fn from(e: ErrorCode) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl From<ErrorCode> for Error {
    fn from(e: ErrorCode) -> Self {
        Error::from(ProgramError::from(e))
    }
}

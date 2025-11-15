use anchor_lang::prelude::ProgramError;

#[repr(u32)]
pub enum ErrorCode {
    RentRecipientMismatch,
}

impl From<ErrorCode> for ProgramError {
    fn from(e: ErrorCode) -> Self {
        ProgramError::Custom(e as u32)
    }
}

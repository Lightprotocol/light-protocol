use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum AccountError {
    #[error("Account owned by wrong program.")]
    AccountOwnedByWrongProgram,
    #[error("Account not mutable.")]
    AccountNotMutable,
    #[error("Invalid Discriminator.")]
    InvalidDiscriminator,
    #[error("Borrow account data failed.")]
    BorrowAccountDataFailed,
    #[error("Account is already initialized.")]
    AlreadyInitialized,
    #[error("Invalid Account size.")]
    InvalidAccountSize,
    #[error("Account is mutable.")]
    AccountMutable,
    #[error("Invalid account balance.")]
    InvalidAccountBalance,
    #[error("Failed to borrow rent sysvar.")]
    FailedBorrowRentSysvar,
}

// TODO: reconfigure error codes
impl From<AccountError> for u32 {
    fn from(e: AccountError) -> u32 {
        match e {
            AccountError::AccountOwnedByWrongProgram => 12007,
            AccountError::AccountNotMutable => 12008,
            AccountError::InvalidDiscriminator => 12006,
            AccountError::BorrowAccountDataFailed => 12009,
            AccountError::InvalidAccountSize => 12010,
            AccountError::AccountMutable => 12011,
            AccountError::AlreadyInitialized => 12012,
            AccountError::InvalidAccountBalance => 12013,
            AccountError::FailedBorrowRentSysvar => 12014,
        }
    }
}

impl From<AccountError> for solana_program::program_error::ProgramError {
    fn from(e: AccountError) -> Self {
        solana_program::program_error::ProgramError::Custom(e.into())
    }
}

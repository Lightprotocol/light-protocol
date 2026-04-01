use thiserror::Error;

#[derive(Debug, Clone, Copy, Error, PartialEq)]
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
    #[error("Invalid Signer")]
    InvalidSigner,
    #[error("Invalid Seeds")]
    InvalidSeeds,
    #[error("Invalid Program Id")]
    InvalidProgramId,
    #[error("Program not executable.")]
    ProgramNotExecutable,
    #[error("Account not zeroed.")]
    AccountNotZeroed,
    #[error("Not enough account keys provided.")]
    NotEnoughAccountKeys,
    #[error("Invalid Account.")]
    InvalidAccount,
    #[error("Failed to access sysvar.")]
    FailedSysvarAccess,
    #[error("Pinocchio program error with code: {0}")]
    PinocchioProgramError(u32),
    #[error("Arithmetic overflow.")]
    ArithmeticOverflow,
}

impl From<AccountError> for u32 {
    fn from(e: AccountError) -> u32 {
        match e {
            AccountError::InvalidDiscriminator => 20000,
            AccountError::AccountOwnedByWrongProgram => 20001,
            AccountError::AccountNotMutable => 20002,
            AccountError::BorrowAccountDataFailed => 20003,
            AccountError::InvalidAccountSize => 20004,
            AccountError::AccountMutable => 20005,
            AccountError::AlreadyInitialized => 20006,
            AccountError::InvalidAccountBalance => 20007,
            AccountError::FailedBorrowRentSysvar => 20008,
            AccountError::InvalidSigner => 20009,
            AccountError::InvalidSeeds => 20010,
            AccountError::InvalidProgramId => 20011,
            AccountError::ProgramNotExecutable => 20012,
            AccountError::AccountNotZeroed => 20013,
            AccountError::NotEnoughAccountKeys => 20014,
            AccountError::InvalidAccount => 20015,
            AccountError::FailedSysvarAccess => 20016,
            AccountError::PinocchioProgramError(code) => code,
            AccountError::ArithmeticOverflow => 20017,
        }
    }
}


#[cfg(feature = "solana")]
impl From<AccountError> for solana_program_error::ProgramError {
    fn from(e: AccountError) -> Self {
        solana_program_error::ProgramError::Custom(e.into())
    }
}

#[cfg(any(feature = "pinocchio", feature = "solana"))]
impl From<solana_program_error::ProgramError> for AccountError {
    fn from(e: solana_program_error::ProgramError) -> Self {
        match e {
            solana_program_error::ProgramError::Custom(code) => {
                AccountError::PinocchioProgramError(code)
            }
            _ => AccountError::PinocchioProgramError(u64::from(e) as u32),
        }
    }
}

#[cfg(feature = "solana")]
impl From<core::cell::BorrowError> for AccountError {
    fn from(_: core::cell::BorrowError) -> Self {
        AccountError::BorrowAccountDataFailed
    }
}

#[cfg(feature = "solana")]
impl From<core::cell::BorrowMutError> for AccountError {
    fn from(_: core::cell::BorrowMutError) -> Self {
        AccountError::BorrowAccountDataFailed
    }
}

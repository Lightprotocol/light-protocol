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
    #[error("Pinocchio program error with code: {0}")]
    PinocchioProgramError(u32),
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
            AccountError::PinocchioProgramError(code) => code,
        }
    }
}

#[cfg(feature = "pinocchio")]
impl From<AccountError> for pinocchio::program_error::ProgramError {
    fn from(e: AccountError) -> Self {
        pinocchio::program_error::ProgramError::Custom(e.into())
    }
}

#[cfg(feature = "solana")]
impl From<AccountError> for solana_program_error::ProgramError {
    fn from(e: AccountError) -> Self {
        solana_program_error::ProgramError::Custom(e.into())
    }
}

#[cfg(feature = "pinocchio")]
impl From<pinocchio::program_error::ProgramError> for AccountError {
    fn from(error: pinocchio::program_error::ProgramError) -> Self {
        match error {
            pinocchio::program_error::ProgramError::Custom(code) => {
                AccountError::PinocchioProgramError(code)
            }
            _ => {
                // Convert other ProgramError variants to error codes
                let error_code = match error {
                    pinocchio::program_error::ProgramError::InvalidArgument => 1,
                    pinocchio::program_error::ProgramError::InvalidInstructionData => 2,
                    pinocchio::program_error::ProgramError::InvalidAccountData => 3,
                    pinocchio::program_error::ProgramError::AccountDataTooSmall => 4,
                    pinocchio::program_error::ProgramError::InsufficientFunds => 5,
                    pinocchio::program_error::ProgramError::IncorrectProgramId => 6,
                    pinocchio::program_error::ProgramError::MissingRequiredSignature => 7,
                    pinocchio::program_error::ProgramError::AccountAlreadyInitialized => 8,
                    pinocchio::program_error::ProgramError::UninitializedAccount => 9,
                    pinocchio::program_error::ProgramError::NotEnoughAccountKeys => 10,
                    pinocchio::program_error::ProgramError::AccountBorrowFailed => 11,
                    _ => 0, // Unknown error
                };
                AccountError::PinocchioProgramError(error_code)
            }
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

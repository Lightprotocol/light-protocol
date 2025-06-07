use light_hasher::HasherError;
use thiserror::Error;

use crate::ProgramError;

pub type Result<T> = std::result::Result<T, LightSdkError>;

#[derive(Debug, Error, PartialEq)]
pub enum LightSdkError {
    #[error("Constraint violation")]
    ConstraintViolation,
    #[error("Invalid light-system-program ID")]
    InvalidLightSystemProgram,
    #[error("Expected accounts in the instruction")]
    ExpectedAccounts,
    #[error("Expected address Merkle context to be provided")]
    ExpectedAddressTreeInfo,
    #[error("Expected address root index to be provided")]
    ExpectedAddressRootIndex,
    #[error("Accounts with a specified input are expected to have data")]
    ExpectedData,
    #[error("Accounts with specified data are expected to have a discriminator")]
    ExpectedDiscriminator,
    #[error("Accounts with specified data are expected to have a hash")]
    ExpectedHash,
    #[error("Expected the `{0}` light account to be provided")]
    ExpectedLightSystemAccount(String),
    #[error("`mut` and `close` accounts are expected to have a Merkle context")]
    ExpectedMerkleContext,
    #[error("Expected root index to be provided")]
    ExpectedRootIndex,
    #[error("Cannot transfer lamports from an account without input")]
    TransferFromNoInput,
    #[error("Cannot transfer from an account without lamports")]
    TransferFromNoLamports,
    #[error("Account, from which a transfer was attempted, has insufficient amount of lamports")]
    TransferFromInsufficientLamports,
    #[error("Integer overflow resulting from too large resulting amount")]
    TransferIntegerOverflow,
    #[error("Borsh error.")]
    Borsh,
    #[error("Fewer accounts than number of system accounts.")]
    FewerAccountsThanSystemAccounts,
    #[error("InvalidCpiSignerAccount")]
    InvalidCpiSignerAccount,
    #[error("Missing meta field: {0}")]
    MissingField(String),
    #[error("Output state tree index is none. Use an CompressedAccountMeta type with output tree index to initialize or update accounts.")]
    OutputStateTreeIndexIsNone,
    #[error(transparent)]
    Hasher(#[from] HasherError),
    #[error("Program error: {0}")]
    ProgramError(#[from] ProgramError),
}

impl From<LightSdkError> for u32 {
    fn from(e: LightSdkError) -> Self {
        match e {
            LightSdkError::ConstraintViolation => 14001,
            LightSdkError::InvalidLightSystemProgram => 14002,
            LightSdkError::ExpectedAccounts => 14003,
            LightSdkError::ExpectedAddressTreeInfo => 14004,
            LightSdkError::ExpectedAddressRootIndex => 14005,
            LightSdkError::ExpectedData => 14006,
            LightSdkError::ExpectedDiscriminator => 14007,
            LightSdkError::ExpectedHash => 14008,
            LightSdkError::ExpectedLightSystemAccount(_) => 14009,
            LightSdkError::ExpectedMerkleContext => 14010,
            LightSdkError::ExpectedRootIndex => 14011,
            LightSdkError::TransferFromNoInput => 14012,
            LightSdkError::TransferFromNoLamports => 14013,
            LightSdkError::TransferFromInsufficientLamports => 14014,
            LightSdkError::TransferIntegerOverflow => 14015,
            LightSdkError::Borsh => 14016,
            LightSdkError::FewerAccountsThanSystemAccounts => 14017,
            LightSdkError::InvalidCpiSignerAccount => 14018,
            LightSdkError::MissingField(_) => 14019,
            LightSdkError::OutputStateTreeIndexIsNone => 14020,
            LightSdkError::Hasher(e) => e.into(),
            LightSdkError::ProgramError(e) => u32::try_from(u64::from(e)).unwrap(),
        }
    }
}

impl From<LightSdkError> for ProgramError {
    fn from(e: LightSdkError) -> Self {
        ProgramError::Custom(e.into())
    }
}

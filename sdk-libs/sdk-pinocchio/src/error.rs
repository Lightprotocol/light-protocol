use light_hasher::HasherError;
pub use light_sdk_types::error::LightSdkTypesError;
use light_zero_copy::errors::ZeroCopyError;
use pinocchio::program_error::ProgramError;
use thiserror::Error;

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
    #[error("Address is none during initialization")]
    InitAddressIsNone,
    #[error("Address is none during initialization with address")]
    InitWithAddressIsNone,
    #[error("Output is none during initialization with address")]
    InitWithAddressOutputIsNone,
    #[error("Address is none during meta mutation")]
    MetaMutAddressIsNone,
    #[error("Input is none during meta mutation")]
    MetaMutInputIsNone,
    #[error("Output lamports is none during meta mutation")]
    MetaMutOutputLamportsIsNone,
    #[error("Output is none during meta mutation")]
    MetaMutOutputIsNone,
    #[error("Address is none during meta close")]
    MetaCloseAddressIsNone,
    #[error("Input is none during meta close")]
    MetaCloseInputIsNone,
    #[error("CPI accounts index out of bounds: {0}")]
    CpiAccountsIndexOutOfBounds(usize),
    #[error(transparent)]
    Hasher(#[from] HasherError),
    #[error(transparent)]
    ZeroCopy(#[from] ZeroCopyError),
    #[error("Program error: {0:?}")]
    ProgramError(ProgramError),
}

impl From<ProgramError> for LightSdkError {
    fn from(error: ProgramError) -> Self {
        LightSdkError::ProgramError(error)
    }
}

impl From<LightSdkError> for ProgramError {
    fn from(e: LightSdkError) -> Self {
        ProgramError::Custom(e.into())
    }
}

impl From<LightSdkTypesError> for LightSdkError {
    fn from(e: LightSdkTypesError) -> Self {
        match e {
            LightSdkTypesError::InitAddressIsNone => LightSdkError::InitAddressIsNone,
            LightSdkTypesError::InitWithAddressIsNone => LightSdkError::InitWithAddressIsNone,
            LightSdkTypesError::InitWithAddressOutputIsNone => {
                LightSdkError::InitWithAddressOutputIsNone
            }
            LightSdkTypesError::MetaMutAddressIsNone => LightSdkError::MetaMutAddressIsNone,
            LightSdkTypesError::MetaMutInputIsNone => LightSdkError::MetaMutInputIsNone,
            LightSdkTypesError::MetaMutOutputLamportsIsNone => {
                LightSdkError::MetaMutOutputLamportsIsNone
            }
            LightSdkTypesError::MetaMutOutputIsNone => LightSdkError::MetaMutOutputIsNone,
            LightSdkTypesError::MetaCloseAddressIsNone => LightSdkError::MetaCloseAddressIsNone,
            LightSdkTypesError::MetaCloseInputIsNone => LightSdkError::MetaCloseInputIsNone,
            LightSdkTypesError::Hasher(e) => LightSdkError::Hasher(e),
            LightSdkTypesError::FewerAccountsThanSystemAccounts => {
                LightSdkError::FewerAccountsThanSystemAccounts
            }
            LightSdkTypesError::CpiAccountsIndexOutOfBounds(index) => {
                LightSdkError::CpiAccountsIndexOutOfBounds(index)
            }
        }
    }
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
            LightSdkError::InitAddressIsNone => 14021,
            LightSdkError::InitWithAddressIsNone => 14022,
            LightSdkError::InitWithAddressOutputIsNone => 14023,
            LightSdkError::MetaMutAddressIsNone => 14024,
            LightSdkError::MetaMutInputIsNone => 14025,
            LightSdkError::MetaMutOutputLamportsIsNone => 14026,
            LightSdkError::MetaMutOutputIsNone => 14027,
            LightSdkError::MetaCloseAddressIsNone => 14028,
            LightSdkError::MetaCloseInputIsNone => 14029,
            LightSdkError::CpiAccountsIndexOutOfBounds(_) => 14031,
            LightSdkError::Hasher(e) => e.into(),
            LightSdkError::ZeroCopy(e) => e.into(),
            LightSdkError::ProgramError(e) => u64::from(e) as u32,
        }
    }
}

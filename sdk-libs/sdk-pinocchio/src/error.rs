use light_account_checks::error::AccountError;
use light_hasher::HasherError;
pub use light_sdk_types::error::LightSdkTypesError;
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
    #[error("Invalid CPI context account")]
    InvalidCpiContextAccount,
    #[error("Invalid sol pool pda account")]
    InvalidSolPoolPdaAccount,
    #[error("CpiAccounts slice starts with an invalid account. It should start with LightSystemProgram SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7.")]
    InvalidCpiAccountsOffset,
    #[error("Mode mismatch between accounts and instruction")]
    ModeMismatch,
    #[error(transparent)]
    Hasher(#[from] HasherError),
    #[error("Compressed account error: {0:?}")]
    CompressedAccount(light_compressed_account::CompressedAccountError),
    #[error("Program error: {0:?}")]
    ProgramError(ProgramError),
    #[error(transparent)]
    AccountError(#[from] AccountError),
}

impl From<ProgramError> for LightSdkError {
    fn from(error: ProgramError) -> Self {
        LightSdkError::ProgramError(error)
    }
}

impl From<light_compressed_account::CompressedAccountError> for LightSdkError {
    fn from(error: light_compressed_account::CompressedAccountError) -> Self {
        LightSdkError::CompressedAccount(error)
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
            LightSdkTypesError::InvalidCpiContextAccount => LightSdkError::InvalidCpiContextAccount,
            LightSdkTypesError::InvalidSolPoolPdaAccount => LightSdkError::InvalidSolPoolPdaAccount,
            LightSdkTypesError::AccountError(e) => LightSdkError::AccountError(e),
            LightSdkTypesError::InvalidCpiAccountsOffset => LightSdkError::InvalidCpiAccountsOffset,
        }
    }
}

impl From<LightSdkError> for u32 {
    fn from(e: LightSdkError) -> Self {
        match e {
            LightSdkError::ConstraintViolation => 16001,
            LightSdkError::InvalidLightSystemProgram => 16002,
            LightSdkError::ExpectedAccounts => 16003,
            LightSdkError::ExpectedAddressTreeInfo => 16004,
            LightSdkError::ExpectedAddressRootIndex => 16005,
            LightSdkError::ExpectedData => 16006,
            LightSdkError::ExpectedDiscriminator => 16007,
            LightSdkError::ExpectedHash => 16008,
            LightSdkError::ExpectedLightSystemAccount(_) => 16009,
            LightSdkError::ExpectedMerkleContext => 16010,
            LightSdkError::ExpectedRootIndex => 16011,
            LightSdkError::TransferFromNoInput => 16012,
            LightSdkError::TransferFromNoLamports => 16013,
            LightSdkError::TransferFromInsufficientLamports => 16014,
            LightSdkError::TransferIntegerOverflow => 16015,
            LightSdkError::Borsh => 16016,
            LightSdkError::FewerAccountsThanSystemAccounts => 16017,
            LightSdkError::InvalidCpiSignerAccount => 16018,
            LightSdkError::MissingField(_) => 16019,
            LightSdkError::OutputStateTreeIndexIsNone => 16020,
            LightSdkError::InitAddressIsNone => 16021,
            LightSdkError::InitWithAddressIsNone => 16022,
            LightSdkError::InitWithAddressOutputIsNone => 16023,
            LightSdkError::MetaMutAddressIsNone => 16024,
            LightSdkError::MetaMutInputIsNone => 16025,
            LightSdkError::MetaMutOutputLamportsIsNone => 16026,
            LightSdkError::MetaMutOutputIsNone => 16027,
            LightSdkError::MetaCloseAddressIsNone => 16028,
            LightSdkError::MetaCloseInputIsNone => 16029,
            LightSdkError::CpiAccountsIndexOutOfBounds(_) => 16031,
            LightSdkError::InvalidCpiContextAccount => 16032,
            LightSdkError::InvalidSolPoolPdaAccount => 16033,
            LightSdkError::InvalidCpiAccountsOffset => 16034,
            LightSdkError::ModeMismatch => 16035,
            LightSdkError::Hasher(e) => e.into(),
            LightSdkError::CompressedAccount(_) => 16036,
            LightSdkError::ProgramError(e) => u64::from(e) as u32,
            LightSdkError::AccountError(e) => e.into(),
        }
    }
}

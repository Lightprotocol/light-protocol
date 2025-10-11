use light_account_checks::error::AccountError;
use light_compressed_account::CompressedAccountError;
use light_hasher::HasherError;
use light_sdk_types::error::LightSdkTypesError;
use light_zero_copy::errors::ZeroCopyError;
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
    #[error("Invalid SolPool PDA account")]
    InvalidSolPoolPdaAccount,
    #[error("CpigAccounts accounts slice starts with an invalid account. It should start with LightSystemProgram SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7.")]
    InvalidCpiAccountsOffset,
    #[error("Expected LightAccount to have no data for closure.")]
    ExpectedNoData,
    #[error("CPI context must be added before any other accounts (next_index must be 0)")]
    CpiContextOrderingViolation,
    #[error("Invalid merkle tree index in CPI accounts")]
    InvalidMerkleTreeIndex,
    #[error(
        "Read-only account cannot use to_account_info(), use to_packed_read_only_account() instead"
    )]
    ReadOnlyAccountCannotUseToAccountInfo,
    #[error("Account is not read-only, cannot use to_packed_read_only_account()")]
    NotReadOnlyAccount,
    #[error("Read-only accounts are not supported in write_to_cpi_context operations")]
    ReadOnlyAccountsNotSupportedInCpiContext,
    #[error(transparent)]
    AccountError(#[from] AccountError),
    #[error(transparent)]
    Hasher(#[from] HasherError),
    #[error(transparent)]
    ZeroCopy(#[from] ZeroCopyError),
    #[error("Program error: {0}")]
    ProgramError(#[from] ProgramError),
    #[error("Compressed account error: {0}")]
    CompressedAccountError(#[from] CompressedAccountError),
    #[error("Expected tree info to be provided for init_if_needed")]
    ExpectedTreeInfo,
    #[error("ExpectedSelfProgram")]
    ExpectedSelfProgram,
    #[error("Expected CPI context to be provided")]
    ExpectedCpiContext,
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
            LightSdkTypesError::FewerAccountsThanSystemAccounts => {
                LightSdkError::FewerAccountsThanSystemAccounts
            }
            LightSdkTypesError::CpiAccountsIndexOutOfBounds(index) => {
                LightSdkError::CpiAccountsIndexOutOfBounds(index)
            }
            LightSdkTypesError::InvalidSolPoolPdaAccount => LightSdkError::InvalidSolPoolPdaAccount,
            LightSdkTypesError::InvalidCpiContextAccount => LightSdkError::InvalidCpiContextAccount,
            LightSdkTypesError::InvalidCpiAccountsOffset => LightSdkError::InvalidCpiAccountsOffset,
            LightSdkTypesError::AccountError(e) => LightSdkError::AccountError(e),
            LightSdkTypesError::Hasher(e) => LightSdkError::Hasher(e),
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
            LightSdkError::ExpectedNoData => 16035,
            LightSdkError::CpiContextOrderingViolation => 16036,
            LightSdkError::InvalidMerkleTreeIndex => 16037,
            LightSdkError::ReadOnlyAccountCannotUseToAccountInfo => 16038,
            LightSdkError::NotReadOnlyAccount => 16039,
            LightSdkError::ReadOnlyAccountsNotSupportedInCpiContext => 16040,
            LightSdkError::AccountError(e) => e.into(),
            LightSdkError::Hasher(e) => e.into(),
            LightSdkError::ZeroCopy(e) => e.into(),
            LightSdkError::ProgramError(e) => u64::from(e) as u32,
            LightSdkError::CompressedAccountError(e) => e.into(),
            LightSdkError::ExpectedTreeInfo => 16041,
            LightSdkError::ExpectedSelfProgram => 16042,
            LightSdkError::ExpectedCpiContext => 16043,
        }
    }
}

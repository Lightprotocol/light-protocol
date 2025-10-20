use light_account_checks::error::AccountError;
use light_hasher::HasherError;
use thiserror::Error;

pub type Result<T> = core::result::Result<T, LightSdkTypesError>;

#[derive(Debug, Error, PartialEq)]
pub enum LightSdkTypesError {
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
    #[error("Fewer accounts than system accounts")]
    FewerAccountsThanSystemAccounts,
    #[error("CPI accounts index out of bounds: {0}")]
    CpiAccountsIndexOutOfBounds(usize),
    #[error("Invalid CPI context account")]
    InvalidCpiContextAccount,
    #[error("Invalid sol pool pda account")]
    InvalidSolPoolPdaAccount,
    #[error("CpigAccounts accounts slice starts with an invalid account. It should start with LightSystemProgram SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7.")]
    InvalidCpiAccountsOffset,
    #[error(transparent)]
    AccountError(#[from] AccountError),
    #[error(transparent)]
    Hasher(#[from] HasherError),
}

impl From<LightSdkTypesError> for u32 {
    fn from(e: LightSdkTypesError) -> Self {
        match e {
            LightSdkTypesError::InitAddressIsNone => 14021,
            LightSdkTypesError::InitWithAddressIsNone => 14022,
            LightSdkTypesError::InitWithAddressOutputIsNone => 14023,
            LightSdkTypesError::MetaMutAddressIsNone => 14024,
            LightSdkTypesError::MetaMutInputIsNone => 14025,
            LightSdkTypesError::MetaMutOutputLamportsIsNone => 14026,
            LightSdkTypesError::MetaMutOutputIsNone => 14027,
            LightSdkTypesError::MetaCloseAddressIsNone => 14028,
            LightSdkTypesError::MetaCloseInputIsNone => 14029,
            LightSdkTypesError::FewerAccountsThanSystemAccounts => 14017,
            LightSdkTypesError::CpiAccountsIndexOutOfBounds(_) => 14031,
            LightSdkTypesError::InvalidCpiContextAccount => 14032,
            LightSdkTypesError::InvalidSolPoolPdaAccount => 14033,
            LightSdkTypesError::InvalidCpiAccountsOffset => 14034,
            LightSdkTypesError::AccountError(e) => e.into(),
            LightSdkTypesError::Hasher(e) => e.into(),
        }
    }
}

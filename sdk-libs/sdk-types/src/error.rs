use light_hasher::HasherError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, LightSdkTypesError>;

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
            LightSdkTypesError::FewerAccountsThanSystemAccounts => 14030,
            LightSdkTypesError::Hasher(e) => e.into(),
        }
    }
}

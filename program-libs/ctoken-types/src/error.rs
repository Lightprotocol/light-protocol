use light_zero_copy::errors::ZeroCopyError;
use thiserror::Error;

#[derive(Debug, PartialEq, Error)]
pub enum CTokenError {
    #[error("Invalid instruction data provided")]
    InvalidInstructionData,

    #[error("Invalid account data format")]
    InvalidAccountData,

    #[error("Arithmetic operation resulted in overflow")]
    ArithmeticOverflow,

    #[error("Failed to compute hash for data")]
    HashComputationError,

    #[error("Invalid or malformed extension data")]
    InvalidExtensionData,

    #[error("Missing required mint authority")]
    MissingMintAuthority,

    #[error("Missing required freeze authority")]
    MissingFreezeAuthority,

    #[error("Invalid metadata pointer configuration")]
    InvalidMetadataPointer,

    #[error("Token metadata validation failed")]
    InvalidTokenMetadata,

    #[error("Insufficient token supply for operation")]
    InsufficientSupply,

    #[error("Token account is frozen and cannot be modified")]
    AccountFrozen,

    #[error("Invalid compressed proof provided")]
    InvalidProof,

    #[error("Address derivation failed")]
    AddressDerivationFailed,

    #[error("Extension type not supported")]
    UnsupportedExtension,

    #[error("Maximum number of extensions exceeded")]
    TooManyExtensions,

    #[error("Invalid merkle tree root index")]
    InvalidRootIndex,

    #[error("Compressed account data size exceeds limit")]
    DataSizeExceeded,

    #[error("Invalid compression mode")]
    InvalidCompressionMode,

    #[error("Insufficient funds for compression.")]
    CompressInsufficientFunds,

    #[error("Failed to access sysvar")]
    SysvarAccessError,

    #[error("Compressed token account TLV is unimplemented.")]
    CompressedTokenAccountTlvUnimplemented,

    #[error("Input accounts lamports length mismatch")]
    InputAccountsLamportsLengthMismatch,

    #[error("Output accounts lamports length mismatch")]
    OutputAccountsLamportsLengthMismatch,

    #[error("Invalid token data version")]
    InvalidTokenDataVersion,

    #[error("Instruction data expected mint authority")]
    InstructionDataExpectedMintAuthority,

    #[error("Zero-copy expected mint authority")]
    ZeroCopyExpectedMintAuthority,

    #[error("Instruction data expected freeze authority")]
    InstructionDataExpectedFreezeAuthority,

    #[error("Zero-copy expected mint authority")]
    ZeroCopyExpectedFreezeAuthority,

    #[error("Invalid authority type provided")]
    InvalidAuthorityType,

    #[error("Expected mint signer account")]
    ExpectedMintSignerAccount,

    #[error("Light hasher error: {0}")]
    HasherError(#[from] light_hasher::HasherError),

    #[error("Light zero copy error: {0}")]
    ZeroCopyError(#[from] ZeroCopyError),

    #[error("Light compressed account error: {0}")]
    CompressedAccountError(#[from] light_compressed_account::CompressedAccountError),

    #[error("Invalid token metadata version")]
    InvalidTokenMetadataVersion,
    #[error("InvalidExtensionConfig")]
    InvalidExtensionConfig,
    #[error("InstructionDataExpectedDelegate")]
    InstructionDataExpectedDelegate,
    #[error("ZeroCopyExpectedDelegate")]
    ZeroCopyExpectedDelegate,
    #[error("TokenDataTlvUnimplemented")]
    TokenDataTlvUnimplemented,
    #[error("InvalidAccountState")]
    InvalidAccountState,
    #[error("BorshFailed")]
    BorshFailed,
    #[error(
        "Too many input compressed accounts. Maximum 8 input accounts allowed per instruction"
    )]
    TooManyInputAccounts,

    #[error("Too many additional metadata elements. Maximum 20 allowed")]
    TooManyAdditionalMetadata,

    #[error("Duplicate metadata key found in additional metadata")]
    DuplicateMetadataKey,

    #[error("Too many PDA seeds. Maximum {0} seeds allowed")]
    TooManySeeds(usize),
}

impl From<CTokenError> for u32 {
    fn from(e: CTokenError) -> u32 {
        match e {
            CTokenError::InvalidInstructionData => 18001,
            CTokenError::InvalidAccountData => 18002,
            CTokenError::ArithmeticOverflow => 18003,
            CTokenError::HashComputationError => 18004,
            CTokenError::InvalidExtensionData => 18005,
            CTokenError::MissingMintAuthority => 18006,
            CTokenError::MissingFreezeAuthority => 18007,
            CTokenError::InvalidMetadataPointer => 18008,
            CTokenError::InvalidTokenMetadata => 18009,
            CTokenError::InsufficientSupply => 18010,
            CTokenError::AccountFrozen => 18011,
            CTokenError::InvalidProof => 18012,
            CTokenError::AddressDerivationFailed => 18013,
            CTokenError::UnsupportedExtension => 18014,
            CTokenError::TooManyExtensions => 18015,
            CTokenError::InvalidRootIndex => 18016,
            CTokenError::DataSizeExceeded => 18017,
            CTokenError::InvalidCompressionMode => 18018,
            CTokenError::CompressInsufficientFunds => 18019,
            CTokenError::SysvarAccessError => 18020,
            CTokenError::CompressedTokenAccountTlvUnimplemented => 18021,
            CTokenError::InputAccountsLamportsLengthMismatch => 18022,
            CTokenError::OutputAccountsLamportsLengthMismatch => 18023,
            CTokenError::InvalidTokenDataVersion => 18028,
            CTokenError::InstructionDataExpectedMintAuthority => 18024,
            CTokenError::ZeroCopyExpectedMintAuthority => 18025,
            CTokenError::InstructionDataExpectedFreezeAuthority => 18026,
            CTokenError::ZeroCopyExpectedFreezeAuthority => 18027,
            CTokenError::InvalidAuthorityType => 18029,
            CTokenError::ExpectedMintSignerAccount => 18030,
            CTokenError::InvalidTokenMetadataVersion => 18031,
            CTokenError::InvalidExtensionConfig => 18032,
            CTokenError::InstructionDataExpectedDelegate => 18033,
            CTokenError::ZeroCopyExpectedDelegate => 18034,
            CTokenError::TokenDataTlvUnimplemented => 18035,
            CTokenError::InvalidAccountState => 18036,
            CTokenError::BorshFailed => 18037,
            CTokenError::TooManyInputAccounts => 18038,
            CTokenError::TooManyAdditionalMetadata => 18039,
            CTokenError::DuplicateMetadataKey => 18040,
            CTokenError::TooManySeeds(_) => 18041,
            CTokenError::HasherError(e) => u32::from(e),
            CTokenError::ZeroCopyError(e) => u32::from(e),
            CTokenError::CompressedAccountError(e) => u32::from(e),
        }
    }
}

#[cfg(all(feature = "solana", not(feature = "anchor")))]
impl From<CTokenError> for solana_program_error::ProgramError {
    fn from(e: CTokenError) -> Self {
        solana_program_error::ProgramError::Custom(e.into())
    }
}

impl From<CTokenError> for pinocchio::program_error::ProgramError {
    fn from(e: CTokenError) -> Self {
        pinocchio::program_error::ProgramError::Custom(e.into())
    }
}

#[cfg(feature = "anchor")]
impl From<CTokenError> for anchor_lang::prelude::ProgramError {
    fn from(e: CTokenError) -> Self {
        anchor_lang::prelude::ProgramError::Custom(e.into())
    }
}

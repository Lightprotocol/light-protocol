use light_zero_copy::errors::ZeroCopyError;
use thiserror::Error;

#[derive(Debug, PartialEq, Error)]
pub enum TokenError {
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
    #[error("Unsupported TLV extension type - only CompressedOnly is currently implemented")]
    UnsupportedTlvExtensionType,
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

    #[error("write_top_up exceeds max_top_up from RentConfig")]
    WriteTopUpExceedsMaximum,

    #[error("Calculated top-up exceeds sender's max_top_up limit")]
    MaxTopUpExceeded,

    #[error("CMint account has invalid owner")]
    InvalidCMintOwner,

    #[error("CMint account is not initialized")]
    CMintNotInitialized,

    #[error("Failed to borrow CMint account data")]
    CMintBorrowFailed,

    #[error("Failed to deserialize CMint account data")]
    CMintDeserializationFailed,

    #[error("CompressedOnly tokens cannot have compressed outputs - must decompress only")]
    CompressedOnlyBlocksTransfer,

    #[error("Output TLV data count must match number of compressed outputs")]
    OutTlvOutputCountMismatch,

    #[error("in_lamports field is not yet implemented")]
    InLamportsUnimplemented,

    #[error("out_lamports field is not yet implemented")]
    OutLamportsUnimplemented,

    #[error("TLV extension length mismatch - exactly one extension required")]
    TlvExtensionLengthMismatch,

    #[error("InvalidAccountType")]
    InvalidAccountType,

    #[error("Duplicate compression_index found in input TLV data")]
    DuplicateCompressionIndex,

    #[error("Decompress destination Token is not a fresh account")]
    DecompressDestinationNotFresh,

    #[error("Token account missing required Compressible extension")]
    MissingCompressibleExtension,

    #[error("Decompress destination doesn't match source account")]
    DecompressDestinationMismatch,

    #[error("Token account mint does not match expected mint")]
    MintMismatch,

    #[error("Decompress has delegated_amount but no delegate pubkey provided")]
    DecompressDelegatedAmountWithoutDelegate,

    #[error(
        "Decompress has withheld_transfer_fee but destination lacks TransferFeeAccount extension"
    )]
    DecompressWithheldFeeWithoutExtension,

    #[error("Missing required payer account")]
    MissingPayer,

    #[error("Failed to borrow account data")]
    BorrowFailed,

    #[error("Token account has invalid owner")]
    InvalidTokenOwner,

    #[error("Decompress amount mismatch between compression instruction and input token data")]
    DecompressAmountMismatch,

    #[error("Compression index exceeds maximum allowed value")]
    CompressionIndexOutOfBounds,

    #[error("ATA derivation failed or mismatched for is_ata compressed token")]
    InvalidAtaDerivation,
}

impl From<TokenError> for u32 {
    fn from(e: TokenError) -> u32 {
        match e {
            TokenError::InvalidInstructionData => 18001,
            TokenError::InvalidAccountData => 18002,
            TokenError::ArithmeticOverflow => 18003,
            TokenError::HashComputationError => 18004,
            TokenError::InvalidExtensionData => 18005,
            TokenError::MissingMintAuthority => 18006,
            TokenError::MissingFreezeAuthority => 18007,
            TokenError::InvalidMetadataPointer => 18008,
            TokenError::InvalidTokenMetadata => 18009,
            TokenError::InsufficientSupply => 18010,
            TokenError::AccountFrozen => 18011,
            TokenError::InvalidProof => 18012,
            TokenError::AddressDerivationFailed => 18013,
            TokenError::UnsupportedExtension => 18014,
            TokenError::TooManyExtensions => 18015,
            TokenError::InvalidRootIndex => 18016,
            TokenError::DataSizeExceeded => 18017,
            TokenError::InvalidCompressionMode => 18018,
            TokenError::CompressInsufficientFunds => 18019,
            TokenError::SysvarAccessError => 18020,
            TokenError::CompressedTokenAccountTlvUnimplemented => 18021,
            TokenError::InputAccountsLamportsLengthMismatch => 18022,
            TokenError::OutputAccountsLamportsLengthMismatch => 18023,
            TokenError::InvalidTokenDataVersion => 18028,
            TokenError::InstructionDataExpectedMintAuthority => 18024,
            TokenError::ZeroCopyExpectedMintAuthority => 18025,
            TokenError::InstructionDataExpectedFreezeAuthority => 18026,
            TokenError::ZeroCopyExpectedFreezeAuthority => 18027,
            TokenError::InvalidAuthorityType => 18029,
            TokenError::ExpectedMintSignerAccount => 18030,
            TokenError::InvalidTokenMetadataVersion => 18031,
            TokenError::InvalidExtensionConfig => 18032,
            TokenError::InstructionDataExpectedDelegate => 18033,
            TokenError::ZeroCopyExpectedDelegate => 18034,
            TokenError::UnsupportedTlvExtensionType => 18035,
            TokenError::InvalidAccountState => 18036,
            TokenError::BorshFailed => 18037,
            TokenError::TooManyInputAccounts => 18038,
            TokenError::TooManyAdditionalMetadata => 18039,
            TokenError::DuplicateMetadataKey => 18040,
            TokenError::TooManySeeds(_) => 18041,
            TokenError::WriteTopUpExceedsMaximum => 18042,
            TokenError::MaxTopUpExceeded => 18043,
            TokenError::InvalidCMintOwner => 18044,
            TokenError::CMintNotInitialized => 18045,
            TokenError::CMintBorrowFailed => 18046,
            TokenError::CMintDeserializationFailed => 18047,
            TokenError::CompressedOnlyBlocksTransfer => 18048,
            TokenError::OutTlvOutputCountMismatch => 18049,
            TokenError::InLamportsUnimplemented => 18050,
            TokenError::OutLamportsUnimplemented => 18051,
            TokenError::TlvExtensionLengthMismatch => 18052,
            TokenError::InvalidAccountType => 18053,
            TokenError::DuplicateCompressionIndex => 18054,
            TokenError::DecompressDestinationNotFresh => 18055,
            TokenError::MissingCompressibleExtension => 18056,
            TokenError::DecompressDestinationMismatch => 18057,
            TokenError::MintMismatch => 18058,
            TokenError::DecompressDelegatedAmountWithoutDelegate => 18059,
            TokenError::DecompressWithheldFeeWithoutExtension => 18060,
            TokenError::MissingPayer => 18061,
            TokenError::BorrowFailed => 18062,
            TokenError::InvalidTokenOwner => 18063,
            TokenError::DecompressAmountMismatch => 18064,
            TokenError::CompressionIndexOutOfBounds => 18065,
            TokenError::InvalidAtaDerivation => 18066,
            TokenError::HasherError(e) => u32::from(e),
            TokenError::ZeroCopyError(e) => u32::from(e),
            TokenError::CompressedAccountError(e) => u32::from(e),
        }
    }
}

#[cfg(all(feature = "solana", not(feature = "anchor")))]
impl From<TokenError> for solana_program_error::ProgramError {
    fn from(e: TokenError) -> Self {
        solana_program_error::ProgramError::Custom(e.into())
    }
}

impl From<TokenError> for pinocchio::program_error::ProgramError {
    fn from(e: TokenError) -> Self {
        pinocchio::program_error::ProgramError::Custom(e.into())
    }
}

#[cfg(feature = "anchor")]
impl From<TokenError> for anchor_lang::prelude::ProgramError {
    fn from(e: TokenError) -> Self {
        anchor_lang::prelude::ProgramError::Custom(e.into())
    }
}

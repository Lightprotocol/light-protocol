use anchor_compressed_token::ErrorCode;
use pinocchio::program_error::ProgramError;

fn main() {
    // All ProgramError variants - these use a special encoding where the value is shifted left 32 bits
    // The actual u32 value shown in transaction logs is the upper 32 bits
    let program_errors = vec![
        ("InvalidArgument", ProgramError::InvalidArgument, 2u32),
        (
            "InvalidInstructionData",
            ProgramError::InvalidInstructionData,
            3u32,
        ),
        ("InvalidAccountData", ProgramError::InvalidAccountData, 4u32),
        (
            "AccountDataTooSmall",
            ProgramError::AccountDataTooSmall,
            5u32,
        ),
        ("InsufficientFunds", ProgramError::InsufficientFunds, 6u32),
        ("IncorrectProgramId", ProgramError::IncorrectProgramId, 7u32),
        (
            "MissingRequiredSignature",
            ProgramError::MissingRequiredSignature,
            8u32,
        ),
        (
            "AccountAlreadyInitialized",
            ProgramError::AccountAlreadyInitialized,
            9u32,
        ),
        (
            "UninitializedAccount",
            ProgramError::UninitializedAccount,
            10u32,
        ),
        (
            "NotEnoughAccountKeys",
            ProgramError::NotEnoughAccountKeys,
            11u32,
        ),
        (
            "AccountBorrowFailed",
            ProgramError::AccountBorrowFailed,
            12u32,
        ),
        (
            "MaxSeedLengthExceeded",
            ProgramError::MaxSeedLengthExceeded,
            13u32,
        ),
        ("InvalidSeeds", ProgramError::InvalidSeeds, 14u32),
        ("BorshIoError", ProgramError::BorshIoError, 15u32),
        (
            "AccountNotRentExempt",
            ProgramError::AccountNotRentExempt,
            16u32,
        ),
        ("UnsupportedSysvar", ProgramError::UnsupportedSysvar, 17u32),
        ("IllegalOwner", ProgramError::IllegalOwner, 18u32),
        (
            "MaxAccountsDataAllocationsExceeded",
            ProgramError::MaxAccountsDataAllocationsExceeded,
            19u32,
        ),
        ("InvalidRealloc", ProgramError::InvalidRealloc, 20u32),
        (
            "MaxInstructionTraceLengthExceeded",
            ProgramError::MaxInstructionTraceLengthExceeded,
            21u32,
        ),
        (
            "BuiltinProgramsMustConsumeComputeUnits",
            ProgramError::BuiltinProgramsMustConsumeComputeUnits,
            22u32,
        ),
        (
            "InvalidAccountOwner",
            ProgramError::InvalidAccountOwner,
            23u32,
        ),
        (
            "ArithmeticOverflow",
            ProgramError::ArithmeticOverflow,
            24u32,
        ),
        ("Immutable", ProgramError::Immutable, 25u32),
        (
            "IncorrectAuthority",
            ProgramError::IncorrectAuthority,
            26u32,
        ),
    ];

    // All ErrorCode variants from anchor_compressed_token
    let error_codes = vec![
        (
            "PublicKeyAmountMissmatch",
            ErrorCode::PublicKeyAmountMissmatch,
        ),
        ("ComputeInputSumFailed", ErrorCode::ComputeInputSumFailed),
        ("ComputeOutputSumFailed", ErrorCode::ComputeOutputSumFailed),
        (
            "ComputeCompressSumFailed",
            ErrorCode::ComputeCompressSumFailed,
        ),
        (
            "ComputeDecompressSumFailed",
            ErrorCode::ComputeDecompressSumFailed,
        ),
        ("SumCheckFailed", ErrorCode::SumCheckFailed),
        (
            "DecompressRecipientUndefinedForDecompress",
            ErrorCode::DecompressRecipientUndefinedForDecompress,
        ),
        (
            "CompressedPdaUndefinedForDecompress",
            ErrorCode::CompressedPdaUndefinedForDecompress,
        ),
        (
            "DeCompressAmountUndefinedForDecompress",
            ErrorCode::DeCompressAmountUndefinedForDecompress,
        ),
        (
            "CompressedPdaUndefinedForCompress",
            ErrorCode::CompressedPdaUndefinedForCompress,
        ),
        (
            "DeCompressAmountUndefinedForCompress",
            ErrorCode::DeCompressAmountUndefinedForCompress,
        ),
        (
            "DelegateSignerCheckFailed",
            ErrorCode::DelegateSignerCheckFailed,
        ),
        ("MintTooLarge", ErrorCode::MintTooLarge),
        ("SplTokenSupplyMismatch", ErrorCode::SplTokenSupplyMismatch),
        ("HeapMemoryCheckFailed", ErrorCode::HeapMemoryCheckFailed),
        ("InstructionNotCallable", ErrorCode::InstructionNotCallable),
        ("ArithmeticUnderflow", ErrorCode::ArithmeticUnderflow),
        ("HashToFieldError", ErrorCode::HashToFieldError),
        ("InvalidAuthorityMint", ErrorCode::InvalidAuthorityMint),
        ("InvalidFreezeAuthority", ErrorCode::InvalidFreezeAuthority),
        ("InvalidDelegateIndex", ErrorCode::InvalidDelegateIndex),
        ("TokenPoolPdaUndefined", ErrorCode::TokenPoolPdaUndefined),
        ("IsTokenPoolPda", ErrorCode::IsTokenPoolPda),
        ("InvalidTokenPoolPda", ErrorCode::InvalidTokenPoolPda),
        (
            "NoInputTokenAccountsProvided",
            ErrorCode::NoInputTokenAccountsProvided,
        ),
        ("NoInputsProvided", ErrorCode::NoInputsProvided),
        (
            "MintHasNoFreezeAuthority",
            ErrorCode::MintHasNoFreezeAuthority,
        ),
        (
            "MintWithInvalidExtension",
            ErrorCode::MintWithInvalidExtension,
        ),
        (
            "InsufficientTokenAccountBalance",
            ErrorCode::InsufficientTokenAccountBalance,
        ),
        ("InvalidTokenPoolBump", ErrorCode::InvalidTokenPoolBump),
        ("FailedToDecompress", ErrorCode::FailedToDecompress),
        (
            "FailedToBurnSplTokensFromTokenPool",
            ErrorCode::FailedToBurnSplTokensFromTokenPool,
        ),
        ("NoMatchingBumpFound", ErrorCode::NoMatchingBumpFound),
        ("NoAmount", ErrorCode::NoAmount),
        (
            "AmountsAndAmountProvided",
            ErrorCode::AmountsAndAmountProvided,
        ),
        ("CpiContextSetNotUsable", ErrorCode::CpiContextSetNotUsable),
        ("MintIsNone", ErrorCode::MintIsNone),
        ("InvalidMintPda", ErrorCode::InvalidMintPda),
        ("InputsOutOfOrder", ErrorCode::InputsOutOfOrder),
        ("TooManyMints", ErrorCode::TooManyMints),
        ("InvalidExtensionType", ErrorCode::InvalidExtensionType),
        (
            "InstructionDataExpectedDelegate",
            ErrorCode::InstructionDataExpectedDelegate,
        ),
        (
            "ZeroCopyExpectedDelegate",
            ErrorCode::ZeroCopyExpectedDelegate,
        ),
        (
            "TokenDataTlvUnimplemented",
            ErrorCode::TokenDataTlvUnimplemented,
        ),
        (
            "MintActionNoActionsProvided",
            ErrorCode::MintActionNoActionsProvided,
        ),
        (
            "MintActionMissingSplMintSigner",
            ErrorCode::MintActionMissingSplMintSigner,
        ),
        (
            "MintActionMissingSystemAccount",
            ErrorCode::MintActionMissingSystemAccount,
        ),
        (
            "MintActionInvalidMintBump",
            ErrorCode::MintActionInvalidMintBump,
        ),
        (
            "MintActionMissingMintAccount",
            ErrorCode::MintActionMissingMintAccount,
        ),
        (
            "MintActionMissingTokenPoolAccount",
            ErrorCode::MintActionMissingTokenPoolAccount,
        ),
        (
            "MintActionMissingTokenProgram",
            ErrorCode::MintActionMissingTokenProgram,
        ),
        ("MintAccountMismatch", ErrorCode::MintAccountMismatch),
        (
            "InvalidCompressAuthority",
            ErrorCode::InvalidCompressAuthority,
        ),
        (
            "MintActionInvalidQueueIndex",
            ErrorCode::MintActionInvalidQueueIndex,
        ),
        (
            "MintActionSerializationFailed",
            ErrorCode::MintActionSerializationFailed,
        ),
        ("MintActionProofMissing", ErrorCode::MintActionProofMissing),
        (
            "MintActionUnsupportedActionType",
            ErrorCode::MintActionUnsupportedActionType,
        ),
        (
            "MintActionMetadataNotDecompressed",
            ErrorCode::MintActionMetadataNotDecompressed,
        ),
        (
            "MintActionMissingMetadataExtension",
            ErrorCode::MintActionMissingMetadataExtension,
        ),
        (
            "MintActionInvalidExtensionIndex",
            ErrorCode::MintActionInvalidExtensionIndex,
        ),
        (
            "MintActionInvalidMetadataValue",
            ErrorCode::MintActionInvalidMetadataValue,
        ),
        (
            "MintActionInvalidMetadataKey",
            ErrorCode::MintActionInvalidMetadataKey,
        ),
        (
            "MintActionInvalidExtensionType",
            ErrorCode::MintActionInvalidExtensionType,
        ),
        (
            "MintActionMetadataKeyNotFound",
            ErrorCode::MintActionMetadataKeyNotFound,
        ),
        (
            "MintActionMissingExecutingAccounts",
            ErrorCode::MintActionMissingExecutingAccounts,
        ),
        (
            "MintActionInvalidMintAuthority",
            ErrorCode::MintActionInvalidMintAuthority,
        ),
        (
            "MintActionInvalidMintPda",
            ErrorCode::MintActionInvalidMintPda,
        ),
        (
            "MintActionMissingSystemAccountsForQueue",
            ErrorCode::MintActionMissingSystemAccountsForQueue,
        ),
        (
            "MintActionOutputSerializationFailed",
            ErrorCode::MintActionOutputSerializationFailed,
        ),
        (
            "MintActionAmountTooLarge",
            ErrorCode::MintActionAmountTooLarge,
        ),
        (
            "MintActionInvalidInitialSupply",
            ErrorCode::MintActionInvalidInitialSupply,
        ),
        (
            "MintActionUnsupportedVersion",
            ErrorCode::MintActionUnsupportedVersion,
        ),
        (
            "MintActionInvalidCompressionState",
            ErrorCode::MintActionInvalidCompressionState,
        ),
        (
            "MintActionUnsupportedOperation",
            ErrorCode::MintActionUnsupportedOperation,
        ),
        ("NonNativeHasBalance", ErrorCode::NonNativeHasBalance),
        ("OwnerMismatch", ErrorCode::OwnerMismatch),
        ("AccountFrozen", ErrorCode::AccountFrozen),
        (
            "InsufficientAccountSize",
            ErrorCode::InsufficientAccountSize,
        ),
        ("AlreadyInitialized", ErrorCode::AlreadyInitialized),
        (
            "InvalidExtensionInstructionData",
            ErrorCode::InvalidExtensionInstructionData,
        ),
        (
            "MintActionLamportsAmountTooLarge",
            ErrorCode::MintActionLamportsAmountTooLarge,
        ),
        ("InvalidTokenProgram", ErrorCode::InvalidTokenProgram),
        (
            "Transfer2CpiContextWriteInvalidAccess",
            ErrorCode::Transfer2CpiContextWriteInvalidAccess,
        ),
        (
            "Transfer2CpiContextWriteWithSolPool",
            ErrorCode::Transfer2CpiContextWriteWithSolPool,
        ),
        (
            "Transfer2InvalidChangeAccountData",
            ErrorCode::Transfer2InvalidChangeAccountData,
        ),
        ("CpiContextExpected", ErrorCode::CpiContextExpected),
        (
            "CpiAccountsSliceOutOfBounds",
            ErrorCode::CpiAccountsSliceOutOfBounds,
        ),
        (
            "CompressAndCloseDestinationMissing",
            ErrorCode::CompressAndCloseDestinationMissing,
        ),
        (
            "CompressAndCloseAuthorityMissing",
            ErrorCode::CompressAndCloseAuthorityMissing,
        ),
        (
            "CompressAndCloseInvalidOwner",
            ErrorCode::CompressAndCloseInvalidOwner,
        ),
        (
            "CompressAndCloseAmountMismatch",
            ErrorCode::CompressAndCloseAmountMismatch,
        ),
        (
            "CompressAndCloseBalanceMismatch",
            ErrorCode::CompressAndCloseBalanceMismatch,
        ),
        (
            "CompressAndCloseDelegateNotAllowed",
            ErrorCode::CompressAndCloseDelegateNotAllowed,
        ),
        (
            "CompressAndCloseInvalidVersion",
            ErrorCode::CompressAndCloseInvalidVersion,
        ),
        ("InvalidAddressTree", ErrorCode::InvalidAddressTree),
        (
            "TooManyCompressionTransfers",
            ErrorCode::TooManyCompressionTransfers,
        ),
    ];

    println!("ProgramError variants (actual error code shown in logs):");
    println!("=========================================================");
    for (name, _error, actual_code) in program_errors {
        println!("ProgramError::{:<45} -> error code: {}", name, actual_code);
    }

    println!("\nErrorCode variants (as Custom):");
    println!("=================================");
    for (name, error_code) in error_codes {
        let error_u32: u32 = error_code.into();
        let error_u64 = u64::from(ProgramError::Custom(error_u32));
        println!(
            "ErrorCode::{:<45} -> u64: {} (Custom u32: {})",
            name, error_u64, error_u32
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_error_codes() {
        main();
    }
}

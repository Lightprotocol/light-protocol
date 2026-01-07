use anchor_compressed_token::ErrorCode;
use pinocchio_token_program::error::TokenError;

/// Convert generic pinocchio errors to anchor ProgramError with +6000 offset.
/// Use this for system program operations, data access, and non-token operations.
pub fn convert_program_error(
    pinocchio_program_error: pinocchio::program_error::ProgramError,
) -> anchor_lang::prelude::ProgramError {
    anchor_lang::prelude::ProgramError::Custom(u64::from(pinocchio_program_error) as u32 + 6000)
}

/// Convert TokenError directly to anchor ProgramError.
/// Use for functions returning TokenError (e.g., unpack_amount_and_decimals).
pub fn convert_token_error(e: TokenError) -> anchor_lang::prelude::ProgramError {
    convert_spl_token_error_code(e as u32)
}

/// Convert pinocchio token processor errors to our custom ErrorCode.
/// Maps SPL Token error codes (0-18) to our enum variants for consistent error reporting.
///
/// IMPORTANT: Only use this for pinocchio_token_program processor calls.
/// For system program and other operations, use `convert_program_error` instead.
pub fn convert_pinocchio_token_error(
    pinocchio_error: pinocchio::program_error::ProgramError,
) -> anchor_lang::prelude::ProgramError {
    convert_spl_token_error_code(u64::from(pinocchio_error) as u32)
}

/// Internal: Map SPL Token error code (0-18) to ErrorCode.
fn convert_spl_token_error_code(code: u32) -> anchor_lang::prelude::ProgramError {
    let error_code = match code {
        0 => ErrorCode::NotRentExempt,
        1 => ErrorCode::InsufficientFunds,
        2 => ErrorCode::InvalidMint,
        3 => ErrorCode::MintMismatch,
        4 => ErrorCode::OwnerMismatch,
        5 => ErrorCode::FixedSupply,
        6 => ErrorCode::AlreadyInUse,
        7 => ErrorCode::InvalidNumberOfProvidedSigners,
        8 => ErrorCode::InvalidNumberOfRequiredSigners,
        9 => ErrorCode::UninitializedState,
        10 => ErrorCode::NativeNotSupported,
        11 => ErrorCode::NonNativeHasBalance,
        12 => ErrorCode::InvalidInstruction,
        13 => ErrorCode::InvalidState,
        14 => ErrorCode::Overflow,
        15 => ErrorCode::AuthorityTypeNotSupported,
        16 => ErrorCode::MintHasNoFreezeAuthority,
        17 => ErrorCode::AccountFrozen,
        18 => ErrorCode::MintDecimalsMismatch,
        // Pass through unknown/higher codes with standard +6900 offset
        _ => return anchor_lang::prelude::ProgramError::Custom(code + 6900),
    };
    error_code.into()
}

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{account_info::AccountInfo, program_error::ProgramError};

/// Instruction data for CTokenMintTo operations
#[derive(BorshSerialize, BorshDeserialize)]
pub struct MintToData {
    pub amount: u64,
}

/// Handler for minting to CToken (invoke)
///
/// Note: This operation (minting to a regular token account) is no longer part of the public SDK API.
/// Use the compressed token minting API instead.
pub fn process_ctoken_mint_to_invoke(
    _accounts: &[AccountInfo],
    _amount: u64,
) -> Result<(), ProgramError> {
    // This operation is deprecated - simple token minting is no longer supported
    Err(ProgramError::Custom(999))
}

/// Handler for minting to CToken with PDA authority (invoke_signed)
///
/// Note: This operation (minting to a regular token account) is no longer part of the public SDK API.
/// Use the compressed token minting API instead.
pub fn process_ctoken_mint_to_invoke_signed(
    _accounts: &[AccountInfo],
    _amount: u64,
) -> Result<(), ProgramError> {
    // This operation is deprecated - simple token minting is no longer supported
    Err(ProgramError::Custom(999))
}

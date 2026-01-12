use anchor_lang::solana_program::program_error::ProgramError;
use pinocchio::account_info::AccountInfo;
use pinocchio_token_program::processor::{
    freeze_account::process_freeze_account, thaw_account::process_thaw_account,
};

use crate::shared::{convert_pinocchio_token_error, owner_validation::check_token_program_owner};

/// Process CToken freeze account instruction.
/// Validates mint ownership before calling pinocchio-token-program.
#[inline(always)]
pub fn process_ctoken_freeze_account(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    // accounts[1] is the mint
    let mint_info = accounts.get(1).ok_or(ProgramError::NotEnoughAccountKeys)?;
    check_token_program_owner(mint_info)?;
    process_freeze_account(accounts).map_err(convert_pinocchio_token_error)
}

/// Process CToken thaw account instruction.
/// Validates mint ownership before calling pinocchio-token-program.
#[inline(always)]
pub fn process_ctoken_thaw_account(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    // accounts[1] is the mint
    let mint_info = accounts.get(1).ok_or(ProgramError::NotEnoughAccountKeys)?;
    check_token_program_owner(mint_info)?;
    process_thaw_account(accounts).map_err(convert_pinocchio_token_error)
}

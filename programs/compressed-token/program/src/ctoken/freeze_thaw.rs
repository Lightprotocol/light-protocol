use anchor_lang::solana_program::program_error::ProgramError;
use pinocchio::AccountView as AccountInfo;
use pinocchio_token_program::processor::{
    freeze_account::process_freeze_account, thaw_account::process_thaw_account,
};

use super::burn::convert_v9_result;
use crate::shared::owner_validation::check_token_program_owner;

/// Process CToken freeze account instruction.
/// Validates mint ownership before calling pinocchio-token-program.
#[inline(always)]
pub fn process_ctoken_freeze_account(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    // accounts[1] is the mint
    let mint_info = accounts.get(1).ok_or(ProgramError::NotEnoughAccountKeys)?;
    check_token_program_owner(mint_info)?;
    // SAFETY: pinocchio 0.9 AccountInfo and 0.10 AccountView have the same memory layout.
    convert_v9_result(process_freeze_account(unsafe {
        core::mem::transmute(accounts)
    }))
}

/// Process CToken thaw account instruction.
/// Validates mint ownership before calling pinocchio-token-program.
#[inline(always)]
pub fn process_ctoken_thaw_account(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    // accounts[1] is the mint
    let mint_info = accounts.get(1).ok_or(ProgramError::NotEnoughAccountKeys)?;
    check_token_program_owner(mint_info)?;
    // SAFETY: pinocchio 0.9 AccountInfo and 0.10 AccountView have the same memory layout.
    convert_v9_result(process_thaw_account(unsafe {
        core::mem::transmute(accounts)
    }))
}

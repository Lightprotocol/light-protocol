use anchor_lang::solana_program::program_error::ProgramError;
use pinocchio::account_info::AccountInfo;
use pinocchio_token_program::processor::{
    freeze_account::process_freeze_account, thaw_account::process_thaw_account,
};

/// Process CToken freeze account instruction.
/// Direct passthrough to pinocchio-token-program - no extension processing needed.
#[inline(always)]
pub fn process_ctoken_freeze_account(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    process_freeze_account(accounts).map_err(|e| ProgramError::Custom(u64::from(e) as u32))
}

/// Process CToken thaw account instruction.
/// Direct passthrough to pinocchio-token-program - no extension processing needed.
#[inline(always)]
pub fn process_ctoken_thaw_account(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    process_thaw_account(accounts).map_err(|e| ProgramError::Custom(u64::from(e) as u32))
}

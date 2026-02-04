//! Freeze CPI for Light Token operations.

use pinocchio::{
    account_info::AccountInfo,
    cpi::{slice_invoke, slice_invoke_signed},
    instruction::{AccountMeta, Instruction, Signer},
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::constants::LIGHT_TOKEN_PROGRAM_ID;

/// Freeze ctoken via CPI.
///
/// # Example
///
/// ```rust,ignore
/// use light_token_pinocchio::instruction::FreezeCpi;
///
/// FreezeCpi {
///     token_account: &ctx.accounts.token_account,
///     mint: &ctx.accounts.mint,
///     freeze_authority: &ctx.accounts.freeze_authority,
/// }
/// .invoke()?;
/// ```
pub struct FreezeCpi<'info> {
    pub token_account: &'info AccountInfo,
    pub mint: &'info AccountInfo,
    pub freeze_authority: &'info AccountInfo,
}

impl<'info> FreezeCpi<'info> {
    pub fn invoke(self) -> Result<(), ProgramError> {
        self.invoke_signed(&[])
    }

    pub fn invoke_signed(self, signers: &[Signer]) -> Result<(), ProgramError> {
        // Build instruction data: discriminator(1) only
        let data = [10u8]; // Freeze discriminator

        let program_id = Pubkey::from(LIGHT_TOKEN_PROGRAM_ID);

        let account_metas = [
            AccountMeta::writable(self.token_account.key()),
            AccountMeta::readonly(self.mint.key()),
            AccountMeta::readonly_signer(self.freeze_authority.key()),
        ];

        let instruction = Instruction {
            program_id: &program_id,
            accounts: &account_metas,
            data: &data,
        };

        let account_infos = [self.token_account, self.mint, self.freeze_authority];

        if signers.is_empty() {
            slice_invoke(&instruction, &account_infos)
        } else {
            slice_invoke_signed(&instruction, &account_infos, signers)
        }
    }
}

//! Revoke CPI for Light Token operations.

use pinocchio::{
    account_info::AccountInfo,
    cpi::{slice_invoke, slice_invoke_signed},
    instruction::{AccountMeta, Instruction, Signer},
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::constants::LIGHT_TOKEN_PROGRAM_ID;

/// Revoke ctoken via CPI.
///
/// # Example
///
/// ```rust,ignore
/// use light_token_pinocchio::instruction::RevokeCpi;
///
/// RevokeCpi {
///     token_account: &ctx.accounts.token_account,
///     owner: &ctx.accounts.owner,
///     system_program: &ctx.accounts.system_program,
///     fee_payer: &ctx.accounts.fee_payer,
/// }
/// .invoke()?;
/// ```
pub struct RevokeCpi<'info> {
    pub token_account: &'info AccountInfo,
    pub owner: &'info AccountInfo,
    pub system_program: &'info AccountInfo,
    /// Fee payer for compressible rent top-ups (writable signer)
    pub fee_payer: &'info AccountInfo,
}

impl<'info> RevokeCpi<'info> {
    pub fn invoke(self) -> Result<(), ProgramError> {
        self.invoke_signed(&[])
    }

    pub fn invoke_signed(self, signers: &[Signer]) -> Result<(), ProgramError> {
        // Build instruction data: discriminator(1) only
        let data = [5u8]; // Revoke discriminator

        let program_id = Pubkey::from(LIGHT_TOKEN_PROGRAM_ID);

        let account_metas = [
            AccountMeta::writable(self.token_account.key()),
            AccountMeta::readonly_signer(self.owner.key()),
            AccountMeta::readonly(self.system_program.key()),
            AccountMeta::writable_signer(self.fee_payer.key()),
        ];

        let instruction = Instruction {
            program_id: &program_id,
            accounts: &account_metas,
            data: &data,
        };

        let account_infos = [
            self.token_account,
            self.owner,
            self.system_program,
            self.fee_payer,
        ];

        if signers.is_empty() {
            slice_invoke(&instruction, &account_infos)
        } else {
            slice_invoke_signed(&instruction, &account_infos, signers)
        }
    }
}

//! Approve CPI for Light Token operations.

use pinocchio::{
    account_info::AccountInfo,
    cpi::{slice_invoke, slice_invoke_signed},
    instruction::{AccountMeta, Instruction, Signer},
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::constants::LIGHT_TOKEN_PROGRAM_ID;

/// Approve ctoken via CPI.
///
/// # Example
///
/// ```rust,ignore
/// use light_token_pinocchio::instruction::ApproveCpi;
///
/// ApproveCpi {
///     token_account: &ctx.accounts.token_account,
///     delegate: &ctx.accounts.delegate,
///     owner: &ctx.accounts.owner,
///     system_program: &ctx.accounts.system_program,
///     amount: 100,
/// }
/// .invoke()?;
/// ```
pub struct ApproveCpi<'info> {
    pub token_account: &'info AccountInfo,
    pub delegate: &'info AccountInfo,
    pub owner: &'info AccountInfo,
    pub system_program: &'info AccountInfo,
    pub amount: u64,
}

impl<'info> ApproveCpi<'info> {
    pub fn invoke(self) -> Result<(), ProgramError> {
        self.invoke_signed(&[])
    }

    pub fn invoke_signed(self, signers: &[Signer]) -> Result<(), ProgramError> {
        // Build instruction data: discriminator(1) + amount(8)
        let mut data = [0u8; 9];
        data[0] = 4u8; // Approve discriminator
        data[1..9].copy_from_slice(&self.amount.to_le_bytes());

        let program_id = Pubkey::from(LIGHT_TOKEN_PROGRAM_ID);

        let account_metas = [
            AccountMeta::writable(self.token_account.key()),
            AccountMeta::readonly(self.delegate.key()),
            AccountMeta::writable_signer(self.owner.key()),
            AccountMeta::readonly(self.system_program.key()),
        ];

        let instruction = Instruction {
            program_id: &program_id,
            accounts: &account_metas,
            data: &data,
        };

        let account_infos = [
            self.token_account,
            self.delegate,
            self.owner,
            self.system_program,
        ];

        if signers.is_empty() {
            slice_invoke(&instruction, &account_infos)
        } else {
            slice_invoke_signed(&instruction, &account_infos, signers)
        }
    }
}

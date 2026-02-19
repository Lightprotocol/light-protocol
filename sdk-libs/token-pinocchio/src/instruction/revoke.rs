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
///     max_top_up: None,
/// }
/// .invoke()?;
/// ```
pub struct RevokeCpi<'info> {
    pub token_account: &'info AccountInfo,
    pub owner: &'info AccountInfo,
    pub system_program: &'info AccountInfo,
    pub max_top_up: Option<u16>,
}

impl<'info> RevokeCpi<'info> {
    pub fn invoke(self) -> Result<(), ProgramError> {
        self.invoke_signed(&[])
    }

    pub fn invoke_signed(self, signers: &[Signer]) -> Result<(), ProgramError> {
        // Build instruction data: discriminator(1) + optional max_top_up(2)
        let mut data = [0u8; 3];
        data[0] = 5u8; // Revoke discriminator
        let mut data_len = 1;
        if let Some(max_top_up) = self.max_top_up {
            data[1..3].copy_from_slice(&max_top_up.to_le_bytes());
            data_len = 3;
        }

        let owner_meta = if self.max_top_up.is_some() {
            AccountMeta::writable_signer(self.owner.key())
        } else {
            AccountMeta::readonly_signer(self.owner.key())
        };

        let program_id = Pubkey::from(LIGHT_TOKEN_PROGRAM_ID);

        let account_metas = [
            AccountMeta::writable(self.token_account.key()),
            owner_meta,
            AccountMeta::readonly(self.system_program.key()),
        ];

        let instruction = Instruction {
            program_id: &program_id,
            accounts: &account_metas,
            data: &data[..data_len],
        };

        let account_infos = [self.token_account, self.owner, self.system_program];

        if signers.is_empty() {
            slice_invoke(&instruction, &account_infos)
        } else {
            slice_invoke_signed(&instruction, &account_infos, signers)
        }
    }
}

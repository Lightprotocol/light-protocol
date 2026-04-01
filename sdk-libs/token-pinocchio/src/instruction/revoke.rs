//! Revoke CPI for Light Token operations.

use pinocchio::{
    AccountView as AccountInfo,
    cpi::{invoke_with_slice, invoke_signed_with_slice, Signer},
    instruction::{InstructionAccount, InstructionView},
    error::ProgramError,
    address::Address,
};

use crate::constants::LIGHT_TOKEN_PROGRAM_ID;

/// Revoke light-token via CPI.
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
    /// Fee payer for rent top-ups.
    pub fee_payer: &'info AccountInfo,
}

impl<'info> RevokeCpi<'info> {
    pub fn invoke(self) -> Result<(), ProgramError> {
        self.invoke_signed(&[])
    }

    pub fn invoke_signed(self, signers: &[Signer]) -> Result<(), ProgramError> {
        // Build instruction data: discriminator(1) only
        let data = [5u8]; // Revoke discriminator

        let program_id = Address::from(LIGHT_TOKEN_PROGRAM_ID);

        let account_metas = [
            InstructionAccount::writable(self.token_account.address()),
            InstructionAccount::readonly_signer(self.owner.address()),
            InstructionAccount::readonly(self.system_program.address()),
            InstructionAccount::writable_signer(self.fee_payer.address()),
        ];

        let instruction = InstructionView {
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
            invoke_with_slice(&instruction, &account_infos)
        } else {
            invoke_signed_with_slice(&instruction, &account_infos, signers)
        }
    }
}

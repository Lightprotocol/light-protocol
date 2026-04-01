//! Approve CPI for Light Token operations.

use pinocchio::{
    AccountView as AccountInfo,
    cpi::{invoke_with_slice, invoke_signed_with_slice, Signer},
    instruction::{InstructionAccount, InstructionView},
    error::ProgramError,
    address::Address,
};

use crate::constants::LIGHT_TOKEN_PROGRAM_ID;

/// Approve light-token via CPI.
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
///     fee_payer: &ctx.accounts.fee_payer,
/// }
/// .invoke()?;
/// ```
pub struct ApproveCpi<'info> {
    pub token_account: &'info AccountInfo,
    pub delegate: &'info AccountInfo,
    pub owner: &'info AccountInfo,
    pub system_program: &'info AccountInfo,
    pub amount: u64,
    /// Fee payer for rent top-ups.
    pub fee_payer: &'info AccountInfo,
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

        let program_id = Address::from(LIGHT_TOKEN_PROGRAM_ID);

        let account_metas = [
            InstructionAccount::writable(self.token_account.address()),
            InstructionAccount::readonly(self.delegate.address()),
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
            self.delegate,
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

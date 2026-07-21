//! MintToChecked CPI for Light Token operations.

use pinocchio::{
    address::Address,
    cpi::{invoke_signed_with_slice, invoke_with_slice, Signer},
    error::ProgramError,
    instruction::{InstructionAccount, InstructionView},
    AccountView as AccountInfo,
};

use crate::constants::LIGHT_TOKEN_PROGRAM_ID;

/// Mint to light-token account checked via CPI.
///
/// # Example
///
/// ```rust,ignore
/// use light_token_pinocchio::instruction::MintToCheckedCpi;
///
/// MintToCheckedCpi {
///     mint: &ctx.accounts.mint,
///     destination: &ctx.accounts.destination,
///     amount: 100,
///     decimals: 9,
///     authority: &ctx.accounts.authority,
///     system_program: &ctx.accounts.system_program,
///     fee_payer: &ctx.accounts.fee_payer,
/// }
/// .invoke()?;
/// ```
pub struct MintToCheckedCpi<'info> {
    pub mint: &'info AccountInfo,
    pub destination: &'info AccountInfo,
    pub amount: u64,
    pub decimals: u8,
    pub authority: &'info AccountInfo,
    pub system_program: &'info AccountInfo,
    /// Fee payer for rent top-ups.
    pub fee_payer: &'info AccountInfo,
}

impl<'info> MintToCheckedCpi<'info> {
    pub fn invoke(self) -> Result<(), ProgramError> {
        self.invoke_signed(&[])
    }

    pub fn invoke_signed(self, signers: &[Signer]) -> Result<(), ProgramError> {
        let mut data = [0u8; 10]; // discriminator(1) + amount(8) + decimals(1)
        data[0] = 14u8;
        data[1..9].copy_from_slice(&self.amount.to_le_bytes());
        data[9] = self.decimals;

        let program_id = Address::from(LIGHT_TOKEN_PROGRAM_ID);

        let account_metas = [
            InstructionAccount::writable(self.mint.address()),
            InstructionAccount::writable(self.destination.address()),
            InstructionAccount::readonly_signer(self.authority.address()),
            InstructionAccount::readonly(self.system_program.address()),
            InstructionAccount::writable_signer(self.fee_payer.address()),
        ];

        let instruction = InstructionView {
            program_id: &program_id,
            accounts: &account_metas,
            data: &data,
        };

        let account_infos = [
            self.mint,
            self.destination,
            self.authority,
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

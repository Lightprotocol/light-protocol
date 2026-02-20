//! TransferChecked CPI for Light Token operations.

use pinocchio::{
    account_info::AccountInfo,
    cpi::{slice_invoke, slice_invoke_signed},
    instruction::{AccountMeta, Instruction, Signer},
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::constants::LIGHT_TOKEN_PROGRAM_ID;

const SYSTEM_PROGRAM_ID: Pubkey = [0u8; 32];

/// Transfer ctoken checked via CPI.
///
/// # Example
///
/// ```rust,ignore
/// use light_token_pinocchio::instruction::TransferCheckedCpi;
///
/// TransferCheckedCpi {
///     source: &ctx.accounts.source,
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
pub struct TransferCheckedCpi<'info> {
    pub source: &'info AccountInfo,
    pub mint: &'info AccountInfo,
    pub destination: &'info AccountInfo,
    pub amount: u64,
    pub decimals: u8,
    pub authority: &'info AccountInfo,
    pub system_program: &'info AccountInfo,
    /// Fee payer for rent top-ups (writable signer). Authority stays readonly.
    pub fee_payer: &'info AccountInfo,
}

impl<'info> TransferCheckedCpi<'info> {
    pub fn invoke(self) -> Result<(), ProgramError> {
        self.invoke_signed(&[])
    }

    pub fn invoke_signed(self, signers: &[Signer]) -> Result<(), ProgramError> {
        let mut data = [0u8; 10]; // discriminator(1) + amount(8) + decimals(1)
        data[0] = 12u8; // TransferChecked discriminator
        data[1..9].copy_from_slice(&self.amount.to_le_bytes());
        data[9] = self.decimals;

        let program_id = Pubkey::from(LIGHT_TOKEN_PROGRAM_ID);

        let account_metas = [
            AccountMeta::writable(self.source.key()),
            AccountMeta::readonly(self.mint.key()),
            AccountMeta::writable(self.destination.key()),
            AccountMeta::readonly_signer(self.authority.key()),
            AccountMeta::readonly(&SYSTEM_PROGRAM_ID),
            AccountMeta::writable_signer(self.fee_payer.key()),
        ];

        let instruction = Instruction {
            program_id: &program_id,
            accounts: &account_metas,
            data: &data,
        };

        let account_infos = [
            self.source,
            self.mint,
            self.destination,
            self.authority,
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

//! MintTo CPI for Light Token operations.

use pinocchio::{
    account_info::AccountInfo,
    cpi::{slice_invoke, slice_invoke_signed},
    instruction::{AccountMeta, Instruction, Signer},
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::constants::LIGHT_TOKEN_PROGRAM_ID;

/// Mint to ctoken via CPI.
///
/// # Example
///
/// ```rust,ignore
/// use light_token_pinocchio::instruction::MintToCpi;
///
/// MintToCpi {
///     mint: &ctx.accounts.mint,
///     destination: &ctx.accounts.destination,
///     amount: 100,
///     authority: &ctx.accounts.authority,
///     system_program: &ctx.accounts.system_program,
///     fee_payer: None,
/// }
/// .invoke()?;
/// ```
pub struct MintToCpi<'info> {
    pub mint: &'info AccountInfo,
    pub destination: &'info AccountInfo,
    pub amount: u64,
    pub authority: &'info AccountInfo,
    pub system_program: &'info AccountInfo,
    /// Optional fee payer for rent top-ups. If not provided, authority pays.
    pub fee_payer: Option<&'info AccountInfo>,
}

impl<'info> MintToCpi<'info> {
    pub fn invoke(self) -> Result<(), ProgramError> {
        self.invoke_signed(&[])
    }

    pub fn invoke_signed(self, signers: &[Signer]) -> Result<(), ProgramError> {
        // Build instruction data: discriminator(1) + amount(8) + optional max_top_up(2)
        let mut data = [0u8; 11];
        data[0] = 7u8; // MintTo discriminator
        data[1..9].copy_from_slice(&self.amount.to_le_bytes());
        let data_len = 9;

        // Authority is writable when no fee_payer is provided
        let authority_writable = self.fee_payer.is_none();

        let program_id = Pubkey::from(LIGHT_TOKEN_PROGRAM_ID);

        if let Some(fee_payer) = self.fee_payer {
            let account_metas = [
                AccountMeta::writable(self.mint.key()),
                AccountMeta::writable(self.destination.key()),
                if authority_writable {
                    AccountMeta::writable_signer(self.authority.key())
                } else {
                    AccountMeta::readonly_signer(self.authority.key())
                },
                AccountMeta::readonly(self.system_program.key()),
                AccountMeta::writable_signer(fee_payer.key()),
            ];

            let instruction = Instruction {
                program_id: &program_id,
                accounts: &account_metas,
                data: &data[..data_len],
            };

            let account_infos = [
                self.mint,
                self.destination,
                self.authority,
                self.system_program,
                fee_payer,
            ];

            if signers.is_empty() {
                slice_invoke(&instruction, &account_infos)
            } else {
                slice_invoke_signed(&instruction, &account_infos, signers)
            }
        } else {
            let account_metas = [
                AccountMeta::writable(self.mint.key()),
                AccountMeta::writable(self.destination.key()),
                if authority_writable {
                    AccountMeta::writable_signer(self.authority.key())
                } else {
                    AccountMeta::readonly_signer(self.authority.key())
                },
                AccountMeta::readonly(self.system_program.key()),
            ];

            let instruction = Instruction {
                program_id: &program_id,
                accounts: &account_metas,
                data: &data[..data_len],
            };

            let account_infos = [
                self.mint,
                self.destination,
                self.authority,
                self.system_program,
            ];

            if signers.is_empty() {
                slice_invoke(&instruction, &account_infos)
            } else {
                slice_invoke_signed(&instruction, &account_infos, signers)
            }
        }
    }
}

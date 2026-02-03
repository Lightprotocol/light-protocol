//! TransferChecked CPI for Light Token operations.

use pinocchio::{
    account_info::AccountInfo,
    cpi::slice_invoke_signed,
    instruction::{AccountMeta, Instruction, Seed, Signer},
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::constants::LIGHT_TOKEN_PROGRAM_ID;

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
///     max_top_up: None,
///     fee_payer: None,
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
    /// Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
    pub max_top_up: Option<u16>,
    /// Optional fee payer for rent top-ups. If not provided, authority pays.
    pub fee_payer: Option<&'info AccountInfo>,
}

impl<'info> TransferCheckedCpi<'info> {
    pub fn invoke(self) -> Result<(), ProgramError> {
        self.invoke_signed(&[])
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        // Build instruction data: discriminator(1) + amount(8) + decimals(1) + optional max_top_up(2)
        let mut data = [0u8; 12];
        data[0] = 12u8; // TransferChecked discriminator
        data[1..9].copy_from_slice(&self.amount.to_le_bytes());
        data[9] = self.decimals;
        let data_len = if let Some(max_top_up) = self.max_top_up {
            data[10..12].copy_from_slice(&max_top_up.to_le_bytes());
            12
        } else {
            10
        };

        // Authority is writable only when max_top_up is set AND no fee_payer
        let authority_writable = self.max_top_up.is_some() && self.fee_payer.is_none();

        let program_id = Pubkey::from(LIGHT_TOKEN_PROGRAM_ID);

        if let Some(fee_payer) = self.fee_payer {
            let account_metas = [
                AccountMeta::writable(self.source.key()),
                AccountMeta::readonly(self.mint.key()),
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
                self.source,
                self.mint,
                self.destination,
                self.authority,
                self.system_program,
                fee_payer,
            ];

            invoke_with_seeds(&instruction, &account_infos, signer_seeds)
        } else {
            let account_metas = [
                AccountMeta::writable(self.source.key()),
                AccountMeta::readonly(self.mint.key()),
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
                self.source,
                self.mint,
                self.destination,
                self.authority,
                self.system_program,
            ];

            invoke_with_seeds(&instruction, &account_infos, signer_seeds)
        }
    }
}

/// Helper function to invoke with signer seeds
#[inline(always)]
fn invoke_with_seeds(
    instruction: &Instruction,
    account_infos: &[&AccountInfo],
    signer_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    if signer_seeds.is_empty() {
        pinocchio::cpi::slice_invoke(instruction, account_infos)
    } else {
        // Convert signer seeds to pinocchio format
        // We support up to 2 signers with up to 4 seeds each
        let num_signers = signer_seeds.len().min(2);

        // Build seed arrays for each signer
        let mut seeds_storage_0: [Seed; 4] = [
            Seed::from(&[][..]),
            Seed::from(&[][..]),
            Seed::from(&[][..]),
            Seed::from(&[][..]),
        ];
        let mut seeds_storage_1: [Seed; 4] = [
            Seed::from(&[][..]),
            Seed::from(&[][..]),
            Seed::from(&[][..]),
            Seed::from(&[][..]),
        ];

        // Fill first signer's seeds
        if num_signers >= 1 {
            let seeds = signer_seeds[0];
            let num_seeds = seeds.len().min(4);
            for (j, seed) in seeds.iter().take(num_seeds).enumerate() {
                seeds_storage_0[j] = Seed::from(*seed);
            }
        }

        // Fill second signer's seeds if present
        if num_signers >= 2 {
            let seeds = signer_seeds[1];
            let num_seeds = seeds.len().min(4);
            for (j, seed) in seeds.iter().take(num_seeds).enumerate() {
                seeds_storage_1[j] = Seed::from(*seed);
            }
        }

        // Create signers from seeds
        let num_seeds_0 = if num_signers >= 1 {
            signer_seeds[0].len().min(4)
        } else {
            0
        };
        let num_seeds_1 = if num_signers >= 2 {
            signer_seeds[1].len().min(4)
        } else {
            0
        };

        let signer_0 = Signer::from(&seeds_storage_0[..num_seeds_0]);
        let signer_1 = Signer::from(&seeds_storage_1[..num_seeds_1]);

        let signers = [signer_0, signer_1];
        slice_invoke_signed(instruction, account_infos, &signers[..num_signers])
    }
}

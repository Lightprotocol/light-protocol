//! Freeze CPI for Light Token operations.

use pinocchio::{
    account_info::AccountInfo,
    cpi::slice_invoke_signed,
    instruction::{AccountMeta, Instruction, Seed, Signer},
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

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
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

        invoke_with_seeds(&instruction, &account_infos, signer_seeds)
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

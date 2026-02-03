//! Approve CPI for Light Token operations.

use pinocchio::{
    account_info::AccountInfo,
    cpi::slice_invoke_signed,
    instruction::{AccountMeta, Instruction, Seed, Signer},
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

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
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

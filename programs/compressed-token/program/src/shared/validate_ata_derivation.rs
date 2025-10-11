use anchor_lang::prelude::ProgramError;
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;

/// Validates that an account is the correct Associated Token Account PDA
///
/// Returns Ok(()) if the account key matches the expected PDA derivation.
/// This is used by both the regular and idempotent create ATA instructions.
#[inline(always)]
#[profile]
pub fn validate_ata_derivation(
    account: &AccountInfo,
    owner: &[u8; 32],
    mint: &[u8; 32],
    bump: u8,
) -> Result<(), ProgramError> {
    let seeds = &[
        owner.as_ref(),
        crate::LIGHT_CPI_SIGNER.program_id.as_ref(),
        mint.as_ref(),
    ];

    crate::shared::verify_pda(
        account.key(),
        seeds,
        bump,
        &crate::LIGHT_CPI_SIGNER.program_id,
    )
}

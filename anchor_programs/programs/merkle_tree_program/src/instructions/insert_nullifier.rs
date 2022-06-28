use crate::config;
use crate::utils::constants::NF_SEED;
use anchor_lang::prelude::*;
use anchor_lang::solana_program;

/// Nullfier pdas are derived from the nullifier
/// existence of a nullifier is the check to prevent double spends.
#[account]
pub struct Nullifier {}

#[derive(Accounts)]
#[instruction(nullifier: [u8;32], index: u64)]
pub struct InitializeNullifier<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [&(nullifier.as_slice()[0..32]), NF_SEED.as_ref()],
        bump,
        space = 8,
    )]
    pub nullifier_pda: Account<'info, Nullifier>,
    /// CHECK:` Signer is registered verifier program.
    #[account(mut, address=solana_program::pubkey::Pubkey::new(&config::REGISTERED_VERIFIER_KEY_ARRAY[index as usize]))]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

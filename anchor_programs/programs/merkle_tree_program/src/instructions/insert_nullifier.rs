use anchor_lang::prelude::*;
use crate::utils::constants::NF_SEED;
use crate::config;
use anchor_lang::solana_program;

// Nullfier pdas are derived from the nullifier
// existence of a nullifier is the check to
// prevent double spends.
#[account]
pub struct Nullifier {}

#[derive(Accounts)]
#[instruction(nullifier: [u8;32])]
pub struct InitializeNullifier<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [&(nullifier.as_slice()[0..32]), NF_SEED.as_ref()],
        bump,
        space = 8,
    )]
    pub nullifier_pda: Account<'info, Nullifier>,
    /// CHECK:` should be , address = Pubkey::new(&MERKLE_TREE_SIGNER_AUTHORITY)
    #[account(mut, address=solana_program::pubkey::Pubkey::new(&config::REGISTERED_VERIFIER_KEY_ARRAY[0]))]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

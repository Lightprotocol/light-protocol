use crate::config;
use crate::utils::constants::NF_SEED;
use anchor_lang::prelude::*;
use crate::RegisteredVerifier;
/// Nullfier pdas are derived from the nullifier
/// existence of a nullifier is the check to prevent double spends.
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
        space = 8
    )]
    pub nullifier_pda: Account<'info, Nullifier>,
    /// CHECK:` Signer is owned by registered verifier program.
    #[account(mut, seeds=[program_id.to_bytes().as_ref()],bump,seeds::program=registered_verifier_pda.pubkey)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    #[account(seeds=[&registered_verifier_pda.pubkey.to_bytes()],  bump)]
    pub registered_verifier_pda: Account<'info, RegisteredVerifier>
}

use crate::config;
use anchor_lang::prelude::*;
use crate::MerkleTreeAuthority;

/// Nullfier pdas are derived from the nullifier
/// existence of a nullifier is the check to prevent double spends.
#[account]
pub struct RegisteredVerifier {
    pub pubkey: Pubkey
}


#[derive(Accounts)]
#[instruction(verifier_pubkey: Pubkey)]
pub struct RegisterVerifier<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [&verifier_pubkey.to_bytes()],
        bump,
        space = 8 + 32
    )]
    pub registered_verifier_pda: Account<'info, RegisteredVerifier>,
    /// CHECK:` Signer is checked according to authority pda in instruction
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK:` New authority no need to be checked
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    
}

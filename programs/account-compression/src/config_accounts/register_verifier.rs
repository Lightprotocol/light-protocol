use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;

use crate::{errors::ErrorCode, MerkleTreeAuthority};

///
#[account]
#[aligned_sized(anchor)]
pub struct RegisteredVerifier {
    pub pubkey: Pubkey,
}

#[derive(Accounts)]
#[instruction(verifier_pubkey: Pubkey)]
pub struct RegisterVerifier<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [&verifier_pubkey.to_bytes()],
        bump,
        space = RegisteredVerifier::LEN,
    )]
    pub registered_verifier_pda: Account<'info, RegisteredVerifier>,
    /// CHECK:` Signer is checked according to authority pda in instruction
    #[account(mut, address=merkle_tree_authority_pda.pubkey @ErrorCode::InvalidAuthority)]
    pub authority: Signer<'info>,
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn process_register_verifier(
    ctx: Context<RegisterVerifier>,
    verifier_pubkey: Pubkey,
) -> Result<()> {
    if !ctx
        .accounts
        .merkle_tree_authority_pda
        .enable_permissionless_merkle_tree_registration
        && ctx.accounts.authority.key() != ctx.accounts.merkle_tree_authority_pda.pubkey
    {
        return err!(ErrorCode::InvalidAuthority);
    }
    ctx.accounts.registered_verifier_pda.pubkey = verifier_pubkey;
    Ok(())
}

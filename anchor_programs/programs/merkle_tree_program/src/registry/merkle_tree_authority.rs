use crate::config;
use anchor_lang::prelude::*;
use crate::errors::ErrorCode;

/// Nullfier pdas are derived from the nullifier
/// existence of a nullifier is the check to prevent double spends.
#[account]
pub struct MerkleTreeAuthority {
    pub pubkey: Pubkey,
    pub enable_nfts: bool,
    pub enable_permissionless_spl_tokens: bool,
    pub enable_permissionless_merkle_tree_registration: bool
}


#[derive(Accounts)]
pub struct InitializeMerkleTreeAuthority<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [&b"MERKLE_TREE_AUTHORITY"[..]],
        bump,
        space = 8 + 32
    )]
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
    /// CHECK:` Signer is registered verifier program.
    #[account(mut, address=anchor_lang::prelude::Pubkey::new(&config::INITIAL_MERKLE_TREE_AUTHORITY) @ErrorCode::InvalidAuthority)]
    pub authority: Signer<'info>,
    /// CHECK:` New authority no need to be checked
    pub new_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}


#[derive(Accounts)]
pub struct UpdateMerkleTreeAuthority<'info> {
    #[account(seeds = [&b"MERKLE_TREE_AUTHORITY"[..]], bump)]
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
    /// CHECK:` Signer is registered verifier program.
    #[account(mut, address=merkle_tree_authority_pda.pubkey)]
    pub authority: Signer<'info>,
    /// CHECK:` New authority no need to be checked
    pub new_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>
}

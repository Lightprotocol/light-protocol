use crate::utils::constants::{POOL_CONFIG_SEED, POOL_SEED, POOL_TYPE_SEED, TOKEN_AUTHORITY_SEED};
use crate::MerkleTreeAuthority;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

/// Nullfier pdas are derived from the nullifier
/// existence of a nullifier is the check to prevent double spends.
#[account]
pub struct RegisteredAssetPool {
    pub asset_pool_pubkey: Pubkey,
    pub pool_type: [u8; 32],
    pub index: u64,
}

/// Pool type
#[account]
pub struct RegisteredPoolType {
    pub pool_type: [u8; 32],
}

#[derive(Accounts)]
#[instruction(pool_type: [u8;32])]
pub struct RegisterPoolType<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [&pool_type, &POOL_TYPE_SEED[..]],
        bump,
        space = 8 + 32
    )]
    pub registered_pool_type_pda: Account<'info, RegisteredPoolType>,
    /// CHECK:` Signer is checked according to authority pda in instruction
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    /// CHECK:` Is checked in instruction to account for the case of permissionless pool creations.
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
}

#[derive(Accounts)]
pub struct RegisterSplPool<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [&mint.key().to_bytes(), &registered_pool_type_pda.pool_type, &POOL_CONFIG_SEED[..]],
        bump,
        space = 8 + 32 + 32 + 8
    )]
    pub registered_asset_pool_pda: Account<'info, RegisteredAssetPool>,
    #[account(init,
              seeds = [
                  &mint.key().to_bytes(), &registered_pool_type_pda.pool_type,
                  &POOL_SEED[..]
              ],
              bump,
              payer = authority,
              token::mint = mint,
              token::authority = token_authority
    )]
    pub merkle_tree_pda_token: Account<'info, TokenAccount>,
    /// CHECK:` Signer is checked according to authority pda in instruction
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    /// CHECK:
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    /// CHECK:
    #[account(mut, seeds=[TOKEN_AUTHORITY_SEED], bump)]
    pub token_authority: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    /// Just needs to exist and be derived correctly.
    #[account(
        seeds = [&registered_pool_type_pda.pool_type[..], &POOL_TYPE_SEED[..]],
        bump,
    )]
    pub registered_pool_type_pda: Account<'info, RegisteredPoolType>,
    /// CHECK:` Is checked in instruction to account for the case of permissionless pool creations.
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
}

#[derive(Accounts)]
pub struct RegisterSolPool<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [&[0u8;32], &registered_pool_type_pda.pool_type, &POOL_CONFIG_SEED[..]],
        bump,
        space = 8 + 32 + 32 + 8
    )]
    pub registered_asset_pool_pda: Account<'info, RegisteredAssetPool>,
    /// CHECK:` Signer is checked according to authority pda in instruction
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    #[account(
        seeds = [&registered_pool_type_pda.pool_type[..], &POOL_TYPE_SEED[..]],
        bump,
    )]
    pub registered_pool_type_pda: Account<'info, RegisteredPoolType>,
    /// CHECK:` Is checked in instruction to account for the case of permissionless pool creations.
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
}

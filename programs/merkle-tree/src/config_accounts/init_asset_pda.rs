use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{
    errors::ErrorCode,
    utils::constants::{POOL_CONFIG_SEED, POOL_SEED, POOL_TYPE_SEED, TOKEN_AUTHORITY_SEED},
    MerkleTreeAuthority,
};

/// Nullfier pdas are derived from the nullifier
/// existence of a nullifier is the check to prevent double spends.
#[account]
#[aligned_sized(anchor)]
pub struct RegisteredAssetPool {
    pub asset_pool_pubkey: Pubkey,
    pub pool_type: [u8; 32],
    pub index: u64,
}

/// Pool type
#[account]
#[aligned_sized(anchor)]
pub struct RegisteredPoolType {
    pub pool_type: [u8; 32],
}

#[derive(Accounts)]
#[instruction(pool_type: [u8;32])]
pub struct RegisterPoolType<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [&pool_type, POOL_TYPE_SEED],
        bump,
        space = RegisteredPoolType::LEN,
    )]
    pub registered_pool_type_pda: Account<'info, RegisteredPoolType>,
    /// CHECK:` Signer is checked according to authority pda in instruction
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK:` Is checked in instruction to account for the case of permissionless pool creations.
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
}

#[derive(Accounts)]
pub struct RegisterSplPool<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [&mint.key().to_bytes(), &registered_pool_type_pda.pool_type, POOL_CONFIG_SEED],
        bump,
        space = RegisteredAssetPool::LEN,
    )]
    pub registered_asset_pool_pda: Account<'info, RegisteredAssetPool>,
    #[account(init,
              seeds = [
                  &mint.key().to_bytes(), &registered_pool_type_pda.pool_type,
                  POOL_SEED
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
    /// CHECK:
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    /// CHECK:
    #[account(mut, seeds=[TOKEN_AUTHORITY_SEED], bump)]
    pub token_authority: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    /// Just needs to exist and be derived correctly.
    #[account(
        seeds = [&registered_pool_type_pda.pool_type[..], POOL_TYPE_SEED],
        bump,
    )]
    pub registered_pool_type_pda: Account<'info, RegisteredPoolType>,
    /// CHECK:` Is checked in instruction to account for the case of permissionless pool creations.
    #[account(mut)]
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
}

#[derive(Accounts)]
pub struct RegisterSolPool<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [&[0u8;32], &registered_pool_type_pda.pool_type, POOL_CONFIG_SEED],
        bump,
        space = RegisteredAssetPool::LEN,
    )]
    pub registered_asset_pool_pda: Account<'info, RegisteredAssetPool>,
    /// CHECK:` Signer is checked according to authority pda in instruction
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(
        seeds = [&registered_pool_type_pda.pool_type[..], POOL_TYPE_SEED],
        bump,
    )]
    pub registered_pool_type_pda: Account<'info, RegisteredPoolType>,
    /// CHECK:` Is checked in instruction to account for the case of permissionless pool creations.
    #[account(mut)]
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
}

/// Registers a new pooltype.
pub fn process_register_pool_type(
    ctx: Context<RegisterPoolType>,
    pool_type: [u8; 32],
) -> Result<()> {
    if !ctx
        .accounts
        .merkle_tree_authority_pda
        .enable_permissionless_spl_tokens
        && ctx.accounts.authority.key() != ctx.accounts.merkle_tree_authority_pda.pubkey
    {
        return err!(ErrorCode::InvalidAuthority);
    }
    ctx.accounts.registered_pool_type_pda.pool_type = pool_type;
    Ok(())
}

/// Creates a new spl token pool which can be used by any registered verifier.
pub fn process_register_spl_pool(ctx: Context<RegisterSplPool>) -> Result<()> {
    // any token enabled
    // if !ctx
    //     .accounts
    //     .merkle_tree_authority_pda
    //     .enable_permissionless_spl_tokens
    //     && ctx.accounts.authority.key() != ctx.accounts.merkle_tree_authority_pda.pubkey
    // {
    //     return err!(ErrorCode::InvalidAuthority);
    // }

    ctx.accounts.registered_asset_pool_pda.asset_pool_pubkey =
        ctx.accounts.merkle_tree_pda_token.key();
    ctx.accounts.registered_asset_pool_pda.pool_type =
        ctx.accounts.registered_pool_type_pda.pool_type;
    ctx.accounts.registered_asset_pool_pda.index = ctx
        .accounts
        .merkle_tree_authority_pda
        .registered_asset_index;
    ctx.accounts
        .merkle_tree_authority_pda
        .registered_asset_index += 1;
    Ok(())
}

/// Creates a new sol pool which can be used by any registered verifier.
pub fn process_register_sol_pool(ctx: Context<RegisterSolPool>) -> Result<()> {
    if !ctx
        .accounts
        .merkle_tree_authority_pda
        .enable_permissionless_spl_tokens
        && ctx.accounts.authority.key() != ctx.accounts.merkle_tree_authority_pda.pubkey
    {
        return err!(ErrorCode::InvalidAuthority);
    }

    ctx.accounts.registered_asset_pool_pda.asset_pool_pubkey =
        ctx.accounts.registered_asset_pool_pda.key();
    ctx.accounts.registered_asset_pool_pda.pool_type =
        ctx.accounts.registered_pool_type_pda.pool_type;
    ctx.accounts.registered_asset_pool_pda.index = ctx
        .accounts
        .merkle_tree_authority_pda
        .registered_asset_index;
    ctx.accounts
        .merkle_tree_authority_pda
        .registered_asset_index += 1;
    Ok(())
}

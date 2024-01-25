use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;

use crate::{
    config, errors::ErrorCode, state::MerkleTreeSet, utils::constants::MERKLE_TREE_AUTHORITY_SEED,
};

/// Configures the authority of the merkle tree which can:
/// - register new verifiers
/// - register new asset pools
/// - register new asset pool types
/// - set permissions for new asset pool creation
/// - keeps current highest index for assets and merkle trees to enable lookups of these
#[account]
#[aligned_sized(anchor)]
pub struct MerkleTreeAuthority {
    pub pubkey: Pubkey,
    pub merkle_tree_set_index: u64,
    pub registered_asset_index: u64,
    pub enable_permissionless_spl_tokens: bool,
    pub enable_permissionless_merkle_tree_registration: bool,
}

#[derive(Accounts)]
pub struct InitializeMerkleTreeAuthority<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [MERKLE_TREE_AUTHORITY_SEED],
        bump,
        space = MerkleTreeAuthority::LEN,
    )]
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
    #[account(zero)]
    pub merkle_tree_set: AccountLoader<'info, MerkleTreeSet>,
    /// CHECK:` Signer is merkle tree init authority.
    #[account(
        mut,
        address=anchor_lang::prelude::Pubkey::from(config::INITIAL_MERKLE_TREE_AUTHORITY) @ErrorCode::InvalidAuthority
    )]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct UpdateMerkleTreeAuthority<'info> {
    #[account(mut, seeds = [MERKLE_TREE_AUTHORITY_SEED], bump)]
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
    /// CHECK:` Signer is merkle tree authority.
    #[account(address=merkle_tree_authority_pda.pubkey @ErrorCode::InvalidAuthority)]
    pub authority: Signer<'info>,
    /// CHECK:` New authority no need to be checked
    pub new_authority: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct UpdateMerkleTreeAuthorityConfig<'info> {
    #[account(mut, seeds = [MERKLE_TREE_AUTHORITY_SEED], bump)]
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
    /// CHECK:` Signer is merkle tree authority.
    #[account(address=merkle_tree_authority_pda.pubkey @ErrorCode::InvalidAuthority)]
    pub authority: Signer<'info>,
}

pub fn process_initialize_merkle_tree_authority(
    ctx: &mut Context<InitializeMerkleTreeAuthority>,
) -> Result<()> {
    ctx.accounts.merkle_tree_authority_pda.pubkey = ctx.accounts.authority.key();

    // Initialize new Merkle trees.
    let mut new_merkle_trees = ctx.accounts.merkle_tree_set.load_init()?;
    new_merkle_trees.init(ctx.accounts.merkle_tree_authority_pda.merkle_tree_set_index)?;
    ctx.accounts.merkle_tree_authority_pda.merkle_tree_set_index += 1;

    Ok(())
}

pub fn process_update_merkle_tree_authority(ctx: Context<UpdateMerkleTreeAuthority>) -> Result<()> {
    ctx.accounts.merkle_tree_authority_pda.pubkey = ctx.accounts.new_authority.key();
    Ok(())
}

pub fn process_enable_permissionless_spl_tokens(
    ctx: Context<UpdateMerkleTreeAuthorityConfig>,
    enable_permissionless: bool,
) -> Result<()> {
    ctx.accounts
        .merkle_tree_authority_pda
        .enable_permissionless_spl_tokens = enable_permissionless;
    Ok(())
}

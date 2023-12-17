use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;

use crate::{
    config,
    errors::ErrorCode,
    event_merkle_tree::EventMerkleTree,
    process_initialize_new_event_merkle_tree, process_initialize_new_merkle_tree,
    transaction_merkle_tree::state::TransactionMerkleTree,
    utils::{
        config::MERKLE_TREE_HEIGHT,
        constants::{
            EVENT_MERKLE_TREE_SEED, MERKLE_TREE_AUTHORITY_SEED, TRANSACTION_MERKLE_TREE_SEED,
        },
    },
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
    pub transaction_merkle_tree_index: u64,
    pub event_merkle_tree_index: u64,
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
    #[account(
        init,
        seeds = [
            TRANSACTION_MERKLE_TREE_SEED,
            0u64.to_le_bytes().as_ref(),
        ],
        bump,
        payer = authority,
        space = TransactionMerkleTree::LEN,
    )]
    pub transaction_merkle_tree: AccountLoader<'info, TransactionMerkleTree>,
    #[account(
        init,
        seeds = [
            EVENT_MERKLE_TREE_SEED,
            0u64.to_le_bytes().as_ref(),
        ],
        bump,
        payer=authority,
        space = EventMerkleTree::LEN,
    )]
    pub event_merkle_tree: AccountLoader<'info, EventMerkleTree>,
    /// CHECK:` Signer is merkle tree init authority.
    #[account(mut, address=anchor_lang::prelude::Pubkey::try_from(config::INITIAL_MERKLE_TREE_AUTHORITY).map_err(|_| ErrorCode::PubkeyTryFromFailed)? @ErrorCode::InvalidAuthority)]
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
    #[account( address=merkle_tree_authority_pda.pubkey @ErrorCode::InvalidAuthority)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateLockDuration<'info> {
    #[account(mut, seeds = [MERKLE_TREE_AUTHORITY_SEED], bump)]
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
    /// CHECK:` Signer is merkle tree authority.
    #[account( address=merkle_tree_authority_pda.pubkey @ErrorCode::InvalidAuthority)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub transaction_merkle_tree: AccountLoader<'info, TransactionMerkleTree>,
}

pub fn process_initialize_merkle_tree_authority(
    ctx: &mut Context<InitializeMerkleTreeAuthority>,
) -> Result<()> {
    ctx.accounts.merkle_tree_authority_pda.pubkey = ctx.accounts.authority.key();

    let merkle_tree = &mut ctx.accounts.transaction_merkle_tree.load_init()?;
    let merkle_tree_authority = &mut ctx.accounts.merkle_tree_authority_pda;
    process_initialize_new_merkle_tree(merkle_tree, merkle_tree_authority, MERKLE_TREE_HEIGHT)?;

    let event_merkle_tree = &mut ctx.accounts.event_merkle_tree.load_init()?;
    let merkle_tree_authority = &mut ctx.accounts.merkle_tree_authority_pda;

    process_initialize_new_event_merkle_tree(event_merkle_tree, merkle_tree_authority)?;

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

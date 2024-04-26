use anchor_lang::prelude::*;

use crate::{
    initialize_address_merkle_tree::process_initialize_address_merkle_tree,
    initialize_address_queue::process_initialize_address_queue, state::AddressQueueAccount,
    utils::constants::ADDRESS_MERKLE_TREE_HEIGHT, AddressMerkleTreeAccount, NullifierQueueConfig,
    StateMerkleTreeConfig,
};

pub type AddressMerkleTreeConfig = StateMerkleTreeConfig;
pub type AddressQueueConfig = NullifierQueueConfig;
#[derive(Accounts)]
pub struct InitializeAddressMerkleTreeAndQueue<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(zero)]
    pub merkle_tree: AccountLoader<'info, AddressMerkleTreeAccount>,
    #[account(zero)]
    pub queue: AccountLoader<'info, AddressQueueAccount>,
}

pub fn process_initialize_address_merkle_tree_and_queue<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeAddressMerkleTreeAndQueue<'info>>,
    index: u64,
    owner: Pubkey,
    delegate: Option<Pubkey>,
    merkle_tree_config: AddressMerkleTreeConfig,
    queue_config: AddressQueueConfig,
) -> Result<()> {
    process_initialize_address_queue(
        &ctx.accounts.queue.to_account_info(),
        &ctx.accounts.queue,
        index,
        owner,
        delegate,
        ctx.accounts.merkle_tree.key(),
        queue_config.capacity_indices,
        queue_config.capacity_values,
        queue_config.sequence_threshold,
        queue_config.tip.unwrap_or_default(),
        merkle_tree_config.rollover_threshold,
        merkle_tree_config.height,
        ctx.accounts.merkle_tree.get_lamports(),
    )?;
    let height = ADDRESS_MERKLE_TREE_HEIGHT as u32;
    process_initialize_address_merkle_tree(
        &ctx.accounts.merkle_tree,
        index,
        owner,
        delegate,
        height,
        merkle_tree_config.changelog_size,
        merkle_tree_config.roots_size,
        merkle_tree_config.canopy_depth,
        ctx.accounts.queue.key(),
        merkle_tree_config.tip.unwrap_or_default(),
        merkle_tree_config.rollover_threshold,
        merkle_tree_config.close_threshold,
        ctx.accounts.merkle_tree.get_lamports(),
    )
}

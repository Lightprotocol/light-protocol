use anchor_lang::prelude::*;

use crate::{
    errors::AccountCompressionErrorCode,
    initialize_address_merkle_tree::process_initialize_address_merkle_tree,
    initialize_address_queue::process_initialize_address_queue,
    state::QueueAccount,
    utils::constants::{
        ADDRESS_MERKLE_TREE_CANOPY_DEPTH, ADDRESS_MERKLE_TREE_CHANGELOG,
        ADDRESS_MERKLE_TREE_HEIGHT, ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG,
        ADDRESS_MERKLE_TREE_ROOTS,
    },
    AddressMerkleTreeAccount, NullifierQueueConfig, SAFETY_MARGIN,
};

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize, PartialEq)]
pub struct AddressMerkleTreeConfig {
    pub height: u32,
    pub changelog_size: u64,
    pub roots_size: u64,
    pub canopy_depth: u64,
    pub address_changelog_size: u64,
    pub network_fee: Option<u64>,
    pub rollover_threshold: Option<u64>,
    pub close_threshold: Option<u64>,
}

impl Default for AddressMerkleTreeConfig {
    fn default() -> Self {
        Self {
            height: ADDRESS_MERKLE_TREE_HEIGHT as u32,
            changelog_size: ADDRESS_MERKLE_TREE_CHANGELOG,
            roots_size: ADDRESS_MERKLE_TREE_ROOTS,
            canopy_depth: ADDRESS_MERKLE_TREE_CANOPY_DEPTH,
            address_changelog_size: ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG,
            network_fee: Some(5000),
            rollover_threshold: Some(95),
            close_threshold: None,
        }
    }
}

pub type AddressQueueConfig = NullifierQueueConfig;

#[derive(Accounts)]
pub struct InitializeAddressMerkleTreeAndQueue<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(zero)]
    pub merkle_tree: AccountLoader<'info, AddressMerkleTreeAccount>,
    #[account(zero)]
    pub queue: AccountLoader<'info, QueueAccount>,
}

pub fn process_initialize_address_merkle_tree_and_queue<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeAddressMerkleTreeAndQueue<'info>>,
    index: u64,
    owner: Pubkey,
    program_owner: Option<Pubkey>,
    merkle_tree_config: AddressMerkleTreeConfig,
    queue_config: AddressQueueConfig,
) -> Result<()> {
    if merkle_tree_config.height as u64 != ADDRESS_MERKLE_TREE_HEIGHT {
        msg!(
            "Unsupported Merkle tree height: {}. The only currently supported height is: {}",
            merkle_tree_config.height,
            ADDRESS_MERKLE_TREE_HEIGHT
        );
        return err!(AccountCompressionErrorCode::UnsupportedHeight);
    }
    if merkle_tree_config.canopy_depth != ADDRESS_MERKLE_TREE_CANOPY_DEPTH {
        msg!(
            "Unsupported canopy depth: {}. The only currently supported depth is: {}",
            merkle_tree_config.canopy_depth,
            ADDRESS_MERKLE_TREE_CANOPY_DEPTH
        );
        return err!(AccountCompressionErrorCode::UnsupportedCanopyDepth);
    }
    let minimum_sequence_threshold = merkle_tree_config.roots_size + SAFETY_MARGIN;
    if queue_config.sequence_threshold < minimum_sequence_threshold {
        msg!(
            "Sequence threshold should be at least {}",
            minimum_sequence_threshold
        );
        return err!(AccountCompressionErrorCode::InvalidSequenceThreshold);
    }

    let merkle_tree_rent = ctx.accounts.merkle_tree.get_lamports();
    process_initialize_address_queue(
        &ctx.accounts.queue.to_account_info(),
        &ctx.accounts.queue,
        index,
        owner,
        program_owner,
        ctx.accounts.merkle_tree.key(),
        queue_config.capacity,
        queue_config.sequence_threshold,
        queue_config.network_fee.unwrap_or_default(),
        merkle_tree_config.rollover_threshold,
        merkle_tree_config.close_threshold,
        merkle_tree_config.height,
        merkle_tree_rent,
    )?;
    process_initialize_address_merkle_tree(
        &ctx.accounts.merkle_tree,
        index,
        owner,
        program_owner,
        merkle_tree_config.height,
        merkle_tree_config.changelog_size,
        merkle_tree_config.roots_size,
        merkle_tree_config.canopy_depth,
        merkle_tree_config.address_changelog_size,
        ctx.accounts.queue.key(),
        merkle_tree_config.network_fee.unwrap_or_default(),
        merkle_tree_config.rollover_threshold,
        merkle_tree_config.close_threshold,
    )
}

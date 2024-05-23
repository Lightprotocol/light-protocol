use account_compression::{MerkleTreeMetadata, QueueMetadata};
use anchor_lang::prelude::Pubkey;
use light_concurrent_merkle_tree::ConcurrentMerkleTree;
use light_hasher::Hasher;

pub fn assert_rolledover_merkle_trees<H, const HEIGHT: usize>(
    old_merkle_tree: &ConcurrentMerkleTree<H, HEIGHT>,
    new_merkle_tree: &ConcurrentMerkleTree<H, HEIGHT>,
) where
    H: Hasher,
{
    assert_eq!(old_merkle_tree.height, new_merkle_tree.height);
    assert_eq!(
        old_merkle_tree.changelog_capacity,
        new_merkle_tree.changelog_capacity
    );
    assert_eq!(
        old_merkle_tree.changelog_length,
        new_merkle_tree.changelog_length
    );
    assert_eq!(
        old_merkle_tree.current_changelog_index,
        new_merkle_tree.current_changelog_index
    );
    assert_eq!(
        old_merkle_tree.roots_capacity,
        new_merkle_tree.roots_capacity
    );
    assert_eq!(old_merkle_tree.roots_length, new_merkle_tree.roots_length);
    assert_eq!(old_merkle_tree.canopy_depth, new_merkle_tree.canopy_depth);
}

pub fn assert_rolledover_merkle_trees_metadata(
    old_merkle_tree_metadata: &MerkleTreeMetadata,
    new_merkle_tree_metadata: &MerkleTreeMetadata,
    current_slot: u64,
    new_queue_pubkey: &Pubkey,
) {
    // Old Merkle tree
    // 1. rolled over slot is set to current slot
    // 2. next Merkle tree is set to the new Merkle tree

    // New Merkle tree
    // 1. index is equal to the old Merkle tree index
    // 2. rollover fee is equal to the old Merkle tree rollover fee (the fee is calculated onchain in case rent should change the fee might be different)
    // 3. tip is equal to the old Merkle tree tip
    // 4. rollover threshold is equal to the old Merkle tree rollover threshold
    // 5. rolled over slot is set to u64::MAX (not rolled over)
    // 6. close threshold is equal to the old Merkle tree close threshold
    // 7. associated queue is equal to the new queue
    // 7. next merkle tree is set to Pubkey::default() (not set)
    // 8. owner is equal to the old Merkle tree owner
    // 9. delegate is equal to the old Merkle tree delegate

    assert_eq!(
        old_merkle_tree_metadata.access_metadata,
        new_merkle_tree_metadata.access_metadata
    );

    assert_eq!(
        old_merkle_tree_metadata.rollover_metadata.index,
        new_merkle_tree_metadata.rollover_metadata.index
    );
    assert_eq!(
        old_merkle_tree_metadata.rollover_metadata.rollover_fee,
        new_merkle_tree_metadata.rollover_metadata.rollover_fee,
    );
    assert_eq!(
        old_merkle_tree_metadata
            .rollover_metadata
            .rollover_threshold,
        new_merkle_tree_metadata
            .rollover_metadata
            .rollover_threshold,
    );
    assert_eq!(
        old_merkle_tree_metadata.rollover_metadata.network_fee,
        new_merkle_tree_metadata.rollover_metadata.network_fee,
    );
    assert_eq!(
        old_merkle_tree_metadata.rollover_metadata.rolledover_slot,
        current_slot,
    );
    assert_eq!(
        old_merkle_tree_metadata.rollover_metadata.close_threshold,
        new_merkle_tree_metadata.rollover_metadata.close_threshold
    );

    assert_eq!(new_merkle_tree_metadata.associated_queue, *new_queue_pubkey);
    assert_eq!(new_merkle_tree_metadata.next_merkle_tree, Pubkey::default());
}

#[allow(clippy::too_many_arguments)]
pub fn assert_rolledover_queues_metadata(
    old_queue_metadata: &QueueMetadata,
    new_queue_metadata: &QueueMetadata,
    current_slot: u64,
    new_merkle_tree_pubkey: &Pubkey,
    new_queue_pubkey: &Pubkey,
    old_merkle_tree_lamports: u64,
    new_merkle_tree_lamports: u64,
    new_queue_lamports: u64,
) {
    assert_eq!(
        old_queue_metadata.rollover_metadata.rolledover_slot,
        current_slot
    );
    // Isn't this wrong???
    assert_eq!(
        old_queue_metadata.rollover_metadata.index,
        new_queue_metadata.rollover_metadata.index,
    );
    assert_eq!(
        old_queue_metadata.rollover_metadata.rollover_fee,
        new_queue_metadata.rollover_metadata.rollover_fee
    );
    assert_eq!(
        old_queue_metadata.rollover_metadata.network_fee,
        new_queue_metadata.rollover_metadata.network_fee
    );
    assert_eq!(
        u64::MAX,
        new_queue_metadata.rollover_metadata.rolledover_slot
    );

    assert_eq!(
        old_queue_metadata.access_metadata.owner,
        new_queue_metadata.access_metadata.owner
    );

    assert_eq!(
        old_queue_metadata.access_metadata.delegate,
        new_queue_metadata.access_metadata.delegate
    );
    assert_eq!(
        new_queue_metadata.associated_merkle_tree,
        *new_merkle_tree_pubkey
    );
    assert_eq!(old_queue_metadata.next_queue, *new_queue_pubkey);
    assert_eq!(
        old_merkle_tree_lamports,
        new_merkle_tree_lamports + new_queue_lamports + old_merkle_tree_lamports
    );
}

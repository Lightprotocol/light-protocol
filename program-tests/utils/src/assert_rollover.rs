use anchor_lang::prelude::Pubkey;
use light_concurrent_merkle_tree::ConcurrentMerkleTree;
use light_hasher::Hasher;
use light_merkle_tree_metadata::{merkle_tree::MerkleTreeMetadata, queue::QueueMetadata};

#[track_caller]
pub fn assert_rolledover_merkle_trees<H, const HEIGHT: usize>(
    old_merkle_tree: &ConcurrentMerkleTree<H, HEIGHT>,
    new_merkle_tree: &ConcurrentMerkleTree<H, HEIGHT>,
) where
    H: Hasher,
{
    assert_eq!(old_merkle_tree.height, new_merkle_tree.height);
    assert_eq!(
        old_merkle_tree.changelog.capacity(),
        new_merkle_tree.changelog.capacity(),
    );
    assert_eq!(
        old_merkle_tree.changelog.capacity(),
        new_merkle_tree.changelog.capacity()
    );
    assert_eq!(
        old_merkle_tree.roots.capacity(),
        new_merkle_tree.roots.capacity()
    );
    assert_eq!(
        old_merkle_tree.roots.capacity(),
        new_merkle_tree.roots.capacity()
    );
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
    // 3. network_fee is equal to the old Merkle tree network_fee
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
    assert_eq!(
        old_merkle_tree_metadata.rollover_metadata.additional_bytes,
        new_merkle_tree_metadata.rollover_metadata.additional_bytes
    );

    assert_eq!(
        new_merkle_tree_metadata.associated_queue.to_bytes(),
        (*new_queue_pubkey).to_bytes()
    );
    assert_eq!(
        new_merkle_tree_metadata.next_merkle_tree.to_bytes(),
        Pubkey::default().to_bytes()
    );
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
        old_queue_metadata.access_metadata.program_owner,
        new_queue_metadata.access_metadata.program_owner
    );
    assert_eq!(
        new_queue_metadata.associated_merkle_tree.to_bytes(),
        (*new_merkle_tree_pubkey).to_bytes()
    );
    assert_eq!(
        old_queue_metadata.next_queue,
        light_compressed_account::Pubkey::from(*new_queue_pubkey)
    );
    assert_eq!(
        old_merkle_tree_lamports,
        new_merkle_tree_lamports + new_queue_lamports + old_merkle_tree_lamports
    );
}

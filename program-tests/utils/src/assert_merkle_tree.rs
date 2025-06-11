use account_compression::StateMerkleTreeAccount;
use forester_utils::account_zero_copy::{get_concurrent_merkle_tree, AccountZeroCopy};
use light_client::rpc::Rpc;
use light_hasher::Poseidon;
use light_merkle_tree_metadata::fee::compute_rollover_fee;
use solana_sdk::pubkey::Pubkey;

#[allow(clippy::too_many_arguments)]
pub async fn assert_merkle_tree_initialized<R: Rpc>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
    queue_pubkey: &Pubkey,
    height: usize,
    changelog_capacity: usize,
    roots_capacity: usize,
    canopy_depth: usize,
    expected_changelog_length: usize,
    expected_roots_length: usize,
    expected_next_index: usize,
    expected_rightmost_leaf: &[u8; 32],
    rollover_threshold: Option<u64>,
    close_threshold: Option<u64>,
    network_fee: u64,
    payer_pubkey: &Pubkey,
) {
    let merkle_tree_account = AccountZeroCopy::<account_compression::StateMerkleTreeAccount>::new(
        rpc,
        *merkle_tree_pubkey,
    )
    .await;
    let merkle_tree_account = merkle_tree_account.deserialized();

    let balance_merkle_tree = rpc
        .get_account(*merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let balance_nullifier_queue = rpc
        .get_account(*queue_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    assert_eq!(
        merkle_tree_account
            .metadata
            .rollover_metadata
            .rollover_threshold,
        rollover_threshold.unwrap_or_default()
    );
    assert_eq!(
        merkle_tree_account.metadata.rollover_metadata.network_fee,
        network_fee
    );

    let expected_rollover_fee = match rollover_threshold {
        Some(rollover_threshold) => {
            compute_rollover_fee(rollover_threshold, height as u32, balance_merkle_tree).unwrap()
                + compute_rollover_fee(rollover_threshold, height as u32, balance_nullifier_queue)
                    .unwrap()
        }
        None => 0,
    };
    assert_eq!(
        merkle_tree_account.metadata.rollover_metadata.rollover_fee,
        expected_rollover_fee
    );
    assert_eq!(merkle_tree_account.metadata.rollover_metadata.index, 1);
    assert_eq!(
        merkle_tree_account
            .metadata
            .rollover_metadata
            .rolledover_slot,
        u64::MAX
    );

    assert_eq!(
        merkle_tree_account
            .metadata
            .rollover_metadata
            .close_threshold,
        close_threshold.unwrap_or(u64::MAX)
    );

    assert_eq!(
        merkle_tree_account.metadata.next_merkle_tree.to_bytes(),
        [0u8; 32]
    );
    assert_eq!(
        merkle_tree_account
            .metadata
            .access_metadata
            .owner
            .to_bytes(),
        (*payer_pubkey).to_bytes()
    );
    assert_eq!(
        merkle_tree_account
            .metadata
            .access_metadata
            .program_owner
            .to_bytes(),
        [0u8; 32]
    );
    assert_eq!(
        merkle_tree_account.metadata.associated_queue.to_bytes(),
        (*queue_pubkey).to_bytes()
    );

    let merkle_tree = get_concurrent_merkle_tree::<StateMerkleTreeAccount, R, Poseidon, 26>(
        rpc,
        *merkle_tree_pubkey,
    )
    .await;

    assert_eq!(merkle_tree.height, height);
    assert_eq!(merkle_tree.changelog.capacity(), changelog_capacity);
    assert_eq!(merkle_tree.changelog.len(), expected_changelog_length);
    assert_eq!(
        merkle_tree.changelog_index(),
        expected_changelog_length.saturating_sub(1)
    );
    assert_eq!(merkle_tree.roots.capacity(), roots_capacity);
    assert_eq!(merkle_tree.roots.len(), expected_roots_length);
    assert_eq!(
        merkle_tree.root_index(),
        expected_roots_length.saturating_sub(1)
    );
    assert_eq!(merkle_tree.canopy_depth, canopy_depth);
    assert_eq!(merkle_tree.next_index(), expected_next_index);
    assert_eq!(
        merkle_tree.sequence_number(),
        expected_roots_length.saturating_sub(1)
    );
    assert_eq!(&merkle_tree.rightmost_leaf(), expected_rightmost_leaf);
}

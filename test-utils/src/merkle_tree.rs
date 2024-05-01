use light_utils::fee::compute_rollover_fee;
use solana_program_test::ProgramTestContext;
use solana_sdk::pubkey::Pubkey;

use crate::AccountZeroCopy;

#[allow(clippy::too_many_arguments)]
pub async fn assert_merkle_tree_initialized(
    context: &mut ProgramTestContext,
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
    let merkle_tree = AccountZeroCopy::<account_compression::StateMerkleTreeAccount>::new(
        context,
        *merkle_tree_pubkey,
    )
    .await;
    let merkle_tree_account = merkle_tree.deserialized();

    let merkle_tree = merkle_tree_account.copy_merkle_tree().unwrap();

    let balance_merkle_tree = context
        .banks_client
        .get_account(*merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let balance_nullifier_queue = context
        .banks_client
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
        merkle_tree_account.metadata.next_merkle_tree,
        Pubkey::default()
    );
    assert_eq!(
        merkle_tree_account.metadata.access_metadata.owner,
        *payer_pubkey
    );
    assert_eq!(
        merkle_tree_account.metadata.access_metadata.delegate,
        Pubkey::default()
    );
    assert_eq!(merkle_tree_account.metadata.associated_queue, *queue_pubkey);
    assert_eq!(merkle_tree.height, height);
    assert_eq!(merkle_tree.changelog_capacity, changelog_capacity);
    assert_eq!(merkle_tree.changelog_length, expected_changelog_length);
    assert_eq!(
        merkle_tree.current_changelog_index,
        expected_changelog_length.saturating_sub(1)
    );
    assert_eq!(merkle_tree.roots_capacity, roots_capacity);
    assert_eq!(merkle_tree.roots_length, expected_roots_length);
    assert_eq!(
        merkle_tree.current_root_index,
        expected_roots_length.saturating_sub(1)
    );
    assert_eq!(merkle_tree.canopy_depth, canopy_depth);
    assert_eq!(merkle_tree.next_index, expected_next_index);
    assert_eq!(
        merkle_tree.sequence_number,
        expected_roots_length.saturating_sub(1)
    );
    assert_eq!(&merkle_tree.rightmost_leaf, expected_rightmost_leaf);
}

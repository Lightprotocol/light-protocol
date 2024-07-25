use crate::{get_indexed_merkle_tree, rpc::rpc_connection::RpcConnection, AccountZeroCopy};
use light_hasher::Poseidon;
use solana_sdk::pubkey::Pubkey;

#[allow(clippy::too_many_arguments)]
pub async fn assert_address_merkle_tree_initialized<R: RpcConnection>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
    queue_pubkey: &Pubkey,
    merkle_tree_config: &account_compression::AddressMerkleTreeConfig,
    index: u64,
    program_owner: Option<Pubkey>,
    expected_changelog_length: usize,
    expected_roots_length: usize,
    expected_next_index: usize,
    expected_rightmost_leaf: &[u8; 32],
    owner_pubkey: &Pubkey,
    expected_indexed_changelog_length: usize,
) {
    let merkle_tree = AccountZeroCopy::<account_compression::AddressMerkleTreeAccount>::new(
        rpc,
        *merkle_tree_pubkey,
    )
    .await;
    let merkle_tree_account = merkle_tree.deserialized();

    assert_eq!(
        merkle_tree_account
            .metadata
            .rollover_metadata
            .rollover_threshold,
        merkle_tree_config.rollover_threshold.unwrap_or_default()
    );
    assert_eq!(
        merkle_tree_account.metadata.rollover_metadata.network_fee,
        merkle_tree_config.network_fee.unwrap_or_default()
    );

    // The address Merkle tree is never directly called by the user.
    // The whole rollover fees are collected by the address queue.
    let expected_rollover_fee = 0;
    assert_eq!(
        merkle_tree_account.metadata.rollover_metadata.rollover_fee,
        expected_rollover_fee
    );

    assert_eq!(merkle_tree_account.metadata.rollover_metadata.index, index);
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
        merkle_tree_config.close_threshold.unwrap_or(u64::MAX)
    );

    assert_eq!(
        merkle_tree_account.metadata.next_merkle_tree,
        Pubkey::default()
    );
    let expected_access_meta_data = account_compression::AccessMetadata {
        owner: *owner_pubkey,
        program_owner: program_owner.unwrap_or_default(),
    };
    assert_eq!(
        merkle_tree_account.metadata.access_metadata,
        expected_access_meta_data
    );
    assert_eq!(merkle_tree_account.metadata.associated_queue, *queue_pubkey);

    let merkle_tree = get_indexed_merkle_tree::<
        account_compression::AddressMerkleTreeAccount,
        R,
        Poseidon,
        usize,
        26,
        16,
    >(rpc, *merkle_tree_pubkey)
    .await;

    assert_eq!(merkle_tree.height, merkle_tree_config.height as usize);
    assert_eq!(
        merkle_tree.merkle_tree.changelog.capacity(),
        merkle_tree_config.changelog_size as usize
    );
    assert_eq!(
        merkle_tree.merkle_tree.changelog.len(),
        expected_changelog_length
    );
    assert_eq!(
        merkle_tree.merkle_tree.changelog_index(),
        expected_changelog_length.saturating_sub(1)
    );
    assert_eq!(
        merkle_tree.roots.capacity(),
        merkle_tree_config.roots_size as usize
    );
    assert_eq!(merkle_tree.roots.len(), expected_roots_length);
    assert_eq!(
        merkle_tree.root_index(),
        expected_roots_length.saturating_sub(1)
    );
    assert_eq!(
        merkle_tree.canopy_depth,
        merkle_tree_config.canopy_depth as usize
    );
    assert_eq!(merkle_tree.next_index(), expected_next_index);
    assert_eq!(
        merkle_tree.sequence_number() % merkle_tree_config.roots_size as usize,
        expected_roots_length.saturating_sub(1)
    );
    assert_eq!(&merkle_tree.rightmost_leaf(), expected_rightmost_leaf);
    // TODO: complete asserts
    assert_eq!(
        merkle_tree.indexed_changelog_index(),
        expected_indexed_changelog_length.saturating_sub(1)
    );
}

use std::cmp;

use account_compression::{AddressMerkleTreeConfig, AddressQueueConfig, RegisteredProgram};
use forester_utils::account_zero_copy::{get_hash_set, get_indexed_merkle_tree, AccountZeroCopy};
use light_client::rpc::{Rpc, RpcError};
use light_hasher::Poseidon;
use light_merkle_tree_metadata::{
    access::AccessMetadata, fee::compute_rollover_fee, queue::QueueMetadata,
    rollover::RolloverMetadata, QueueType,
};
use light_program_test::accounts::address_tree::create_address_merkle_tree_and_queue_account;
use light_registry::account_compression_cpi::sdk::get_registered_program_pda;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
};

#[allow(clippy::too_many_arguments)]
#[inline(never)]
pub async fn create_address_merkle_tree_and_queue_account_with_assert<R: Rpc>(
    payer: &Keypair,
    registry: bool,
    context: &mut R,
    address_merkle_tree_keypair: &Keypair,
    address_queue_keypair: &Keypair,
    program_owner: Option<Pubkey>,
    forester: Option<Pubkey>,
    merkle_tree_config: &AddressMerkleTreeConfig,
    queue_config: &AddressQueueConfig,
    index: u64,
) -> Result<Signature, RpcError> {
    let result = create_address_merkle_tree_and_queue_account(
        payer,
        registry,
        context,
        address_merkle_tree_keypair,
        address_queue_keypair,
        program_owner,
        forester,
        merkle_tree_config,
        queue_config,
        index,
    )
    .await;

    // To initialize the indexed tree we do 4 operations:
    // 1. insert 0 append 0 and update 0
    // 2. insert 1 append BN254_FIELD_SIZE -1 and update 0
    // we appended two values this the expected next index is 2;
    // The right most leaf is the hash of the indexed array element with value FIELD_SIZE - 1
    // index 1, next_index: 0
    let expected_change_log_length = cmp::min(4, merkle_tree_config.changelog_size as usize);
    let expected_roots_length = cmp::min(4, merkle_tree_config.roots_size as usize);
    let expected_next_index = 2;
    let expected_indexed_change_log_length =
        cmp::min(4, merkle_tree_config.address_changelog_size as usize);

    let mut reference_tree =
        light_indexed_merkle_tree::reference::IndexedMerkleTree::<Poseidon, usize>::new(
            account_compression::utils::constants::ADDRESS_MERKLE_TREE_HEIGHT as usize,
            account_compression::utils::constants::ADDRESS_MERKLE_TREE_CANOPY_DEPTH as usize,
        )
        .unwrap();
    reference_tree.init().unwrap();

    let expected_right_most_leaf = reference_tree
        .merkle_tree
        .get_leaf(reference_tree.merkle_tree.rightmost_index - 1)
        .unwrap();

    let _expected_right_most_leaf = [
        30, 164, 22, 238, 180, 2, 24, 181, 64, 193, 207, 184, 219, 233, 31, 109, 84, 232, 162, 158,
        220, 48, 163, 158, 50, 107, 64, 87, 167, 217, 99, 245,
    ];
    assert_eq!(expected_right_most_leaf, _expected_right_most_leaf);
    let owner = if registry {
        let registered_program = get_registered_program_pda(&light_registry::ID);
        let registered_program_account = context
            .get_anchor_account::<RegisteredProgram>(&registered_program)
            .await
            .unwrap()
            .unwrap();
        registered_program_account.group_authority_pda
    } else {
        payer.pubkey()
    };

    assert_address_merkle_tree_initialized(
        context,
        &address_merkle_tree_keypair.pubkey(),
        &address_queue_keypair.pubkey(),
        merkle_tree_config,
        index,
        program_owner,
        forester,
        expected_change_log_length,
        expected_roots_length,
        expected_next_index,
        &expected_right_most_leaf,
        &owner,
        expected_indexed_change_log_length,
    )
    .await;

    assert_address_queue_initialized(
        context,
        &address_queue_keypair.pubkey(),
        queue_config,
        &address_merkle_tree_keypair.pubkey(),
        merkle_tree_config,
        QueueType::AddressV1,
        index,
        program_owner,
        forester,
        &owner,
    )
    .await;

    result
}

#[allow(clippy::too_many_arguments)]
pub async fn assert_address_merkle_tree_initialized<R: Rpc>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
    queue_pubkey: &Pubkey,
    merkle_tree_config: &account_compression::AddressMerkleTreeConfig,
    index: u64,
    program_owner: Option<Pubkey>,
    forester: Option<Pubkey>,
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
        merkle_tree_config.rollover_threshold.unwrap_or(u64::MAX)
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
        merkle_tree_account.metadata.next_merkle_tree.to_bytes(),
        [0u8; 32]
    );
    let expected_access_meta_data = AccessMetadata {
        owner: (*owner_pubkey).into(),
        program_owner: program_owner.unwrap_or_default().into(),
        forester: forester.unwrap_or_default().into(),
    };
    assert_eq!(
        merkle_tree_account.metadata.access_metadata,
        expected_access_meta_data
    );
    assert_eq!(
        merkle_tree_account.metadata.associated_queue.to_bytes(),
        (*queue_pubkey).to_bytes()
    );

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

#[allow(clippy::too_many_arguments)]
pub async fn assert_address_queue_initialized<R: Rpc>(
    rpc: &mut R,
    queue_pubkey: &Pubkey,
    queue_config: &account_compression::AddressQueueConfig,
    associated_merkle_tree_pubkey: &Pubkey,
    associated_tree_config: &account_compression::AddressMerkleTreeConfig,
    expected_queue_type: QueueType,
    expected_index: u64,
    expected_program_owner: Option<Pubkey>,
    expected_forester: Option<Pubkey>,
    payer_pubkey: &Pubkey,
) {
    assert_address_queue(
        rpc,
        queue_pubkey,
        queue_config,
        associated_merkle_tree_pubkey,
        associated_tree_config,
        expected_queue_type,
        expected_index,
        expected_program_owner,
        expected_forester,
        None,
        None,
        payer_pubkey,
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
pub async fn assert_address_queue<R: Rpc>(
    rpc: &mut R,
    queue_pubkey: &Pubkey,
    queue_config: &account_compression::AddressQueueConfig,
    associated_merkle_tree_pubkey: &Pubkey,
    associated_tree_config: &account_compression::AddressMerkleTreeConfig,
    expected_queue_type: QueueType,
    expected_index: u64,
    expected_program_owner: Option<Pubkey>,
    expected_forester: Option<Pubkey>,
    expected_rolledover_slot: Option<u64>,
    expected_next_queue: Option<Pubkey>,
    payer_pubkey: &Pubkey,
) {
    let balance_merkle_tree = rpc
        .get_account(*associated_merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let balance_queue = rpc
        .get_account(*queue_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    // The address queue is the only account that collects the rollover fees.
    let expected_rollover_fee = match associated_tree_config.rollover_threshold {
        Some(threshold) => {
            compute_rollover_fee(threshold, associated_tree_config.height, balance_queue).unwrap()
                + compute_rollover_fee(
                    threshold,
                    associated_tree_config.height,
                    balance_merkle_tree,
                )
                .unwrap()
        }
        None => 0,
    };
    assert_queue(
        rpc,
        queue_pubkey,
        queue_config,
        associated_merkle_tree_pubkey,
        associated_tree_config,
        expected_rollover_fee,
        expected_queue_type,
        expected_index,
        expected_program_owner,
        expected_forester,
        expected_rolledover_slot,
        expected_next_queue,
        payer_pubkey,
    )
    .await;
}
#[allow(clippy::too_many_arguments)]
pub async fn assert_queue<R: Rpc>(
    rpc: &mut R,
    queue_pubkey: &Pubkey,
    queue_config: &account_compression::AddressQueueConfig,
    associated_merkle_tree_pubkey: &Pubkey,
    associated_tree_config: &account_compression::AddressMerkleTreeConfig,
    expected_rollover_fee: u64,
    expected_queue_type: QueueType,
    expected_index: u64,
    expected_program_owner: Option<Pubkey>,
    expected_forester: Option<Pubkey>,
    expected_rolledover_slot: Option<u64>,
    expected_next_queue: Option<Pubkey>,
    payer_pubkey: &Pubkey,
) {
    let queue = AccountZeroCopy::<account_compression::QueueAccount>::new(rpc, *queue_pubkey).await;
    let queue_account = queue.deserialized();

    let expected_rollover_meta_data = RolloverMetadata {
        index: expected_index,
        rolledover_slot: expected_rolledover_slot.unwrap_or(u64::MAX),
        rollover_threshold: associated_tree_config
            .rollover_threshold
            .unwrap_or(u64::MAX),
        network_fee: queue_config.network_fee.unwrap_or_default(),
        rollover_fee: expected_rollover_fee,
        close_threshold: associated_tree_config.close_threshold.unwrap_or(u64::MAX),
        additional_bytes: 0,
    };
    let expected_access_meta_data = AccessMetadata {
        owner: (*payer_pubkey).into(),
        program_owner: expected_program_owner.unwrap_or_default().into(),
        forester: expected_forester.unwrap_or_default().into(),
    };
    let expected_queue_meta_data = QueueMetadata {
        access_metadata: expected_access_meta_data,
        rollover_metadata: expected_rollover_meta_data,
        associated_merkle_tree: (*associated_merkle_tree_pubkey).into(),
        next_queue: expected_next_queue.unwrap_or_default().into(),
        queue_type: expected_queue_type as u64,
    };
    assert_eq!(queue_account.metadata, expected_queue_meta_data);

    let queue =
        unsafe { get_hash_set::<account_compression::QueueAccount, R>(rpc, *queue_pubkey).await };
    assert_eq!(queue.get_capacity(), queue_config.capacity as usize);
    assert_eq!(
        queue.sequence_threshold,
        queue_config.sequence_threshold as usize
    );
}

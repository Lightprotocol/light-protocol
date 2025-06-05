use account_compression::QueueAccount;
use forester_utils::account_zero_copy::{get_hash_set, AccountZeroCopy};
use light_client::rpc::Rpc;
use light_merkle_tree_metadata::{
    access::AccessMetadata, fee::compute_rollover_fee, queue::QueueMetadata,
    rollover::RolloverMetadata, QueueType,
};
use solana_sdk::pubkey::Pubkey;

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
pub async fn assert_nullifier_queue_initialized<R: Rpc>(
    rpc: &mut R,
    queue_pubkey: &Pubkey,
    queue_config: &account_compression::NullifierQueueConfig,
    associated_merkle_tree_pubkey: &Pubkey,
    associated_tree_config: &account_compression::StateMerkleTreeConfig,
    expected_queue_type: QueueType,
    expected_index: u64,
    expected_program_owner: Option<Pubkey>,
    expected_forester: Option<Pubkey>,
    payer_pubkey: &Pubkey,
) {
    let associated_tree_config = account_compression::AddressMerkleTreeConfig {
        height: associated_tree_config.height,
        changelog_size: associated_tree_config.changelog_size,
        // not asserted here
        address_changelog_size: 0,
        roots_size: associated_tree_config.roots_size,
        canopy_depth: associated_tree_config.canopy_depth,
        rollover_threshold: associated_tree_config.rollover_threshold,
        close_threshold: associated_tree_config.close_threshold,
        network_fee: associated_tree_config.network_fee,
    };
    // The address queue is the only account that collects the rollover fees.
    let expected_rollover_fee = 0;
    assert_queue(
        rpc,
        queue_pubkey,
        queue_config,
        associated_merkle_tree_pubkey,
        &associated_tree_config,
        expected_rollover_fee,
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

    let queue = unsafe { get_hash_set::<QueueAccount, R>(rpc, *queue_pubkey).await };
    assert_eq!(queue.get_capacity(), queue_config.capacity as usize);
    assert_eq!(
        queue.sequence_threshold,
        queue_config.sequence_threshold as usize
    );
}

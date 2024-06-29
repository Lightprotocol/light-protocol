use crate::{
    get_concurrent_merkle_tree, get_hash_set, get_indexed_merkle_tree,
    indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts},
    rpc::rpc_connection::RpcConnection,
    AccountZeroCopy,
};
use account_compression::{
    AddressMerkleTreeAccount, AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig,
    QueueAccount, StateMerkleTreeAccount, StateMerkleTreeConfig,
};
use light_hasher::Poseidon;
use num_traits::Zero;
use solana_sdk::pubkey::Pubkey;

pub async fn get_address_bundle_config<R: RpcConnection>(
    rpc: &mut R,
    address_bundle: AddressMerkleTreeAccounts,
) -> (AddressMerkleTreeConfig, AddressQueueConfig) {
    let address_queue_meta_data =
        AccountZeroCopy::<account_compression::QueueAccount>::new(rpc, address_bundle.queue)
            .await
            .deserialized()
            .metadata;
    let address_queue = unsafe { get_hash_set::<QueueAccount, R>(rpc, address_bundle.queue).await };
    let queue_config = AddressQueueConfig {
        network_fee: Some(address_queue_meta_data.rollover_metadata.network_fee),
        // rollover_threshold: address_queue_meta_data.rollover_threshold,
        capacity: address_queue.capacity as u16,
        sequence_threshold: address_queue.sequence_threshold as u64,
    };
    let address_tree_meta_data =
        AccountZeroCopy::<account_compression::AddressMerkleTreeAccount>::new(
            rpc,
            address_bundle.merkle_tree,
        )
        .await
        .deserialized()
        .metadata;
    let address_tree =
        get_indexed_merkle_tree::<AddressMerkleTreeAccount, R, Poseidon, usize, 26, 16>(
            rpc,
            address_bundle.merkle_tree,
        )
        .await;
    let address_merkle_tree_config = AddressMerkleTreeConfig {
        height: address_tree.height as u32,
        changelog_size: address_tree.merkle_tree.changelog.capacity() as u64,
        roots_size: address_tree.merkle_tree.roots.capacity() as u64,
        canopy_depth: address_tree.canopy_depth as u64,
        address_changelog_size: address_tree.indexed_changelog.capacity() as u64,
        rollover_threshold: if address_tree_meta_data
            .rollover_metadata
            .rollover_threshold
            .is_zero()
        {
            None
        } else {
            Some(address_tree_meta_data.rollover_metadata.rollover_threshold)
        },
        network_fee: Some(address_tree_meta_data.rollover_metadata.network_fee),
        close_threshold: None,
    };
    (address_merkle_tree_config, queue_config)
}

pub async fn get_state_bundle_config<R: RpcConnection>(
    rpc: &mut R,
    state_tree_bundle: StateMerkleTreeAccounts,
) -> (StateMerkleTreeConfig, NullifierQueueConfig) {
    let address_queue_meta_data = AccountZeroCopy::<account_compression::QueueAccount>::new(
        rpc,
        state_tree_bundle.nullifier_queue,
    )
    .await
    .deserialized()
    .metadata;
    let address_queue =
        unsafe { get_hash_set::<QueueAccount, R>(rpc, state_tree_bundle.nullifier_queue).await };
    let queue_config = NullifierQueueConfig {
        network_fee: Some(address_queue_meta_data.rollover_metadata.network_fee),
        capacity: address_queue.capacity as u16,
        sequence_threshold: address_queue.sequence_threshold as u64,
    };
    let address_tree_meta_data =
        AccountZeroCopy::<account_compression::StateMerkleTreeAccount>::new(
            rpc,
            state_tree_bundle.merkle_tree,
        )
        .await
        .deserialized()
        .metadata;
    let address_tree = get_concurrent_merkle_tree::<StateMerkleTreeAccount, R, Poseidon, 26>(
        rpc,
        state_tree_bundle.merkle_tree,
    )
    .await;
    let address_merkle_tree_config = StateMerkleTreeConfig {
        height: address_tree.height as u32,
        changelog_size: address_tree.changelog.capacity() as u64,
        roots_size: address_tree.roots.capacity() as u64,
        canopy_depth: address_tree.canopy_depth as u64,
        rollover_threshold: if address_tree_meta_data
            .rollover_metadata
            .rollover_threshold
            .is_zero()
        {
            None
        } else {
            Some(address_tree_meta_data.rollover_metadata.rollover_threshold)
        },
        network_fee: Some(address_tree_meta_data.rollover_metadata.network_fee),
        close_threshold: None,
    };
    (address_merkle_tree_config, queue_config)
}

pub async fn address_tree_ready_for_rollover<R: RpcConnection>(
    rpc: &mut R,
    merkle_tree: Pubkey,
) -> bool {
    let account =
        AccountZeroCopy::<account_compression::AddressMerkleTreeAccount>::new(rpc, merkle_tree)
            .await;
    let rent_exemption = rpc
        .get_minimum_balance_for_rent_exemption(account.account.data.len())
        .await
        .unwrap();
    let address_tree_meta_data = account.deserialized().metadata;

    let address_tree =
        get_indexed_merkle_tree::<AddressMerkleTreeAccount, R, Poseidon, usize, 26, 16>(
            rpc,
            merkle_tree,
        )
        .await;
    // rollover threshold is reached
    address_tree.next_index()
        >= ((1 << address_tree.merkle_tree.height)
            * address_tree_meta_data.rollover_metadata.rollover_threshold
            / 100) as usize
                // hash sufficient funds for rollover
&& account.account.lamports >= rent_exemption * 2
               // has not been rolled over
 && address_tree_meta_data.rollover_metadata.rolledover_slot == u64::MAX
}

pub async fn state_tree_ready_for_rollover<R: RpcConnection>(
    rpc: &mut R,
    merkle_tree: Pubkey,
) -> bool {
    let account = AccountZeroCopy::<StateMerkleTreeAccount>::new(rpc, merkle_tree).await;
    let rent_exemption = rpc
        .get_minimum_balance_for_rent_exemption(account.account.data.len())
        .await
        .unwrap();
    let tree_meta_data = account.deserialized().metadata;
    let tree =
        get_concurrent_merkle_tree::<StateMerkleTreeAccount, R, Poseidon, 26>(rpc, merkle_tree)
            .await;

    // rollover threshold is reached
    tree.next_index()
        >= ((1 << tree.height) * tree_meta_data.rollover_metadata.rollover_threshold / 100) as usize
        // hash sufficient funds for rollover
        && account.account.lamports >= rent_exemption * 2
        // has not been rolled over
        && tree_meta_data.rollover_metadata.rolledover_slot == u64::MAX
}

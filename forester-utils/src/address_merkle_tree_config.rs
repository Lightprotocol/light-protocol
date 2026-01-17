use account_compression::{
    AddressMerkleTreeAccount, AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig,
    QueueAccount, StateMerkleTreeAccount, StateMerkleTreeConfig,
};
use anchor_lang::Discriminator;
use light_account_checks::discriminator::Discriminator as LightDiscriminator;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_client::{
    indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts},
    rpc::Rpc,
};
use light_hasher::Poseidon;
use num_traits::Zero;
use solana_sdk::pubkey::Pubkey;

use crate::account_zero_copy::{
    get_concurrent_merkle_tree, get_hash_set, get_indexed_merkle_tree, AccountZeroCopy,
    AccountZeroCopyError,
};

pub async fn get_address_bundle_config<R: Rpc>(
    rpc: &mut R,
    address_bundle: AddressMerkleTreeAccounts,
) -> Result<(AddressMerkleTreeConfig, AddressQueueConfig), AccountZeroCopyError> {
    // Get queue metadata - don't hold AccountZeroCopy across await points
    let address_queue_meta_data = {
        let account = AccountZeroCopy::<QueueAccount>::new(rpc, address_bundle.queue).await?;
        account.deserialized().metadata
    };
    let address_queue =
        unsafe { get_hash_set::<QueueAccount, R>(rpc, address_bundle.queue).await? };
    let queue_config = AddressQueueConfig {
        network_fee: Some(address_queue_meta_data.rollover_metadata.network_fee),
        // rollover_threshold: address_queue_meta_data.rollover_threshold,
        capacity: address_queue.get_capacity() as u16,
        sequence_threshold: address_queue.sequence_threshold as u64,
    };
    // Get tree metadata - don't hold AccountZeroCopy across await points
    let address_tree_meta_data = {
        let account =
            AccountZeroCopy::<AddressMerkleTreeAccount>::new(rpc, address_bundle.merkle_tree)
                .await?;
        account.deserialized().metadata
    };
    let address_tree =
        get_indexed_merkle_tree::<AddressMerkleTreeAccount, R, Poseidon, usize, 26, 16>(
            rpc,
            address_bundle.merkle_tree,
        )
        .await?;
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
    Ok((address_merkle_tree_config, queue_config))
}

pub async fn get_state_bundle_config<R: Rpc>(
    rpc: &mut R,
    state_tree_bundle: StateMerkleTreeAccounts,
) -> Result<(StateMerkleTreeConfig, NullifierQueueConfig), AccountZeroCopyError> {
    // Get queue metadata - don't hold AccountZeroCopy across await points
    let address_queue_meta_data = {
        let account =
            AccountZeroCopy::<QueueAccount>::new(rpc, state_tree_bundle.nullifier_queue).await?;
        account.deserialized().metadata
    };
    let address_queue =
        unsafe { get_hash_set::<QueueAccount, R>(rpc, state_tree_bundle.nullifier_queue).await? };
    let queue_config = NullifierQueueConfig {
        network_fee: Some(address_queue_meta_data.rollover_metadata.network_fee),
        capacity: address_queue.get_capacity() as u16,
        sequence_threshold: address_queue.sequence_threshold as u64,
    };
    // Get tree metadata - don't hold AccountZeroCopy across await points
    let address_tree_meta_data = {
        let account =
            AccountZeroCopy::<StateMerkleTreeAccount>::new(rpc, state_tree_bundle.merkle_tree)
                .await?;
        account.deserialized().metadata
    };
    let address_tree = get_concurrent_merkle_tree::<StateMerkleTreeAccount, R, Poseidon, 26>(
        rpc,
        state_tree_bundle.merkle_tree,
    )
    .await?;
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
    Ok((address_merkle_tree_config, queue_config))
}

pub async fn address_tree_ready_for_rollover<R: Rpc>(
    rpc: &mut R,
    merkle_tree: Pubkey,
) -> Result<bool, AccountZeroCopyError> {
    // Get account data - don't hold AccountZeroCopy across await points
    let (address_tree_meta_data, account_data_len, account_lamports) = {
        let account = AccountZeroCopy::<AddressMerkleTreeAccount>::new(rpc, merkle_tree).await?;
        (
            account.deserialized().metadata,
            account.account.data.len(),
            account.account.lamports,
        )
    };
    let rent_exemption = rpc
        .get_minimum_balance_for_rent_exemption(account_data_len)
        .await
        .map_err(|e| AccountZeroCopyError::RpcError(e.to_string()))?;

    let address_tree =
        get_indexed_merkle_tree::<AddressMerkleTreeAccount, R, Poseidon, usize, 26, 16>(
            rpc,
            merkle_tree,
        )
        .await?;
    // rollover threshold is reached
    Ok(address_tree.next_index()
        >= ((1 << address_tree.merkle_tree.height)
            * address_tree_meta_data.rollover_metadata.rollover_threshold
            / 100) as usize
                // has sufficient funds for rollover
        && account_lamports >= rent_exemption * 2
               // has not been rolled over
        && address_tree_meta_data.rollover_metadata.rolledover_slot == u64::MAX)
}

pub async fn state_tree_ready_for_rollover<R: Rpc>(
    rpc: &mut R,
    merkle_tree: Pubkey,
) -> Result<bool, AccountZeroCopyError> {
    let mut account = rpc
        .get_account(merkle_tree)
        .await
        .map_err(|e| AccountZeroCopyError::RpcError(e.to_string()))?
        .ok_or(AccountZeroCopyError::AccountNotFound(merkle_tree))?;
    let rent_exemption = rpc
        .get_minimum_balance_for_rent_exemption(account.data.len())
        .await
        .map_err(|e| AccountZeroCopyError::RpcError(e.to_string()))?;
    let discriminator = &account.data[0..8];
    let (next_index, tree_meta_data, height) = match discriminator {
        d if d == StateMerkleTreeAccount::DISCRIMINATOR => {
            // Get tree metadata - don't hold AccountZeroCopy across await points
            let tree_meta_data = {
                let account =
                    AccountZeroCopy::<StateMerkleTreeAccount>::new(rpc, merkle_tree).await?;
                account.deserialized().metadata
            };
            let tree = get_concurrent_merkle_tree::<StateMerkleTreeAccount, R, Poseidon, 26>(
                rpc,
                merkle_tree,
            )
            .await?;
            (tree.next_index(), tree_meta_data, 26)
        }
        d if d == BatchedMerkleTreeAccount::LIGHT_DISCRIMINATOR_SLICE => {
            let tree_meta_data = BatchedMerkleTreeAccount::state_from_bytes(
                account.data.as_mut_slice(),
                &merkle_tree.into(),
            )
            .map_err(|e| {
                AccountZeroCopyError::RpcError(format!("Failed to parse batched tree: {:?}", e))
            })?;

            (
                tree_meta_data.next_index as usize,
                tree_meta_data.metadata,
                tree_meta_data.height,
            )
        }
        _ => {
            return Err(AccountZeroCopyError::RpcError(
                "Invalid discriminator".to_string(),
            ))
        }
    };
    // rollover threshold is reached
    Ok(
        next_index >= ((1 << height) * tree_meta_data.rollover_metadata.rollover_threshold / 100) as usize
        // has sufficient funds for rollover
        && account.lamports >= rent_exemption * 2
        // has not been rolled over
        && tree_meta_data.rollover_metadata.rolledover_slot == u64::MAX,
    )
}

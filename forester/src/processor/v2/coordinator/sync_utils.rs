/// Chain synchronization and common utilities for coordinators.
///
/// This module provides common logic for syncing coordinator state with on-chain merkle tree data
/// and other shared coordinator operations.

use anyhow::Result;
use light_batched_merkle_tree::batch::Batch;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use solana_sdk::account::Account;
use solana_sdk::pubkey::Pubkey;
use tracing::info;

use super::batch_utils;
use super::shared_state::SharedTreeState;

/// Syncs coordinator state with on-chain merkle tree data.
///
/// This function:
/// 1. Extracts the current on-chain root
/// 2. Checks if root has changed
/// 3. Resets shared state with new root and batch data
///
/// # Arguments
/// * `shared_state` - Shared coordinator state to update
/// * `on_chain_root` - Current on-chain merkle tree root
/// * `tree_batches` - Batch data from the merkle tree
/// * `output_queue_batches` - Batch data from output queue (if applicable)
///
/// # Returns
/// `true` if the root changed, `false` otherwise
pub async fn sync_coordinator_state(
    shared_state: &tokio::sync::RwLock<SharedTreeState>,
    on_chain_root: [u8; 32],
    tree_batches: &[Batch; 2],
    output_queue_batches: &[Batch; 2],
) -> Result<bool> {
    let mut state = shared_state.write().await;
    info!("Syncing: on-chain root = {:?}", &on_chain_root[..8]);
    let root_changed = state.current_root != on_chain_root;

    state.reset(on_chain_root, tree_batches, output_queue_batches);

    Ok(root_changed)
}

/// Extracts on-chain root and batch data from a merkle tree account for address trees.
///
/// # Arguments
/// * `merkle_tree_account` - The account containing merkle tree data
/// * `merkle_tree_pubkey` - Public key of the merkle tree
///
/// # Returns
/// Tuple of (on_chain_root, tree_batches)
pub fn extract_address_tree_sync_data(
    merkle_tree_account: &mut Account,
    merkle_tree_pubkey: &Pubkey,
) -> Result<([u8; 32], [Batch; 2])> {
    let tree_data = BatchedMerkleTreeAccount::address_from_bytes(
        merkle_tree_account.data.as_mut_slice(),
        &(*merkle_tree_pubkey).into(),
    )?;

    let on_chain_root = batch_utils::extract_current_root(&tree_data)?;
    let tree_batches = tree_data.queue_batches.batches;

    Ok((on_chain_root, tree_batches))
}

/// Extracts on-chain root and batch data from a merkle tree account for state trees.
///
/// # Arguments
/// * `merkle_tree_account` - The account containing merkle tree data
/// * `merkle_tree_pubkey` - Public key of the merkle tree
///
/// # Returns
/// Tuple of (on_chain_root, tree_batches)
pub fn extract_state_tree_sync_data(
    merkle_tree_account: &mut Account,
    merkle_tree_pubkey: &Pubkey,
) -> Result<([u8; 32], [Batch; 2])> {
    let tree_data = BatchedMerkleTreeAccount::state_from_bytes(
        merkle_tree_account.data.as_mut_slice(),
        &(*merkle_tree_pubkey).into(),
    )?;

    let on_chain_root = batch_utils::extract_current_root(&tree_data)?;
    let tree_batches = tree_data.queue_batches.batches;

    Ok((on_chain_root, tree_batches))
}

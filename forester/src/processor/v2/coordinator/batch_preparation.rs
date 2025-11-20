/// Batch preparation logic for computing circuit inputs.
use anyhow::Result;
use light_batched_merkle_tree::constants::DEFAULT_BATCH_STATE_TREE_HEIGHT;
use light_hasher::{hash_chain::create_hash_chain_from_slice, Hasher, Poseidon};
use light_prover_client::proof_types::{
    batch_append::{get_batch_append_inputs_v2, BatchAppendsCircuitInputs},
    batch_update::{get_batch_update_inputs_v2, BatchUpdateCircuitInputs},
};
use tracing::{error, trace};

use super::{
    error::CoordinatorError,
    types::{AppendQueueData, NullifyQueueData, PreparationState},
};

/// Prepares a single append batch for proof generation.
///
/// This function:
/// 1. Extracts batch-specific data (leaves, proofs, etc.)
/// 2. Generates circuit inputs (proofs are already current - no adjustment needed)
/// 3. Updates the tree state immediately after each leaf
pub fn prepare_append_batch(
    append_data: &AppendQueueData,
    state: &mut PreparationState,
) -> Result<BatchAppendsCircuitInputs> {
    let batch_start = std::time::Instant::now();
    let batch_idx = state.append_batch_index;
    let leaves_hash_chain = append_data.leaves_hash_chains[batch_idx];
    let start_idx = batch_idx * append_data.zkp_batch_size as usize;
    let end_idx = start_idx + append_data.zkp_batch_size as usize;

    // Extract batch-specific data
    let batch_leaf_indices = &state.append_leaf_indices[start_idx..end_idx];
    let adjusted_start_index = batch_leaf_indices[0] as u32;
    let batch_elements = &append_data.queue_elements[start_idx..end_idx];

    // Gather leaves from batch elements
    let leaves: Vec<[u8; 32]> = batch_elements
        .iter()
        .map(|elem| elem.account_hash)
        .collect();

    let old_root = state.staging.current_root();
    tracing::debug!(
        "prepare_append_batch (batch {}): Retrieved old_root={:?} from staging.current_root()",
        batch_idx,
        &old_root[..8]
    );

    // For v2: Get proofs and update tree iteratively (each proof depends on previous updates)
    let proof_start = std::time::Instant::now();
    let mut merkle_proofs = Vec::with_capacity(batch_leaf_indices.len());
    let mut old_leaves = Vec::with_capacity(batch_leaf_indices.len());

    for (i, &leaf_idx) in batch_leaf_indices.iter().enumerate() {
        // Get proof and old leaf from current tree state
        merkle_proofs.push(state.staging.get_proof(leaf_idx)?);
        old_leaves.push(state.staging.get_leaf(leaf_idx));

        // Update tree with new leaf so next proof will be adjusted
        state.staging.update_leaf(leaf_idx, leaves[i])?;
    }
    let proof_time = proof_start.elapsed();
    let new_root = state.staging.current_root();

    let circuit_start = std::time::Instant::now();
    let circuit_inputs =
        get_batch_append_inputs_v2::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
            old_root,
            adjusted_start_index,
            leaves.clone(),
            leaves_hash_chain,
            old_leaves,
            merkle_proofs,
            append_data.zkp_batch_size as u32,
            new_root,
        )?;
    let circuit_time = circuit_start.elapsed();

    let update_time = proof_time; // Updates are now part of proof gathering

    state.append_batch_index += 1;

    let total_time = batch_start.elapsed();
    trace!(
        "Prepared append batch {}: new_root={:?}, {} leaves | TIMING: total={:?} proof={:?} circuit={:?} update={:?}",
        batch_idx,
        &new_root[..8],
        leaves.len(),
        total_time,
        proof_time,
        circuit_time,
        update_time
    );

    Ok(circuit_inputs)
}

/// Prepares a single nullify batch for proof generation.
///
/// This function:
/// 1. Extracts batch-specific data (leaves, tx hashes, indices)
/// 2. Gets current leaf values from tree (already includes any prior updates)
/// 3. Computes nullifiers and validates hash chain
/// 4. Generates circuit inputs and updates tree immediately
pub fn prepare_nullify_batch(
    nullify_data: &NullifyQueueData,
    state: &mut PreparationState,
) -> Result<BatchUpdateCircuitInputs> {
    let batch_start = std::time::Instant::now();
    let batch_idx = state.nullify_batch_index;
    let leaves_hash_chain = nullify_data.leaves_hash_chains[batch_idx];
    let start_idx = batch_idx * nullify_data.zkp_batch_size as usize;
    let end_idx = start_idx + nullify_data.zkp_batch_size as usize;

    let batch_elements = &nullify_data.queue_elements[start_idx..end_idx];

    let mut leaves = Vec::new();
    let mut tx_hashes = Vec::new();
    let mut path_indices = Vec::new();

    // Gather basic data from batch elements (non-tree data)
    for element in batch_elements.iter() {
        leaves.push(element.account_hash);
        tx_hashes.push(element.tx_hash.unwrap_or([0u8; 32]));
        path_indices.push(element.leaf_index as u32);
    }

    validate_nullify_hash_chain(
        batch_idx,
        &leaves,
        &path_indices,
        &tx_hashes,
        leaves_hash_chain,
    )?;

    let old_root = state.staging.current_root();

    // For v2: Get proofs and update tree iteratively (each proof depends on previous updates)
    let proof_start = std::time::Instant::now();
    let mut merkle_proofs = Vec::with_capacity(batch_elements.len());
    let mut old_leaves = Vec::with_capacity(batch_elements.len());

    for (i, element) in batch_elements.iter().enumerate() {
        // Get proof and old leaf from current tree state
        merkle_proofs.push(state.staging.get_proof(element.leaf_index)?);
        old_leaves.push(state.staging.get_leaf(element.leaf_index));

        // Compute nullifier: hash(account_hash, path_index, tx_hash)
        let index_bytes = path_indices[i].to_be_bytes();
        let nullifier = Poseidon::hashv(&[&leaves[i], &index_bytes[..], &tx_hashes[i]])
            .map_err(|e| anyhow::anyhow!("Failed to compute nullifier: {}", e))?;

        // Update tree with nullifier so next proof will be adjusted
        state.staging.update_leaf(element.leaf_index, nullifier)?;
    }
    let proof_time = proof_start.elapsed();
    let new_root = state.staging.current_root();

    // Generate circuit inputs
    let circuit_start = std::time::Instant::now();
    let circuit_inputs =
        get_batch_update_inputs_v2::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
            old_root,
            tx_hashes.clone(),
            leaves.clone(),
            leaves_hash_chain,
            old_leaves,
            merkle_proofs,
            path_indices,
            nullify_data.zkp_batch_size as u32,
            new_root,
        )?;
    let circuit_time = circuit_start.elapsed();

    let update_time = proof_time; // Updates are now part of proof gathering

    state.nullify_batch_index += 1;

    let total_time = batch_start.elapsed();
    trace!(
        "Prepared nullify batch {}: new_root={:?}, {} leaves | TIMING: total={:?} proof={:?} circuit={:?} update={:?}",
        batch_idx,
        &new_root[..8],
        leaves.len(),
        total_time,
        proof_time,
        circuit_time,
        update_time
    );

    Ok(circuit_inputs)
}

/// Validates that the computed nullifier hash chain matches the on-chain value.
fn validate_nullify_hash_chain(
    batch_idx: usize,
    leaves: &[[u8; 32]],
    path_indices: &[u32],
    tx_hashes: &[[u8; 32]],
    expected_hash_chain: [u8; 32],
) -> Result<()> {
    let mut computed_nullifiers = Vec::new();

    for (idx, &account_hash) in leaves.iter().enumerate() {
        let index_bytes = path_indices[idx].to_be_bytes();
        let nullifier = Poseidon::hashv(&[&account_hash, &index_bytes[..], &tx_hashes[idx]])
            .map_err(|e| anyhow::anyhow!("Failed to compute nullifier: {}", e))?;
        computed_nullifiers.push(nullifier);
    }

    let computed_hashchain = create_hash_chain_from_slice(&computed_nullifiers)
        .map_err(|e| anyhow::anyhow!("Failed to compute nullify hashchain: {}", e))?;

    if computed_hashchain != expected_hash_chain {
        error!(
            "Hash chain validation failed!\n\
             On-chain hash chain: {:?}\n\
             Computed hash chain: {:?}\n\
             Batch index: {}, Total elements in batch: {}\n\
             All account_hashes: {:?}\n\
             All path_indices: {:?}\n\
             All tx_hashes: {:?}\n\
             All computed nullifiers: {:?}",
            expected_hash_chain,
            computed_hashchain,
            batch_idx,
            leaves.len(),
            leaves.iter().map(|h| &h[..8]).collect::<Vec<_>>(),
            path_indices,
            tx_hashes.iter().map(|h| &h[..8]).collect::<Vec<_>>(),
            computed_nullifiers
                .iter()
                .map(|h| &h[..8])
                .collect::<Vec<_>>()
        );

        let mut expected = [0u8; 8];
        let mut computed = [0u8; 8];
        expected.copy_from_slice(&expected_hash_chain[..8]);
        computed.copy_from_slice(&computed_hashchain[..8]);

        return Err(CoordinatorError::HashChainMismatch {
            batch_index: batch_idx,
            expected,
            computed,
        }
        .into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_chain_validation_matches() {
        let leaves = vec![[1u8; 32], [2u8; 32]];
        let path_indices = vec![0u32, 1u32];
        let tx_hashes = vec![[0u8; 32], [0u8; 32]];

        // Compute expected hash chain
        let mut nullifiers = Vec::new();
        for (idx, &account_hash) in leaves.iter().enumerate() {
            let index_bytes = path_indices[idx].to_be_bytes();
            let nullifier =
                Poseidon::hashv(&[&account_hash, &index_bytes[..], &tx_hashes[idx]]).unwrap();
            nullifiers.push(nullifier);
        }
        let expected = create_hash_chain_from_slice(&nullifiers).unwrap();

        // Should pass validation
        let result = validate_nullify_hash_chain(0, &leaves, &path_indices, &tx_hashes, expected);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hash_chain_validation_mismatch() {
        let leaves = vec![[1u8; 32], [2u8; 32]];
        let path_indices = vec![0u32, 1u32];
        let tx_hashes = vec![[0u8; 32], [0u8; 32]];
        let wrong_hash_chain = [99u8; 32];

        // Should fail validation
        let result =
            validate_nullify_hash_chain(0, &leaves, &path_indices, &tx_hashes, wrong_hash_chain);
        assert!(result.is_err());
    }
}

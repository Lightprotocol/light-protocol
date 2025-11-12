/// Batch preparation logic for computing circuit inputs.
use anyhow::Result;
use light_batched_merkle_tree::constants::DEFAULT_BATCH_STATE_TREE_HEIGHT;
use light_hasher::{
    bigint::bigint_to_be_bytes_array, hash_chain::create_hash_chain_from_slice, Hasher, Poseidon,
};
use light_prover_client::proof_types::{
    batch_append::{get_batch_append_inputs, BatchAppendsCircuitInputs},
    batch_update::{get_batch_update_inputs, BatchUpdateCircuitInputs},
};
use tracing::{debug, error, trace};

use super::{
    error::CoordinatorError,
    tree_state::encode_node_index,
    types::{AppendQueueData, NullifyQueueData, PreparationState},
};

/// Prepares a single append batch for proof generation.
///
/// This function:
/// 1. Extracts batch-specific data (leaves, proofs, etc.)
/// 2. Computes circuit inputs using accumulated changelogs
/// 3. Updates the tree state with new changelogs
/// 4. Tracks leaf modifications for later nullify batches
pub fn prepare_append_batch(
    append_data: &AppendQueueData,
    state: &mut PreparationState,
) -> Result<BatchAppendsCircuitInputs> {
    let batch_idx = state.append_batch_index;
    let leaves_hash_chain = append_data.leaves_hash_chains[batch_idx];
    let start_idx = batch_idx * append_data.zkp_batch_size as usize;
    let end_idx = start_idx + append_data.zkp_batch_size as usize;

    // Extract batch-specific data
    let batch_leaf_indices = &state.append_leaf_indices[start_idx..end_idx];
    let adjusted_start_index = batch_leaf_indices[0] as u32;
    let batch_elements = &append_data.queue_elements[start_idx..end_idx];

    // Gather leaves and proofs
    let leaves: Vec<[u8; 32]> = batch_elements
        .iter()
        .map(|elem| elem.account_hash)
        .collect();

    let merkle_proofs: Vec<Vec<[u8; 32]>> = batch_leaf_indices
        .iter()
        .map(|&idx| state.tree_state.get_proof(idx))
        .collect::<Result<Vec<_>>>()?;

    let old_leaves: Vec<[u8; 32]> = batch_leaf_indices
        .iter()
        .map(|&idx| {
            let node_idx = encode_node_index(0, idx);
            *state.tree_state.nodes.get(&node_idx).unwrap_or(&[0u8; 32])
        })
        .collect();

    // Generate circuit inputs with accumulated changelogs
    let (circuit_inputs, batch_changelogs) =
        get_batch_append_inputs::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
            state.current_root,
            adjusted_start_index,
            leaves.clone(),
            leaves_hash_chain,
            old_leaves,
            merkle_proofs,
            append_data.zkp_batch_size as u32,
            &state.accumulated_changelogs,
        )?;

    // Update state with new root
    let new_root = bigint_to_be_bytes_array::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
        &circuit_inputs.new_root.to_biguint().unwrap(),
    )?;

    state
        .accumulated_changelogs
        .extend(batch_changelogs.iter().cloned());
    state.current_root = new_root;

    // Track leaf modifications for potential nullify batches
    for (i, changelog) in batch_changelogs.iter().enumerate() {
        let new_leaf = leaves[i];
        let tree_index = changelog.index();
        state
            .tree_state
            .track_leaf_modification(tree_index, new_leaf);
    }

    state.append_batch_index += 1;

    trace!(
        "Prepared append batch {}: new_root={:?}, {} changelogs",
        batch_idx,
        &new_root[..8],
        batch_changelogs.len()
    );

    Ok(circuit_inputs)
}

/// Prepares a single nullify batch for proof generation.
///
/// This function:
/// 1. Extracts batch-specific data (leaves, tx hashes, indices)
/// 2. Uses modified leaves from earlier append batches if available
/// 3. Computes nullifiers and validates hash chain
/// 4. Generates circuit inputs with accumulated changelogs
/// 5. Updates the tree state with new changelogs
pub fn prepare_nullify_batch(
    nullify_data: &NullifyQueueData,
    state: &mut PreparationState,
) -> Result<BatchUpdateCircuitInputs> {
    let batch_idx = state.nullify_batch_index;
    let leaves_hash_chain = nullify_data.leaves_hash_chains[batch_idx];
    let start_idx = batch_idx * nullify_data.zkp_batch_size as usize;
    let end_idx = start_idx + nullify_data.zkp_batch_size as usize;

    let batch_elements = &nullify_data.queue_elements[start_idx..end_idx];

    let mut leaves = Vec::new();
    let mut tx_hashes = Vec::new();
    let mut old_leaves = Vec::new();
    let mut path_indices = Vec::new();

    // Gather data, checking for modified leaves from append batches
    for element in batch_elements.iter() {
        let leaf_idx = element.leaf_index;
        let tree_index = leaf_idx as usize;

        leaves.push(element.account_hash);
        tx_hashes.push(element.tx_hash.unwrap_or([0u8; 32]));
        path_indices.push(leaf_idx as u32);

        // Use modified leaf from earlier append if available
        let old_leaf = if let Some(modified) = state.tree_state.get_modified_leaf(tree_index) {
            debug!(
                "Using modified leaf value at index {} from earlier append",
                leaf_idx
            );
            modified
        } else {
            let node_idx = encode_node_index(0, leaf_idx);
            state
                .tree_state
                .nodes
                .get(&node_idx)
                .copied()
                .unwrap_or([0u8; 32])
        };
        old_leaves.push(old_leaf);
    }

    let merkle_proofs: Vec<Vec<[u8; 32]>> = batch_elements
        .iter()
        .map(|elem| state.tree_state.get_proof(elem.leaf_index))
        .collect::<Result<Vec<_>>>()?;

    // Validate hash chain
    validate_nullify_hash_chain(
        batch_idx,
        &leaves,
        &path_indices,
        &tx_hashes,
        leaves_hash_chain,
    )?;

    // Generate circuit inputs with accumulated changelogs
    let (circuit_inputs, batch_changelog) =
        get_batch_update_inputs::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
            state.current_root,
            tx_hashes.clone(),
            leaves.clone(),
            leaves_hash_chain,
            old_leaves,
            merkle_proofs,
            path_indices,
            nullify_data.zkp_batch_size as u32,
            &state.accumulated_changelogs,
        )?;

    // Update state with new root
    let new_root = bigint_to_be_bytes_array::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
        &circuit_inputs.new_root.to_biguint().unwrap(),
    )?;

    state
        .accumulated_changelogs
        .extend(batch_changelog.iter().cloned());
    state.current_root = new_root;
    state.nullify_batch_index += 1;

    trace!(
        "Prepared nullify batch {}: new_root={:?}, {} changelogs",
        batch_idx,
        &new_root[..8],
        batch_changelog.len()
    );

    Ok(circuit_inputs)
}

/// Validates that the computed nullifier hash chain matches the on-chain value.
///
/// This is critical for ensuring that the circuit will not fail constraint validation.
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

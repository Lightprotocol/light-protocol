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

/// Prepares a single append batch for proof generation using staging tree.
///
/// This function:
/// 1. Creates a staging tree overlay for incremental proof generation
/// 2. Generates proofs incrementally as leaves are updated in the staging tree
/// 3. Commits all updates to the main tree state at the end
pub fn prepare_append_batch(
    append_data: &AppendQueueData,
    state: &mut PreparationState,
) -> Result<BatchAppendsCircuitInputs> {
    let batch_start = std::time::Instant::now();
    let batch_idx = state.append_batch_index;
    let leaves_hash_chain = append_data.leaves_hash_chains[batch_idx];
    let start_idx = batch_idx * append_data.zkp_batch_size as usize;
    let end_idx = start_idx + append_data.zkp_batch_size as usize;

    let batch_leaf_indices = &state.append_leaf_indices[start_idx..end_idx];
    let adjusted_start_index = batch_leaf_indices[0] as u32;
    let batch_elements = &append_data.queue_elements[start_idx..end_idx];

    // Debug: verify indices are consecutive
    for (i, &idx) in batch_leaf_indices.iter().enumerate() {
        let expected = adjusted_start_index as u64 + i as u64;
        if idx != expected {
            error!(
                "NON-CONSECUTIVE INDICES in batch {}: batch_leaf_indices[{}]={}, expected={}",
                batch_idx, i, idx, expected
            );
        }
    }
    trace!(
        "Append batch {} indices: start={}, indices={:?}",
        batch_idx,
        adjusted_start_index,
        batch_leaf_indices
    );

    let leaves: Vec<[u8; 32]> = batch_elements
        .iter()
        .map(|elem| elem.account_hash)
        .collect();

    let staging_start = std::time::Instant::now();
    // Use state.current_root as the old_root - this tracks the root BEFORE this batch's updates.
    // After the first batch, state.staging.current_root() would already include previous updates,
    // but state.current_root tracks the starting point for THIS batch.
    let old_root = state.current_root;
    let staging_time = staging_start.elapsed();

    let proof_start = std::time::Instant::now();
    let mut merkle_proofs = Vec::new();
    let mut old_leaves = Vec::new();

    for (i, &idx) in batch_leaf_indices.iter().enumerate() {
        let proof = state.staging.get_proof(idx)?;
        let old_leaf = state.staging.get_leaf(idx);

        trace!(
            "  proof[{}] at index {}: old_leaf={:?}, proof_len={}, first_proof_elem={:?}",
            i,
            idx,
            &old_leaf[..8],
            proof.len(),
            if !proof.is_empty() {
                &proof[0][..8]
            } else {
                &[0u8; 8]
            }
        );

        merkle_proofs.push(proof);
        old_leaves.push(old_leaf);

        // Update for next iteration
        state.staging.update_leaf(idx, leaves[i])?;
    }

    // Get the new root from the staging tree after all updates
    let new_root = state.staging.current_root();
    let proof_time = proof_start.elapsed();

    trace!(
        "Append batch {} circuit inputs: old_root={:?}, new_root={:?}, start_index={}, num_leaves={}",
        batch_idx,
        &state.current_root[..8],
        &new_root[..8],
        adjusted_start_index,
        leaves.len()
    );
    for (i, (old_leaf, new_leaf)) in old_leaves.iter().zip(leaves.iter()).enumerate() {
        trace!(
            "  leaf[{}]: old={:?}, new={:?}",
            i,
            &old_leaf[..8],
            &new_leaf[..8]
        );
    }

    let circuit_start = std::time::Instant::now();
    let circuit_inputs = get_batch_append_inputs_v2::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
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

    let update_start = std::time::Instant::now();
    state.current_root = new_root;
    let update_time = update_start.elapsed();

    state.append_batch_index += 1;

    let total_time = batch_start.elapsed();
    trace!(
        "Prepared append batch {}: new_root={:?}, {} leaves | TIMING: total={:?} staging={:?} proof={:?} circuit={:?} update={:?}",
        batch_idx,
        &new_root[..8],
        leaves.len(),
        total_time,
        staging_time,
        proof_time,
        circuit_time,
        update_time
    );

    Ok(circuit_inputs)
}

/// Prepares a single nullify batch for proof generation using staging tree.
///
/// This function:
/// 1. Creates a staging tree overlay for incremental proof generation
/// 2. Validates the nullifier hash chain
/// 3. Generates proofs incrementally as nullifiers are computed in the staging tree
/// 4. Commits all updates to the main tree state at the end
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

    for element in batch_elements.iter() {
        leaves.push(element.account_hash);
        tx_hashes.push(element.tx_hash.unwrap_or([0u8; 32]));
        path_indices.push(element.leaf_index as u32);
    }

    // Compute all nullifiers first (needed for validation and tree updates)
    let mut computed_nullifiers = Vec::with_capacity(batch_elements.len());
    for idx in 0..batch_elements.len() {
        let index_bytes = path_indices[idx].to_be_bytes();
        let nullifier = Poseidon::hashv(&[&leaves[idx], &index_bytes[..], &tx_hashes[idx]])
            .map_err(|e| anyhow::anyhow!("Failed to compute nullifier: {}", e))?;
        computed_nullifiers.push(nullifier);
    }

    validate_nullify_hash_chain(
        batch_idx,
        &leaves,
        &path_indices,
        &tx_hashes,
        leaves_hash_chain,
    )?;

    let staging_start = std::time::Instant::now();
    
    let old_root = state.current_root;
    let staging_time = staging_start.elapsed();

    let proof_start = std::time::Instant::now();
    let mut merkle_proofs = Vec::new();
    let mut old_leaves = Vec::new();

    
    for (i, element) in batch_elements.iter().enumerate() {
        let idx = element.leaf_index;

        let proof = state.staging.get_proof(idx)?;
        let old_leaf = state.staging.get_leaf(idx);

        merkle_proofs.push(proof);
        old_leaves.push(old_leaf);

        // Update with nullifier for next iteration
        state.staging.update_leaf(idx, computed_nullifiers[i])?;
    }

    // Get the new root from the staging tree after all updates
    let new_root = state.staging.current_root();
    let proof_time = proof_start.elapsed();

    let circuit_start = std::time::Instant::now();
    let circuit_inputs = get_batch_update_inputs_v2::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
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

    let update_start = std::time::Instant::now();
    state.current_root = new_root;
    let update_time = update_start.elapsed();

    state.nullify_batch_index += 1;

    let total_time = batch_start.elapsed();
    trace!(
        "Prepared nullify batch {}: new_root={:?}, {} leaves | TIMING: total={:?} staging={:?} proof={:?} circuit={:?} update={:?}",
        batch_idx,
        &new_root[..8],
        leaves.len(),
        total_time,
        staging_time,
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

        let mut nullifiers = Vec::new();
        for (idx, &account_hash) in leaves.iter().enumerate() {
            let index_bytes = path_indices[idx].to_be_bytes();
            let nullifier =
                Poseidon::hashv(&[&account_hash, &index_bytes[..], &tx_hashes[idx]]).unwrap();
            nullifiers.push(nullifier);
        }
        let expected = create_hash_chain_from_slice(&nullifiers).unwrap();

        let result = validate_nullify_hash_chain(0, &leaves, &path_indices, &tx_hashes, expected);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hash_chain_validation_mismatch() {
        let leaves = vec![[1u8; 32], [2u8; 32]];
        let path_indices = vec![0u32, 1u32];
        let tx_hashes = vec![[0u8; 32], [0u8; 32]];
        let wrong_hash_chain = [99u8; 32];

        let result =
            validate_nullify_hash_chain(0, &leaves, &path_indices, &tx_hashes, wrong_hash_chain);
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_append_v1_vs_v2() {
        use light_hasher::{Hasher, Poseidon};
        use light_merkle_tree_reference::MerkleTree;
        use light_prover_client::proof_types::batch_append::{
            get_batch_append_inputs, get_batch_append_inputs_v2,
        };

        const HEIGHT: usize = 26;
        const BATCH_SIZE: usize = 10;

        // Create a fresh merkle tree
        let mut tree = MerkleTree::<Poseidon>::new(HEIGHT, 0);
        let start_index = 0u32;

        // Create test leaves
        let mut leaves = Vec::new();
        for i in 0..BATCH_SIZE {
            let mut bn = [0u8; 32];
            bn[31] = i as u8;
            let leaf = Poseidon::hash(&bn).unwrap();
            leaves.push(leaf);
        }

        // Get merkle proofs for all positions (all should be zero since tree is empty)
        let mut merkle_proofs = Vec::new();
        let mut old_leaves = Vec::new();
        for i in 0..BATCH_SIZE {
            let proof = tree.get_proof_of_leaf(i, true).unwrap();
            let old_leaf = tree.leaf(i);
            merkle_proofs.push(proof.to_vec());
            old_leaves.push(old_leaf);
        }

        let root = tree.root();
        let leaves_hashchain = create_hash_chain_from_slice(&leaves).unwrap();

        // Test v1
        let (v1_inputs, _changelogs) = get_batch_append_inputs::<HEIGHT>(
            root,
            start_index,
            leaves.clone(),
            leaves_hashchain,
            old_leaves.clone(),
            merkle_proofs.clone(),
            BATCH_SIZE as u32,
            &[], // no previous changelogs
        )
        .unwrap();

        // Apply updates to tree to get actual new_root
        for (i, &leaf) in leaves.iter().enumerate() {
            tree.append(&leaf).unwrap();
        }
        let actual_new_root = tree.root();

        // Test v2
        let v2_inputs = get_batch_append_inputs_v2::<HEIGHT>(
            root,
            start_index,
            leaves.clone(),
            leaves_hashchain,
            old_leaves,
            merkle_proofs,
            BATCH_SIZE as u32,
            actual_new_root,
        )
        .unwrap();

        // Compare
        println!(
            "V1 old_root: {:?}",
            &v1_inputs.old_root.to_bytes_be().1[..8]
        );
        println!(
            "V2 old_root: {:?}",
            &v2_inputs.old_root.to_bytes_be().1[..8]
        );
        println!(
            "V1 new_root: {:?}",
            &v1_inputs.new_root.to_bytes_be().1[..8]
        );
        println!(
            "V2 new_root: {:?}",
            &v2_inputs.new_root.to_bytes_be().1[..8]
        );
        println!("Actual new_root: {:?}", &actual_new_root[..8]);

        assert_eq!(v1_inputs.old_root, v2_inputs.old_root, "old_root mismatch");
        assert_eq!(v1_inputs.new_root, v2_inputs.new_root, "new_root mismatch");

        // Verify v1's computed root matches actual
        let v1_root_bytes = v1_inputs.new_root.to_bytes_be().1;
        assert_eq!(
            &v1_root_bytes[..],
            &actual_new_root[..],
            "v1 root doesn't match actual tree root"
        );
    }
}

use light_hasher::hash_chain::create_hash_chain_from_array;
use light_sparse_merkle_tree::changelog::ChangelogEntry;
use num_bigint::{BigInt, Sign};
use serde::Serialize;
use tracing::{error, info};

use crate::{
    errors::ProverClientError,
    helpers::bigint_to_u8_32,
};

#[derive(Debug, Clone, Serialize)]
pub struct BatchAppendsCircuitInputs {
    pub public_input_hash: BigInt,
    pub old_root: BigInt,
    pub new_root: BigInt,
    pub leaves_hashchain_hash: BigInt,
    pub start_index: u32,
    pub old_leaves: Vec<BigInt>,
    pub leaves: Vec<BigInt>,
    pub merkle_proofs: Vec<Vec<BigInt>>,
    pub height: u32,
    pub batch_size: u32,
}

impl BatchAppendsCircuitInputs {
    pub fn public_inputs_arr(&self) -> [u8; 32] {
        bigint_to_u8_32(&self.public_input_hash).unwrap()
    }
}

#[allow(clippy::too_many_arguments)]
pub fn get_batch_append_inputs<const HEIGHT: usize>(
    // get this from Merkle tree account
    current_root: [u8; 32],
    // get this from Merkle tree account
    start_index: u32,
    // get this from output queue account
    leaves: Vec<[u8; 32]>,
    // get this from output queue account
    leaves_hashchain: [u8; 32],
    // get old_leaves and merkle_proofs from indexer by requesting Merkle proofs
    // by indices
    old_leaves: Vec<[u8; 32]>,
    merkle_proofs: Vec<Vec<[u8; 32]>>,
    batch_size: u32,
    previous_changelogs: &[ChangelogEntry<HEIGHT>],
) -> Result<(BatchAppendsCircuitInputs, Vec<ChangelogEntry<HEIGHT>>), ProverClientError> {
    use std::collections::HashMap;

    let mut new_root = [0u8; 32];
    let mut circuit_merkle_proofs = Vec::with_capacity(batch_size as usize);

    // Create cache for parent node computations across all leaves in this batch
    // Cache key: (level, parent_position, left_hash, right_hash) -> parent_hash
    let mut root_computation_cache: HashMap<(usize, u32, [u8; 32], [u8; 32]), [u8; 32]> =
        HashMap::new();

    // Phase 1: Adjust all proofs and collect data
    let mut adjusted_proofs = Vec::with_capacity(batch_size as usize);
    let mut final_leaves = Vec::with_capacity(batch_size as usize);
    let mut path_indices = Vec::with_capacity(batch_size as usize);
    let mut temp_changelog: Vec<ChangelogEntry<HEIGHT>> = Vec::new();

    for (i, (old_leaf, (new_leaf, mut merkle_proof))) in old_leaves
        .iter()
        .zip(leaves.iter().zip(merkle_proofs.into_iter()))
        .enumerate()
    {
        let current_index = start_index as usize + i;
        info!(
            leaf_index = current_index,
            batch_position = i,
            batch_size = batch_size,
            tree_height = HEIGHT,
            "Processing leaf for batch append"
        );

        for change_log_entry in previous_changelogs.iter() {
            if change_log_entry.index() == current_index {
                continue;
            }

            match change_log_entry.update_proof(current_index, &mut merkle_proof) {
                Ok(_) => {}
                Err(e) => {
                    error!("previous_changelogs: couldn't update proof for index {}: current_root: {:?}: {:?}", current_index, current_root, e);
                    return Err(ProverClientError::GenericError(format!(
                        "ProverClientError: couldn't update proof for index {}: {:?}",
                        current_index, e
                    )));
                }
            }
        }

        // Determine final leaf value
        let is_old_leaf_zero = old_leaf.iter().all(|&byte| byte == 0);
        let final_leaf = if is_old_leaf_zero { *new_leaf } else { *old_leaf };

        final_leaves.push(final_leaf);
        path_indices.push(start_index + i as u32);

        // Adjust proof using previously computed changelogs
        if i > 0 {
            for change_log_entry in temp_changelog.iter() {
                if change_log_entry.index() == current_index {
                    continue;
                }

                match change_log_entry.update_proof(current_index, &mut merkle_proof) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("current changelogs: couldn't update proof for index {}: current_root: {:?}: {:?}",
                            current_index,
                            current_root,
                            e);
                        return Err(ProverClientError::GenericError(format!(
                            "current changelogs: couldn't update proof for index {}: {:?}",
                            current_index, e
                        )));
                    }
                }
            }
        }

        // Compute root and changelog for this leaf with caching
        let merkle_proof_array: [[u8; 32]; HEIGHT] = merkle_proof.clone().try_into().unwrap();
        let (root, changelog_entry) = crate::helpers::compute_root_from_merkle_proof_with_cache(
            final_leaf,
            &merkle_proof_array,
            start_index + i as u32,
            Some(&mut root_computation_cache),
        );
        new_root = root;
        temp_changelog.push(changelog_entry.clone());

        adjusted_proofs.push(merkle_proof);
        circuit_merkle_proofs.push(
            merkle_proof_array
                .iter()
                .map(|hash| BigInt::from_bytes_be(Sign::Plus, hash))
                .collect(),
        );
    }

    // Log cache effectiveness
    let cache_size = root_computation_cache.len();
    let max_possible_hashes = batch_size as usize * HEIGHT;
    let hashes_computed = cache_size;
    let hashes_saved = max_possible_hashes.saturating_sub(hashes_computed);
    if hashes_saved > 0 {
        info!(
            "Batch append root computation: {} leaves, {} unique hashes computed, {} hashes saved via caching ({:.1}% reduction)",
            batch_size,
            hashes_computed,
            hashes_saved,
            (hashes_saved as f64 / max_possible_hashes as f64) * 100.0
        );
    }

    // Use the temp_changelog as the final changelog
    let changelog = temp_changelog;

    let mut start_index_bytes = [0u8; 32];
    start_index_bytes[28..].copy_from_slice(start_index.to_be_bytes().as_slice());
    // Calculate the public input hash chain with old root, new root, and leaves hash chain
    let public_input_hash = create_hash_chain_from_array([
        current_root,
        new_root,
        leaves_hashchain,
        start_index_bytes,
    ])?;
    Ok((
        BatchAppendsCircuitInputs {
            public_input_hash: BigInt::from_bytes_be(Sign::Plus, &public_input_hash),
            old_root: BigInt::from_bytes_be(Sign::Plus, &current_root),
            new_root: BigInt::from_bytes_be(Sign::Plus, &new_root),
            leaves_hashchain_hash: BigInt::from_bytes_be(Sign::Plus, &leaves_hashchain),
            start_index,
            old_leaves: old_leaves
                .iter()
                .map(|leaf| BigInt::from_bytes_be(Sign::Plus, leaf))
                .collect(),
            leaves: leaves
                .iter()
                .map(|leaf| BigInt::from_bytes_be(Sign::Plus, leaf))
                .collect(),
            merkle_proofs: circuit_merkle_proofs,
            height: HEIGHT as u32,
            batch_size,
        },
        changelog,
    ))
}

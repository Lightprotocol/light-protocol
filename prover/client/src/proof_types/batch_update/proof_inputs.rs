use ahash::AHashMap;
use light_hasher::{hash_chain::create_hash_chain_from_array, Hasher, Poseidon};
use light_sparse_merkle_tree::changelog::ChangelogEntry;
use num_bigint::{BigInt, Sign};

use crate::{errors::ProverClientError, helpers::bigint_to_u8_32};

/// Cache type for Merkle root computations
type MerkleRootCache = AHashMap<(usize, u32, [u8; 32], [u8; 32]), [u8; 32]>;

#[derive(Clone, Debug)]
pub struct BatchUpdateCircuitInputs {
    pub public_input_hash: BigInt,
    pub old_root: BigInt,
    pub new_root: BigInt,
    pub tx_hashes: Vec<BigInt>,
    pub leaves_hashchain_hash: BigInt,
    pub leaves: Vec<BigInt>,
    pub old_leaves: Vec<BigInt>,
    pub merkle_proofs: Vec<Vec<BigInt>>,
    pub path_indices: Vec<u32>,
    pub height: u32,
    pub batch_size: u32,
}

impl BatchUpdateCircuitInputs {
    pub fn public_inputs_arr(&self) -> [u8; 32] {
        bigint_to_u8_32(&self.public_input_hash).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct BatchUpdateInputs<'a>(pub &'a [BatchUpdateCircuitInputs]);

impl BatchUpdateInputs<'_> {
    pub fn public_inputs(&self) -> Vec<[u8; 32]> {
        // Concatenate all public inputs into a single flat vector
        vec![self.0[0].public_inputs_arr()]
    }
}

#[allow(clippy::too_many_arguments)]
pub fn get_batch_update_inputs<const HEIGHT: usize>(
    current_root: [u8; 32],
    tx_hashes: Vec<[u8; 32]>,
    leaves: Vec<[u8; 32]>,
    leaves_hashchain: [u8; 32],
    old_leaves: Vec<[u8; 32]>,
    merkle_proofs: Vec<Vec<[u8; 32]>>,
    path_indices: Vec<u32>,
    batch_size: u32,
    previous_changelogs: &[ChangelogEntry<HEIGHT>],
) -> Result<(BatchUpdateCircuitInputs, Vec<ChangelogEntry<HEIGHT>>), ProverClientError> {
    let mut new_root = [0u8; 32];
    let old_root = current_root;
    let mut changelog: Vec<ChangelogEntry<HEIGHT>> = Vec::with_capacity(leaves.len());
    let mut circuit_merkle_proofs = Vec::with_capacity(leaves.len());
    let mut adjusted_path_indices = Vec::with_capacity(leaves.len());

    // Create cache for parent node computations across all leaves in this batch
    let estimated_capacity = (batch_size as usize * HEIGHT) / 2;
    let mut root_computation_cache: MerkleRootCache = AHashMap::with_capacity(estimated_capacity);

    for (i, (leaf, (mut merkle_proof, index))) in leaves
        .iter()
        .zip(merkle_proofs.into_iter().zip(path_indices.iter()))
        .enumerate()
    {
        adjusted_path_indices.push(*index);

        for entry in previous_changelogs.iter() {
            if entry.index() == *index as usize {
                continue;
            }

            entry
                .update_proof(*index as usize, &mut merkle_proof)
                .map_err(|e| {
                    ProverClientError::GenericError(format!(
                        "Failed to update proof with previous changelog: {:?}",
                        e
                    ))
                })?;
        }
        // And update with current batch changelogs accumulated so far.
        if i > 0 {
            for entry in changelog.iter() {
                if entry.index() == *index as usize {
                    continue;
                }

                entry
                    .update_proof(*index as usize, &mut merkle_proof)
                    .map_err(|e| {
                        ProverClientError::GenericError(format!(
                            "Failed to update proof with previous changelog: {:?}",
                            e
                        ))
                    })?;
            }
        }

        let merkle_proof_array: [[u8; 32]; HEIGHT] =
            merkle_proof.as_slice().try_into().map_err(|_| {
                ProverClientError::GenericError("Invalid merkle proof length".to_string())
            })?;

        // Use the adjusted index bytes for computing the nullifier.
        let index_bytes = (*index).to_be_bytes();
        let nullifier = Poseidon::hashv(&[leaf, &index_bytes, &tx_hashes[i]]).unwrap();
        let (root, changelog_entry) = crate::helpers::compute_root_from_merkle_proof_with_cache(
            nullifier,
            &merkle_proof_array,
            *index,
            Some(&mut root_computation_cache),
        );
        new_root = root;
        changelog.push(changelog_entry);
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
        tracing::info!(
            "Batch update root computation: {} leaves, {} unique hashes computed, {} hashes saved via caching ({:.1}% reduction)",
            batch_size,
            hashes_computed,
            hashes_saved,
            (hashes_saved as f64 / max_possible_hashes as f64) * 100.0
        );
    }

    // Compute the public input hash as the chain of (old_root, new_root, leaves_hashchain)
    // (this must match the BatchUpdateCircuit in the prover, which expects exactly three inputs).
    let public_input_hash = create_hash_chain_from_array([old_root, new_root, leaves_hashchain])?;

    let inputs = BatchUpdateCircuitInputs {
        public_input_hash: BigInt::from_bytes_be(Sign::Plus, &public_input_hash),
        old_root: BigInt::from_bytes_be(Sign::Plus, &old_root),
        new_root: BigInt::from_bytes_be(Sign::Plus, &new_root),
        tx_hashes: tx_hashes
            .into_iter()
            .map(|tx| BigInt::from_bytes_be(Sign::Plus, &tx))
            .collect(),
        leaves_hashchain_hash: BigInt::from_bytes_be(Sign::Plus, &leaves_hashchain),
        leaves: leaves
            .into_iter()
            .map(|leaf| BigInt::from_bytes_be(Sign::Plus, &leaf))
            .collect(),
        old_leaves: old_leaves
            .into_iter()
            .map(|leaf| BigInt::from_bytes_be(Sign::Plus, &leaf))
            .collect(),
        merkle_proofs: circuit_merkle_proofs,
        path_indices: adjusted_path_indices,
        height: HEIGHT as u32,
        batch_size,
    };

    Ok((inputs, changelog))
}

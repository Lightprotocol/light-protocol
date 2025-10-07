use light_hasher::hash_chain::create_hash_chain_from_array;
use light_sparse_merkle_tree::changelog::ChangelogEntry;
use num_bigint::{BigInt, Sign};
use serde::Serialize;
use tracing::{error, info};

use crate::{
    errors::ProverClientError,
    helpers::{bigint_to_u8_32, compute_root_from_merkle_proof},
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
    let mut new_root = [0u8; 32];
    let mut changelog: Vec<ChangelogEntry<HEIGHT>> = Vec::new();
    let mut circuit_merkle_proofs = Vec::with_capacity(batch_size as usize);

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

        if i > 0 {
            for change_log_entry in changelog.iter() {
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

        let merkle_proof_array = merkle_proof.try_into().unwrap();
        // Determine if we use the old or new leaf based on whether the old leaf is nullified (zeroed).
        let is_old_leaf_zero = old_leaf.iter().all(|&byte| byte == 0);
        let final_leaf = if is_old_leaf_zero {
            *new_leaf
        } else {
            *old_leaf
        };

        // Update the root based on the current proof and nullifier
        let (updated_root, changelog_entry) =
            compute_root_from_merkle_proof(final_leaf, &merkle_proof_array, start_index + i as u32);
        new_root = updated_root;
        changelog.push(changelog_entry);
        circuit_merkle_proofs.push(
            merkle_proof_array
                .iter()
                .map(|hash| BigInt::from_bytes_be(Sign::Plus, hash))
                .collect(),
        );
    }

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

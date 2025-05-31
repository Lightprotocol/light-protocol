use light_hasher::{hash_chain::create_hash_chain_from_array, Hasher, Poseidon};
use light_sparse_merkle_tree::changelog::ChangelogEntry;
use num_bigint::{BigInt, Sign};

use crate::{
    errors::ProverClientError,
    helpers::{bigint_to_u8_32, compute_root_from_merkle_proof},
};

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
    let mut changelog: Vec<ChangelogEntry<HEIGHT>> = Vec::new();
    let mut circuit_merkle_proofs = vec![];
    let mut adjusted_path_indices = Vec::with_capacity(leaves.len());

    for (i, (leaf, (mut merkle_proof, index))) in leaves
        .iter()
        .zip(merkle_proofs.into_iter().zip(path_indices.iter()))
        .enumerate()
    {
        adjusted_path_indices.push(*index);

        // Update the proof with changelogs from previous batches.
        for entry in previous_changelogs.iter() {
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

        let merkle_proof_array = merkle_proof.try_into().unwrap();

        // Use the adjusted index bytes for computing the nullifier.
        let index_bytes = (*index).to_be_bytes();
        let nullifier = Poseidon::hashv(&[leaf, &index_bytes, &tx_hashes[i]]).unwrap();
        let (root, changelog_entry) =
            compute_root_from_merkle_proof(nullifier, &merkle_proof_array, *index);
        new_root = root;
        changelog.push(changelog_entry);
        circuit_merkle_proofs.push(
            merkle_proof_array
                .iter()
                .map(|hash| BigInt::from_bytes_be(Sign::Plus, hash))
                .collect(),
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

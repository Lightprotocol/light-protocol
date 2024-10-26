use crate::{batch_append::calculate_hash_chain, helpers::bigint_to_u8_32};
use light_bounded_vec::BoundedVec;
use light_concurrent_merkle_tree::changelog::ChangelogEntry;
use light_hasher::{Hasher, Poseidon};
use num_bigint::{BigInt, Sign};
use num_traits::FromBytes;

#[derive(Clone, Debug)]
pub struct BatchUpdateCircuitInputs {
    pub public_input_hash: BigInt,
    pub old_root: BigInt,
    pub new_root: BigInt,
    pub nullifiers: Vec<BigInt>,
    pub leaves_hashchain_hash: BigInt,
    pub leaves: Vec<BigInt>,
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

pub fn get_batch_update_inputs<const HEIGHT: usize>(
    // get from photon
    current_root: [u8; 32],
    // get from photon
    tx_hashes: Vec<[u8; 32]>,
    // get from photon
    leaves: Vec<[u8; 32]>,
    // get from account
    leaves_hashchain: [u8; 32],
    // get from photon
    merkle_proofs: Vec<Vec<[u8; 32]>>,
    // get from photon
    path_indices: Vec<u32>,
    // get from account (every mt account has a hardcoded batch size)
    batch_size: u32,
) -> BatchUpdateCircuitInputs {
    let mut new_root = [0u8; 32];
    let old_root = current_root;
    // We need a changelog because all subsequent proofs change after one update.
    // Hence we patch the proofs with the changelog.
    let mut changelog: Vec<ChangelogEntry<HEIGHT>> = Vec::new();
    let mut circuit_merkle_proofs = vec![];
    let mut nullifiers = vec![];
    for (i, (_leaf, (merkle_proof, index))) in leaves
        .iter()
        .zip(merkle_proofs.iter().zip(path_indices.iter()))
        .enumerate()
    {
        let mut bounded_vec_merkle_proof = BoundedVec::from_slice(merkle_proof.as_slice());

        if i > 0 {
            for change_log_entry in changelog.iter() {
                change_log_entry
                    .update_proof(*index as usize, &mut bounded_vec_merkle_proof)
                    .unwrap();
            }
        }

        let merkle_proof = bounded_vec_merkle_proof.to_array().unwrap();
        let nullifier = Poseidon::hashv(&[&leaves[i], &tx_hashes[i]]).unwrap();
        nullifiers.push(nullifier);
        let (root, changelog_entry) =
            comput_root_from_merkle_proof(nullifier, &merkle_proof, *index);
        new_root = root;

        changelog.push(changelog_entry);

        circuit_merkle_proofs.push(merkle_proof);
    }

    let public_input_hash = calculate_hash_chain(&[old_root, new_root, leaves_hashchain]);

    BatchUpdateCircuitInputs {
        public_input_hash: BigInt::from_be_bytes(&public_input_hash),
        old_root: BigInt::from_be_bytes(&old_root),
        new_root: BigInt::from_be_bytes(&new_root),
        nullifiers: nullifiers
            .iter()
            .map(|tx_hash| BigInt::from_bytes_be(Sign::Plus, tx_hash))
            .collect(),
        leaves_hashchain_hash: BigInt::from_be_bytes(&leaves_hashchain),
        leaves: leaves
            .iter()
            .map(|leaf| BigInt::from_bytes_be(Sign::Plus, leaf))
            .collect(),
        merkle_proofs: circuit_merkle_proofs
            .iter()
            .map(|proof| {
                proof
                    .iter()
                    .map(|hash| BigInt::from_bytes_be(Sign::Plus, hash))
                    .collect()
            })
            .collect(),
        path_indices,
        height: HEIGHT as u32,
        batch_size,
    }
}

pub fn comput_root_from_merkle_proof<const HEIGHT: usize>(
    leaf: [u8; 32],
    path_elements: &[[u8; 32]; HEIGHT],
    path_index: u32,
) -> ([u8; 32], ChangelogEntry<HEIGHT>) {
    let mut changelog_entry = ChangelogEntry::default_with_index(path_index as usize);

    let mut current_hash = leaf;
    let mut current_index = path_index;
    for (level, path_element) in path_elements.iter().enumerate() {
        changelog_entry.path[level] = Some(current_hash);
        if current_index % 2 == 0 {
            current_hash = Poseidon::hashv(&[&current_hash, path_element]).unwrap();
        } else {
            current_hash = Poseidon::hashv(&[path_element, &current_hash]).unwrap();
        }
        current_index /= 2;
    }

    (current_hash, changelog_entry)
}

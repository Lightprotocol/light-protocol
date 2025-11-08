use light_hasher::{
    bigint::bigint_to_be_bytes_array, hash_chain::create_hash_chain_from_array, Poseidon,
};
use light_indexed_array::{array::IndexedElement, changelog::RawIndexedElement};
use light_sparse_merkle_tree::{
    changelog::ChangelogEntry,
    indexed_changelog::{patch_indexed_changelogs, IndexedChangelogEntry},
    SparseMerkleTree,
};
use num_bigint::BigUint;

use crate::{errors::ProverClientError, helpers::compute_root_from_merkle_proof};

#[derive(Debug, Clone)]
pub struct BatchAddressAppendInputs {
    pub batch_size: usize,
    pub hashchain_hash: BigUint,
    pub low_element_values: Vec<BigUint>,
    pub low_element_indices: Vec<BigUint>,
    pub low_element_next_indices: Vec<BigUint>,
    pub low_element_next_values: Vec<BigUint>,
    pub low_element_proofs: Vec<Vec<BigUint>>,
    pub new_element_values: Vec<BigUint>,
    pub new_element_proofs: Vec<Vec<BigUint>>,
    pub new_root: BigUint,
    pub old_root: BigUint,
    pub public_input_hash: BigUint,
    pub start_index: usize,
    pub tree_height: usize,
}

#[allow(clippy::too_many_arguments)]
pub fn get_batch_address_append_circuit_inputs<const HEIGHT: usize>(
    next_index: usize,
    current_root: [u8; 32],
    low_element_values: Vec<[u8; 32]>,
    low_element_next_values: Vec<[u8; 32]>,
    low_element_indices: Vec<usize>,
    low_element_next_indices: Vec<usize>,
    low_element_proofs: Vec<Vec<[u8; 32]>>,
    new_element_values: Vec<[u8; 32]>,
    sparse_merkle_tree: &mut SparseMerkleTree<Poseidon, HEIGHT>,
    leaves_hashchain: [u8; 32],
    zkp_batch_size: usize,
    changelog: &mut Vec<ChangelogEntry<HEIGHT>>,
    indexed_changelog: &mut Vec<IndexedChangelogEntry<usize, HEIGHT>>,
) -> Result<BatchAddressAppendInputs, ProverClientError> {
    // 1. input all elements of a batch.
    // 2. iterate over elements 0..end_index
    // 3. only use elements start_index..end_index in the circuit (we need to
    // iterate over elements prior to start index to create changelog entries to
    // patch subsequent element proofs. The indexer won't be caught up yet.)
    let new_element_values = new_element_values[0..zkp_batch_size].to_vec();
    let mut new_root = [0u8; 32];
    let mut low_element_circuit_merkle_proofs = vec![];
    let mut new_element_circuit_merkle_proofs = vec![];

    let mut patched_low_element_next_values: Vec<[u8; 32]> = Vec::new();
    let mut patched_low_element_next_indices: Vec<usize> = Vec::new();
    let mut patched_low_element_values: Vec<[u8; 32]> = Vec::new();
    let mut patched_low_element_indices: Vec<usize> = Vec::new();

    for i in 0..new_element_values.len() {
        let mut changelog_index = 0;

        let new_element_index = next_index + i;
        let mut low_element: IndexedElement<usize> = IndexedElement {
            index: low_element_indices[i],
            value: BigUint::from_bytes_be(&low_element_values[i]),
            next_index: low_element_next_indices[i],
        };

        let mut new_element: IndexedElement<usize> = IndexedElement {
            index: new_element_index,
            value: BigUint::from_bytes_be(&new_element_values[i]),
            next_index: low_element_next_indices[i],
        };

        let mut low_element_proof = low_element_proofs[i].to_vec();
        let mut low_element_next_value = BigUint::from_bytes_be(&low_element_next_values[i]);
        patch_indexed_changelogs(
            0,
            &mut changelog_index,
            indexed_changelog,
            &mut low_element,
            &mut new_element,
            &mut low_element_next_value,
            &mut low_element_proof,
        )
        .unwrap();
        patched_low_element_next_values
            .push(bigint_to_be_bytes_array::<32>(&low_element_next_value).unwrap());
        patched_low_element_next_indices.push(low_element.next_index());
        patched_low_element_indices.push(low_element.index);
        patched_low_element_values
            .push(bigint_to_be_bytes_array::<32>(&low_element.value).unwrap());

        let new_low_element: IndexedElement<usize> = IndexedElement {
            index: low_element.index,
            value: low_element.value.clone(),
            next_index: new_element.index,
        };
        let new_low_element_raw = RawIndexedElement {
            value: bigint_to_be_bytes_array::<32>(&new_low_element.value).unwrap(),
            next_index: new_low_element.next_index,
            next_value: bigint_to_be_bytes_array::<32>(&new_element.value).unwrap(),
            index: new_low_element.index,
        };

        {
            for change_log_entry in changelog.iter().skip(changelog_index) {
                change_log_entry
                    .update_proof(low_element.index(), &mut low_element_proof)
                    .unwrap();
            }
            let merkle_proof = low_element_proof.clone().try_into().unwrap();
            let new_low_leaf_hash = new_low_element
                .hash::<Poseidon>(&new_element.value)
                .unwrap();
            let (_updated_root, changelog_entry) = compute_root_from_merkle_proof::<HEIGHT>(
                new_low_leaf_hash,
                &merkle_proof,
                new_low_element.index as u32,
            );
            changelog.push(changelog_entry);
            low_element_circuit_merkle_proofs.push(
                merkle_proof
                    .iter()
                    .map(|hash| BigUint::from_bytes_be(hash))
                    .collect(),
            );
        }
        let low_element_changelog_entry = IndexedChangelogEntry {
            element: new_low_element_raw,
            proof: low_element_proof.as_slice()[..HEIGHT].try_into().unwrap(),
            changelog_index: indexed_changelog.len(), //change_log_index,
        };

        indexed_changelog.push(low_element_changelog_entry);

        {
            let new_element_next_value = low_element_next_value;
            let new_element_leaf_hash = new_element
                .hash::<Poseidon>(&new_element_next_value)
                .unwrap();
            let mut merkle_proof_array = sparse_merkle_tree.append(new_element_leaf_hash);

            let current_index = next_index + i;

            for change_log_entry in changelog.iter() {
                change_log_entry
                    .update_proof(current_index, &mut merkle_proof_array)
                    .unwrap();
            }

            let (updated_root, changelog_entry) = compute_root_from_merkle_proof(
                new_element_leaf_hash,
                &merkle_proof_array,
                current_index as u32,
            );
            new_root = updated_root;

            changelog.push(changelog_entry);
            new_element_circuit_merkle_proofs.push(
                merkle_proof_array
                    .iter()
                    .map(|hash| BigUint::from_bytes_be(hash))
                    .collect(),
            );

            let new_element_raw = RawIndexedElement {
                value: bigint_to_be_bytes_array::<32>(&new_element.value).unwrap(),
                next_index: new_element.next_index,
                next_value: bigint_to_be_bytes_array::<32>(&new_element_next_value).unwrap(),
                index: new_element.index,
            };

            let new_element_changelog_entry = IndexedChangelogEntry {
                element: new_element_raw,
                proof: merkle_proof_array,
                changelog_index: indexed_changelog.len(),
            };
            indexed_changelog.push(new_element_changelog_entry);
        }
    }

    let hash_chain_inputs = [
        current_root,
        new_root,
        leaves_hashchain,
        bigint_to_be_bytes_array::<32>(&next_index.into()).unwrap(),
    ];

    for (idx, ((low_value, new_value), high_value)) in patched_low_element_values
        .iter()
        .zip(new_element_values.iter())
        .zip(patched_low_element_next_values.iter())
        .enumerate()
    {
        let low = BigUint::from_bytes_be(low_value);
        let new = BigUint::from_bytes_be(new_value);
        let high = BigUint::from_bytes_be(high_value);

        if !(low < new && new < high) {
            return Err(ProverClientError::GenericError(format!(
                "Invalid address ordering at batch position {} (low = {:#x}, new = {:#x}, high = {:#x})",
                idx, low, new, high
            )));
        }
    }

    let public_input_hash = create_hash_chain_from_array(hash_chain_inputs)?;

    Ok(BatchAddressAppendInputs {
        batch_size: patched_low_element_values.len(),
        hashchain_hash: BigUint::from_bytes_be(&leaves_hashchain),
        low_element_values: patched_low_element_values
            .iter()
            .map(|v| BigUint::from_bytes_be(v))
            .collect(),
        low_element_indices: patched_low_element_indices
            .iter()
            .map(|&i| BigUint::from(i))
            .collect(),
        low_element_next_indices: patched_low_element_next_indices
            .iter()
            .map(|&i| BigUint::from(i))
            .collect(),
        low_element_next_values: patched_low_element_next_values
            .iter()
            .map(|v| BigUint::from_bytes_be(v))
            .collect(),
        low_element_proofs: low_element_circuit_merkle_proofs,
        new_element_values: new_element_values[0..]
            .iter()
            .map(|v| BigUint::from_bytes_be(v))
            .collect(),
        new_element_proofs: new_element_circuit_merkle_proofs,
        new_root: BigUint::from_bytes_be(&new_root),
        old_root: BigUint::from_bytes_be(&current_root),
        public_input_hash: BigUint::from_bytes_be(&public_input_hash),
        start_index: next_index,
        tree_height: HEIGHT,
    })
}

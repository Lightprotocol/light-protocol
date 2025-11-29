use std::collections::HashMap;

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

/// Cache for proof updates - maps (level, node_index) to latest hash.
/// This allows O(HEIGHT) proof updates instead of O(changelog_size).
#[derive(Default)]
struct ProofCache {
    /// Maps (level, node_index_at_level) -> hash
    /// node_index_at_level = leaf_index >> level
    cache: HashMap<(usize, usize), [u8; 32]>,
}

impl ProofCache {
    /// Add a changelog entry to the cache.
    /// For each level, store the hash that would be used as a sibling.
    fn add_entry<const HEIGHT: usize>(&mut self, entry: &ChangelogEntry<HEIGHT>) {
        let index = entry.index();
        for level in 0..HEIGHT {
            if let Some(hash) = entry.path[level] {
                // Store the hash at the node's position at this level
                let node_index = index >> level;
                self.cache.insert((level, node_index), hash);
            }
        }
    }

    /// Update a proof using the cached values.
    /// For each level, check if there's an update for the sibling position.
    fn update_proof<const HEIGHT: usize>(&self, leaf_index: usize, proof: &mut [[u8; 32]; HEIGHT]) {
        for level in 0..HEIGHT {
            // The sibling's node index at this level
            let my_node_index = leaf_index >> level;
            let sibling_node_index = my_node_index ^ 1;

            // If the sibling was updated, use its new hash
            if let Some(&hash) = self.cache.get(&(level, sibling_node_index)) {
                proof[level] = hash;
            }
        }
    }
}

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

impl BatchAddressAppendInputs {
    #[allow(clippy::too_many_arguments)]
    pub fn new<const HEIGHT: usize>(
        batch_size: usize,
        leaves_hashchain: [u8; 32],
        low_element_values: Vec<[u8; 32]>,
        low_element_indices: Vec<usize>,
        low_element_next_indices: Vec<usize>,
        low_element_next_values: Vec<[u8; 32]>,
        low_element_proofs: Vec<Vec<[u8; 32]>>,
        new_element_values: Vec<[u8; 32]>,
        new_element_proofs: Vec<Vec<[u8; 32]>>,
        new_root: [u8; 32],
        old_root: [u8; 32],
        start_index: usize,
    ) -> Result<Self, ProverClientError> {
        let hash_chain_inputs = [
            old_root,
            new_root,
            leaves_hashchain,
            bigint_to_be_bytes_array::<32>(&start_index.into()).unwrap(),
        ];
        let public_input_hash = create_hash_chain_from_array(hash_chain_inputs)?;

        let low_element_proofs_bigint: Vec<Vec<BigUint>> = low_element_proofs
            .into_iter()
            .map(|proof| {
                proof
                    .into_iter()
                    .map(|p| BigUint::from_bytes_be(&p))
                    .collect()
            })
            .collect();

        let new_element_proofs_bigint: Vec<Vec<BigUint>> = new_element_proofs
            .into_iter()
            .map(|proof| {
                proof
                    .into_iter()
                    .map(|p| BigUint::from_bytes_be(&p))
                    .collect()
            })
            .collect();

        Ok(Self {
            batch_size,
            hashchain_hash: BigUint::from_bytes_be(&leaves_hashchain),
            low_element_values: low_element_values
                .iter()
                .map(|v| BigUint::from_bytes_be(v))
                .collect(),
            low_element_indices: low_element_indices
                .iter()
                .map(|&i| BigUint::from(i))
                .collect(),
            low_element_next_indices: low_element_next_indices
                .iter()
                .map(|&i| BigUint::from(i))
                .collect(),
            low_element_next_values: low_element_next_values
                .iter()
                .map(|v| BigUint::from_bytes_be(v))
                .collect(),
            low_element_proofs: low_element_proofs_bigint,
            new_element_values: new_element_values
                .iter()
                .map(|v| BigUint::from_bytes_be(v))
                .collect(),
            new_element_proofs: new_element_proofs_bigint,
            new_root: BigUint::from_bytes_be(&new_root),
            old_root: BigUint::from_bytes_be(&old_root),
            public_input_hash: BigUint::from_bytes_be(&public_input_hash),
            start_index,
            tree_height: HEIGHT,
        })
    }
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

    // Build proof cache from existing changelog entries for O(HEIGHT) proof updates
    // instead of O(changelog_size) iteration
    let mut proof_cache = ProofCache::default();
    for entry in changelog.iter() {
        proof_cache.add_entry::<HEIGHT>(entry);
    }

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
            // Use proof cache for O(HEIGHT) update instead of O(changelog_size) iteration
            let mut low_element_proof_arr: [[u8; 32]; HEIGHT] =
                low_element_proof.clone().try_into().unwrap_or_else(|v: Vec<[u8; 32]>| {
                    panic!("Expected {} elements, got {}", HEIGHT, v.len())
                });
            proof_cache.update_proof::<HEIGHT>(low_element.index(), &mut low_element_proof_arr);
            let merkle_proof = low_element_proof_arr;

            let new_low_leaf_hash = new_low_element
                .hash::<Poseidon>(&new_element.value)
                .unwrap();
            let (_updated_root, changelog_entry) = compute_root_from_merkle_proof::<HEIGHT>(
                new_low_leaf_hash,
                &merkle_proof,
                new_low_element.index as u32,
            );
            // Add to cache before pushing to changelog (for subsequent iterations)
            proof_cache.add_entry::<HEIGHT>(&changelog_entry);
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

            // Use proof cache for O(HEIGHT) update instead of O(changelog_size) iteration
            proof_cache.update_proof::<HEIGHT>(current_index, &mut merkle_proof_array);

            let (updated_root, changelog_entry) = compute_root_from_merkle_proof(
                new_element_leaf_hash,
                &merkle_proof_array,
                current_index as u32,
            );
            new_root = updated_root;

            // Add the new entry to both the changelog (for return) and cache (for next iterations)
            proof_cache.add_entry::<HEIGHT>(&changelog_entry);
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

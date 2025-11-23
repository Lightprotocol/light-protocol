use std::collections::HashMap;

use light_hasher::{
    bigint::bigint_to_be_bytes_array,
    hash_chain::{create_hash_chain_from_array, create_hash_chain_from_slice},
    Poseidon,
};
use light_indexed_array::{array::IndexedElement, changelog::RawIndexedElement};
use light_sparse_merkle_tree::{
    changelog::ChangelogEntry,
    indexed_changelog::{patch_indexed_changelogs, IndexedChangelogEntry},
    SparseMerkleTree,
};
use num_bigint::BigUint;

use crate::{errors::ProverClientError, helpers::compute_root_from_merkle_proof};

#[derive(Default)]
struct ProofCache {
    cache: HashMap<(usize, usize), [u8; 32]>,
}

impl ProofCache {
    fn add_entry<const HEIGHT: usize>(&mut self, entry: &ChangelogEntry<HEIGHT>) {
        let index = entry.index();
        for level in 0..HEIGHT {
            if let Some(hash) = entry.path[level] {
                let node_index = index >> level;
                self.cache.insert((level, node_index), hash);
            }
        }
    }

    fn get_sibling_hash(&self, level: usize, sibling_node_index: usize) -> Option<[u8; 32]> {
        self.cache.get(&(level, sibling_node_index)).copied()
    }
}

struct ChangelogProofPatcher {
    cache: ProofCache,
    hits: usize,
    misses: usize,
    overwrites: usize,
}

impl ChangelogProofPatcher {
    fn new<const HEIGHT: usize>(changelog: &[ChangelogEntry<HEIGHT>]) -> Self {
        let mut cache = ProofCache::default();
        for entry in changelog.iter() {
            cache.add_entry::<HEIGHT>(entry);
        }
        Self {
            cache,
            hits: 0,
            misses: 0,
            overwrites: 0,
        }
    }

    fn update_proof<const HEIGHT: usize>(
        &mut self,
        leaf_index: usize,
        proof: &mut [[u8; 32]; HEIGHT],
    ) {
        for (level, proof_element) in proof.iter_mut().enumerate() {
            let my_node_index = leaf_index >> level;
            let sibling_node_index = my_node_index ^ 1;
            if let Some(hash) = self.cache.get_sibling_hash(level, sibling_node_index) {
                self.hits += 1;
                if *proof_element != hash {
                    self.overwrites += 1;
                }
                *proof_element = hash;
            } else {
                self.misses += 1;
            }
        }
    }

    fn push_changelog_entry<const HEIGHT: usize>(
        &mut self,
        changelog: &mut Vec<ChangelogEntry<HEIGHT>>,
        entry: ChangelogEntry<HEIGHT>,
    ) {
        self.cache.add_entry::<HEIGHT>(&entry);
        changelog.push(entry);
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
        low_element_values: &[[u8; 32]],
        low_element_indices: &[u64],
        low_element_next_indices: &[u64],
        low_element_next_values: &[[u8; 32]],
        low_element_proofs: Vec<Vec<[u8; 32]>>,
        new_element_values: &[[u8; 32]],
        new_element_proofs: Vec<Vec<[u8; 32]>>,
        new_root: [u8; 32],
        old_root: [u8; 32],
        start_index: usize,
    ) -> Result<Self, ProverClientError> {
        let hash_chain_inputs = [
            old_root,
            new_root,
            leaves_hashchain,
            bigint_to_be_bytes_array::<32>(&start_index.into())?,
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
    let new_element_values = new_element_values[0..zkp_batch_size].to_vec();

    let computed_hashchain = create_hash_chain_from_slice(&new_element_values).map_err(|e| {
        ProverClientError::GenericError(format!("Failed to compute hashchain: {}", e))
    })?;
    if computed_hashchain != leaves_hashchain {
        tracing::error!(
            "hashchain mismatch: computed {:?} != indexer {:?} (batch_size={}, next_index={})",
            &computed_hashchain[..8],
            &leaves_hashchain[..8],
            zkp_batch_size,
            next_index
        );
        for (i, addr) in new_element_values.iter().take(3).enumerate() {
            tracing::error!("  address[{}] = {:?}[..8]", i, &addr[..8]);
        }
        return Err(ProverClientError::HashchainMismatch {
            computed: computed_hashchain,
            expected: leaves_hashchain,
            batch_size: zkp_batch_size,
            next_index,
        });
    }
    tracing::debug!(
        "Hashchain validated OK: {:?}[..4] (batch_size={}, next_index={})",
        &computed_hashchain[..4],
        zkp_batch_size,
        next_index
    );

    let mut new_root = [0u8; 32];
    let mut low_element_circuit_merkle_proofs = vec![];
    let mut new_element_circuit_merkle_proofs = vec![];

    let mut patched_low_element_next_values: Vec<[u8; 32]> = Vec::new();
    let mut patched_low_element_next_indices: Vec<usize> = Vec::new();
    let mut patched_low_element_values: Vec<[u8; 32]> = Vec::new();
    let mut patched_low_element_indices: Vec<usize> = Vec::new();

    let mut patcher = ChangelogProofPatcher::new::<HEIGHT>(changelog);

    let is_first_batch = indexed_changelog.is_empty();
    let mut expected_root_for_low = current_root;

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
        .map_err(|e| {
            ProverClientError::ProofPatchFailed(format!(
                "failed to patch indexed changelogs: {}",
                e
            ))
        })?;
        patched_low_element_next_values
            .push(bigint_to_be_bytes_array::<32>(&low_element_next_value)?);
        patched_low_element_next_indices.push(low_element.next_index());
        patched_low_element_indices.push(low_element.index);
        patched_low_element_values.push(bigint_to_be_bytes_array::<32>(&low_element.value)?);

        let new_low_element: IndexedElement<usize> = IndexedElement {
            index: low_element.index,
            value: low_element.value.clone(),
            next_index: new_element.index,
        };
        let new_low_element_raw = RawIndexedElement {
            value: bigint_to_be_bytes_array::<32>(&new_low_element.value)?,
            next_index: new_low_element.next_index,
            next_value: bigint_to_be_bytes_array::<32>(&new_element.value)?,
            index: new_low_element.index,
        };

        let intermediate_root = {
            let mut low_element_proof_arr: [[u8; 32]; HEIGHT] = low_element_proof
                .clone()
                .try_into()
                .map_err(|v: Vec<[u8; 32]>| {
                    ProverClientError::ProofPatchFailed(format!(
                        "low element proof length mismatch: expected {}, got {}",
                        HEIGHT,
                        v.len()
                    ))
                })?;
            patcher.update_proof::<HEIGHT>(low_element.index(), &mut low_element_proof_arr);
            let merkle_proof = low_element_proof_arr;

            if is_first_batch {
                let old_low_leaf_hash = low_element
                    .hash::<Poseidon>(&low_element_next_value)
                    .map_err(|e| {
                        ProverClientError::GenericError(format!(
                            "Failed to hash old low element: {}",
                            e
                        ))
                    })?;
                let (computed_root, _) = compute_root_from_merkle_proof::<HEIGHT>(
                    old_low_leaf_hash,
                    &merkle_proof,
                    low_element.index as u32,
                );
                if computed_root != expected_root_for_low {
                    let low_value_bytes = bigint_to_be_bytes_array::<32>(&low_element.value)
                        .map_err(|e| {
                            ProverClientError::GenericError(format!(
                                "Failed to serialize low element value: {}",
                                e
                            ))
                        })?;
                    let low_next_value_bytes =
                        bigint_to_be_bytes_array::<32>(&low_element_next_value).map_err(|e| {
                            ProverClientError::GenericError(format!(
                                "Failed to serialize low element next value: {}",
                                e
                            ))
                        })?;
                    return Err(ProverClientError::GenericError(format!(
                        "element {}: low proof mismatch (computed {:?}[..4] != expected {:?}[..4], low_idx={}, low_value={:?}[..4], low_next={:?}[..4])",
                        i,
                        &computed_root[..4],
                        &expected_root_for_low[..4],
                        low_element.index,
                        &low_value_bytes[..4],
                        &low_next_value_bytes[..4],
                    )));
                }
            }

            let new_low_leaf_hash = new_low_element
                .hash::<Poseidon>(&new_element.value)
                .map_err(|e| {
                    ProverClientError::GenericError(format!(
                        "Failed to hash new low element: {}",
                        e
                    ))
                })?;
            let (low_update_intermediate_root, changelog_entry) =
                compute_root_from_merkle_proof::<HEIGHT>(
                    new_low_leaf_hash,
                    &merkle_proof,
                    new_low_element.index as u32,
                );

            patcher.push_changelog_entry::<HEIGHT>(changelog, changelog_entry);
            low_element_circuit_merkle_proofs.push(
                merkle_proof
                    .iter()
                    .map(|hash| BigUint::from_bytes_be(hash))
                    .collect(),
            );

            low_update_intermediate_root
        };
        let low_element_changelog_entry = IndexedChangelogEntry {
            element: new_low_element_raw,
            proof: low_element_proof.as_slice()[..HEIGHT]
                .try_into()
                .map_err(|_| {
                    ProverClientError::ProofPatchFailed(
                        "low_element_proof slice conversion failed".to_string(),
                    )
                })?,
            changelog_index: indexed_changelog.len(), //change_log_index,
        };

        indexed_changelog.push(low_element_changelog_entry);

        {
            let new_element_next_value = low_element_next_value;
            let new_element_leaf_hash = new_element
                .hash::<Poseidon>(&new_element_next_value)
                .map_err(|e| {
                    ProverClientError::GenericError(format!("Failed to hash new element: {}", e))
                })?;

            let sparse_root_before = sparse_merkle_tree.root();
            let sparse_next_idx_before = sparse_merkle_tree.get_next_index();

            let mut merkle_proof_array = sparse_merkle_tree.append(new_element_leaf_hash);

            let current_index = next_index + i;

            patcher.update_proof::<HEIGHT>(current_index, &mut merkle_proof_array);

            let (updated_root, changelog_entry) = compute_root_from_merkle_proof(
                new_element_leaf_hash,
                &merkle_proof_array,
                current_index as u32,
            );

            if i == 0 && changelog.len() == 1 {
                if sparse_next_idx_before != current_index {
                    return Err(ProverClientError::GenericError(format!(
                        "sparse index mismatch: sparse tree next_index={} but expected current_index={}",
                        sparse_next_idx_before, current_index
                    )));
                }

                if sparse_root_before != current_root {
                    return Err(ProverClientError::GenericError(format!(
                        "sparse root mismatch: sparse tree root {:?}[..4] != current_root {:?}[..4] \
                         (next_index={}). The subtrees from indexer may be stale.",
                        &sparse_root_before[..4],
                        &current_root[..4],
                        next_index
                    )));
                }
            }

            if is_first_batch {
                let zero_hash = [0u8; 32];
                let (root_with_zero, _) = compute_root_from_merkle_proof::<HEIGHT>(
                    zero_hash,
                    &merkle_proof_array,
                    current_index as u32,
                );
                if root_with_zero != intermediate_root {
                    tracing::error!(
                        "ELEMENT {} NEW_PROOF MISMATCH: proof + ZERO = {:?}[..4] but expected \
                         intermediate_root = {:?}[..4] (index={}, low_idx={})",
                        i,
                        &root_with_zero[..4],
                        &intermediate_root[..4],
                        current_index,
                        low_element.index
                    );
                    return Err(ProverClientError::GenericError(format!(
                        "ELEMENT {} NEW_PROOF MISMATCH: proof + ZERO = {:?}[..4] but expected \
                         intermediate_root = {:?}[..4] (index={}, low_idx={}). Patched proof is incorrect.",
                        i,
                        &root_with_zero[..4],
                        &intermediate_root[..4],
                        current_index,
                        low_element.index
                    )));
                }
                if i == 0 {
                    tracing::info!(
                        "VALIDATION_PASS: element 0 new_element proof OK \
                         (intermediate_root {:?}[..4] -> updated_root {:?}[..4])",
                        &intermediate_root[..4],
                        &updated_root[..4]
                    );
                }
                expected_root_for_low = updated_root;
            }

            new_root = updated_root;

            patcher.push_changelog_entry::<HEIGHT>(changelog, changelog_entry);
            new_element_circuit_merkle_proofs.push(
                merkle_proof_array
                    .iter()
                    .map(|hash| BigUint::from_bytes_be(hash))
                    .collect(),
            );

            let new_element_raw = RawIndexedElement {
                value: bigint_to_be_bytes_array::<32>(&new_element.value)?,
                next_index: new_element.next_index,
                next_value: bigint_to_be_bytes_array::<32>(&new_element_next_value)?,
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
        bigint_to_be_bytes_array::<32>(&next_index.into())?,
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

    tracing::debug!(
        "Address proof patcher stats: hits={}, misses={}, overwrites={}, changelog_len={}, indexed_changelog_len={}",
        patcher.hits,
        patcher.misses,
        patcher.overwrites,
        changelog.len(),
        indexed_changelog.len()
    );
    if patcher.hits == 0 && !changelog.is_empty() {
        tracing::warn!(
            "Address proof patcher had 0 cache hits despite non-empty changelog (changelog_len={}, indexed_changelog_len={})",
            changelog.len(),
            indexed_changelog.len()
        );
    }

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

use std::collections::HashMap;

use light_batched_merkle_tree::constants::DEFAULT_BATCH_ADDRESS_TREE_HEIGHT;
use light_hasher::{bigint::bigint_to_be_bytes_array, Hasher, Poseidon};
use light_indexed_array::{array::IndexedElement, changelog::RawIndexedElement};
use light_merkle_tree_reference::MerkleTree;
use light_prover_client::{
    helpers::compute_root_from_merkle_proof,
    proof_types::batch_address_append::BatchAddressAppendInputs,
};
use light_sparse_merkle_tree::{
    changelog::ChangelogEntry,
    indexed_changelog::{patch_indexed_changelogs, IndexedChangelogEntry},
    SparseMerkleTree,
};
use num_bigint::BigUint;

use crate::error::ForesterUtilsError;

const HEIGHT: usize = DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize;

/// Cache for proof updates - maps (level, node_index) to latest hash.
/// This mirrors the ProofCache from batch_address_append/proof_inputs.rs.
#[derive(Default)]
struct ProofCache {
    /// Maps (level, node_index_at_level) -> hash
    /// node_index_at_level = leaf_index >> level
    cache: HashMap<(usize, usize), [u8; 32]>,
}

impl ProofCache {
    /// Add a changelog entry to the cache.
    /// For each level, store the hash that would be used as a sibling.
    fn add_entry(&mut self, entry: &ChangelogEntry<HEIGHT>) {
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
    fn update_proof(&self, leaf_index: usize, proof: &mut [[u8; 32]; HEIGHT]) {
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

#[derive(Clone, Debug)]
pub struct AddressBatchResult {
    pub circuit_inputs: BatchAddressAppendInputs,
    pub new_root: [u8; 32],
    pub old_root: [u8; 32],
}

#[derive(Clone, Debug)]
pub struct AddressStagingTree {
    merkle_tree: MerkleTree<Poseidon>,
    indexed_changelog: Vec<IndexedChangelogEntry<usize, HEIGHT>>,
    /// Changelog entries for proof patching across batches.
    /// This is separate from indexed_changelog and contains the path hashes
    /// needed by ProofCache to update proofs for subsequent batches.
    changelog: Vec<ChangelogEntry<HEIGHT>>,
    current_root: [u8; 32],
    next_index: usize,
    /// The initial start_index when the tree was created.
    /// The merkle tree only contains leaves with indices 0..initial_start_index.
    /// This is different from next_index which grows as batches are processed.
    initial_start_index: usize,
    /// Optional frontier-based sparse tree (built from `subtrees` provided by the indexer).
    sparse_tree: Option<SparseMerkleTree<Poseidon, HEIGHT>>,
    sparse_changelog: Vec<ChangelogEntry<HEIGHT>>,
    sparse_indexed_changelog: Vec<IndexedChangelogEntry<usize, HEIGHT>>,
}

impl AddressStagingTree {
    pub fn new(initial_root: [u8; 32], start_index: usize) -> Self {
        AddressStagingTree {
            merkle_tree: MerkleTree::<Poseidon>::new(HEIGHT, 0),
            indexed_changelog: Vec::new(),
            changelog: Vec::new(),
            current_root: initial_root,
            next_index: start_index,
            initial_start_index: start_index,
            sparse_tree: None,
            sparse_changelog: Vec::new(),
            sparse_indexed_changelog: Vec::new(),
        }
    }

    pub fn from_nodes(
        nodes: &[u64],
        node_hashes: &[[u8; 32]],
        initial_root: [u8; 32],
        start_index: usize,
        subtrees: Option<[[u8; 32]; HEIGHT]>,
    ) -> Result<Self, ForesterUtilsError> {
        let mut merkle_tree = MerkleTree::<Poseidon>::new(HEIGHT, 0);
        for (&node_index, &node_hash) in nodes.iter().zip(node_hashes.iter()) {
            let level = (node_index >> 56) as usize;
            if level == HEIGHT {
                continue;
            }

            // Filter out "future" nodes that are beyond start_index
            // Calculate the range of leaves covered by this node
            let shift = level;
            let node_start_index = (node_index & 0x00FFFFFFFFFFFFFF) << shift;
            let node_end_index = ((node_index & 0x00FFFFFFFFFFFFFF) + 1) << shift;

            // If the node starts at or after start_index, it is fully in the future -> Skip
            if node_start_index >= start_index as u64 {
                continue;
            }

            // If the node overlaps with start_index (i.e. partially future),
            // we must also skip it because it contains "dirty" data merged with future data.
            // We will recompute these boundary nodes later.
            if node_end_index > start_index as u64 {
                continue;
            }

            merkle_tree
                .insert_node(node_index, node_hash)
                .map_err(|e| {
                    ForesterUtilsError::AddressStagingTree(format!("Failed to insert node: {}", e))
                })?;
        }

        // Set rightmost_index based on start_index
        merkle_tree.rightmost_index = start_index;
        merkle_tree.roots.push(initial_root);

        // If we filtered out boundary nodes, we need to recompute them.
        // We can do this by "updating" the last leaf with its own value.
        if start_index > 0 {
            let last_leaf_index = start_index - 1;
            if let Ok(last_leaf_hash) = merkle_tree.get_leaf(last_leaf_index) {
                merkle_tree
                    .update(&last_leaf_hash, last_leaf_index)
                    .map_err(|e| {
                        ForesterUtilsError::AddressStagingTree(format!(
                            "Failed to fix boundary path: {}",
                            e
                        ))
                    })?;
            }
        }

        Ok(Self {
            merkle_tree,
            indexed_changelog: Vec::new(),
            changelog: Vec::new(),
            current_root: initial_root,
            next_index: start_index,
            initial_start_index: start_index,
            sparse_tree: subtrees.map(|frontier| SparseMerkleTree::new(frontier, start_index)),
            sparse_changelog: Vec::new(),
            sparse_indexed_changelog: Vec::new(),
        })
    }

    pub fn current_root(&self) -> [u8; 32] {
        self.current_root
    }

    pub fn next_index(&self) -> usize {
        self.next_index
    }

    pub fn clear_changelogs(&mut self) {
        self.indexed_changelog.clear();
        self.changelog.clear();
        self.sparse_changelog.clear();
        self.sparse_indexed_changelog.clear();
    }

    fn has_full_tree(&self) -> bool {
        self.merkle_tree
            .layers
            .iter()
            .any(|layer| !layer.is_empty())
    }

    fn get_proof(&self, leaf_index: usize) -> Result<[[u8; 32]; HEIGHT], ForesterUtilsError> {
        let proof_vec = self
            .merkle_tree
            .get_proof_of_leaf(leaf_index, true)
            .map_err(|e| {
                ForesterUtilsError::AddressStagingTree(format!("Failed to get proof: {}", e))
            })?;
        proof_vec.try_into().map_err(|v: Vec<[u8; 32]>| {
            ForesterUtilsError::AddressStagingTree(format!(
                "Proof length mismatch: expected {}, got {}",
                HEIGHT,
                v.len()
            ))
        })
    }

    fn get_append_proof(
        &self,
        leaf_index: usize,
    ) -> Result<[[u8; 32]; HEIGHT], ForesterUtilsError> {
        let mut proof: [[u8; 32]; HEIGHT] = [[0u8; 32]; HEIGHT];
        let mut current_index = leaf_index;

        for level in 0..HEIGHT {
            let is_left = current_index % 2 == 0;
            let sibling_index = if is_left {
                current_index + 1
            } else {
                current_index - 1
            };

            proof[level] = self
                .merkle_tree
                .layers
                .get(level)
                .and_then(|layer| layer.get(sibling_index).copied())
                .unwrap_or(Poseidon::zero_bytes()[level]);

            current_index /= 2;
        }

        Ok(proof)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn process_batch(
        &mut self,
        addresses: &[[u8; 32]],
        low_element_values: &[[u8; 32]],
        low_element_next_values: &[[u8; 32]],
        low_element_indices: &[u64],
        low_element_next_indices: &[u64],
        low_element_proofs: &[Vec<[u8; 32]>],
        leaves_hashchain: [u8; 32],
        zkp_batch_size: usize,
        epoch: u64,
        tree: &str,
    ) -> Result<AddressBatchResult, ForesterUtilsError> {
        if let Some(mut sparse_tree) = self.sparse_tree.take() {
            let result = self.process_batch_sparse(
                &mut sparse_tree,
                addresses,
                low_element_values,
                low_element_next_values,
                low_element_indices,
                low_element_next_indices,
                low_element_proofs,
                leaves_hashchain,
                zkp_batch_size,
                epoch,
                tree,
            );
            self.sparse_tree = Some(sparse_tree);
            return result;
        }

        let old_root = self.current_root;
        let start_index = self.next_index;
        let use_direct_proofs = self.has_full_tree();

        tracing::debug!(
            "AddressStagingTree::process_batch: start_index={}, initial_start_index={}, \
             zkp_batch_size={}, use_direct_proofs={}, changelog_len={}, addresses_len={}",
            start_index,
            self.initial_start_index,
            zkp_batch_size,
            use_direct_proofs,
            self.changelog.len(),
            addresses.len()
        );

        let mut circuit_low_element_values: Vec<[u8; 32]> = Vec::with_capacity(zkp_batch_size);
        let mut circuit_low_element_indices: Vec<u64> = Vec::with_capacity(zkp_batch_size);
        let mut circuit_low_element_next_indices: Vec<u64> = Vec::with_capacity(zkp_batch_size);
        let mut circuit_low_element_next_values: Vec<[u8; 32]> = Vec::with_capacity(zkp_batch_size);
        let mut circuit_low_element_proofs: Vec<Vec<[u8; 32]>> = Vec::with_capacity(zkp_batch_size);
        let mut circuit_new_element_proofs: Vec<Vec<[u8; 32]>> = Vec::with_capacity(zkp_batch_size);

        let mut new_root = old_root;

        // ProofCache tracks updates for leaves that can't be updated in the merkle tree
        // (because they don't exist yet - they're being created in this batch).
        // This allows us to patch proofs for subsequent operations.
        //
        // IMPORTANT: Initialize the proof cache from existing changelog entries.
        // This is critical for multi-batch processing - without this, the second batch
        // won't have the proof updates from the first batch and will fail with
        // constraint errors.
        let mut proof_cache = ProofCache::default();
        for entry in self.changelog.iter() {
            proof_cache.add_entry(entry);
        }

        for i in 0..zkp_batch_size {
            let new_element_index = start_index + i;

            let mut low_element = IndexedElement {
                index: low_element_indices[i] as usize,
                value: BigUint::from_bytes_be(&low_element_values[i]),
                next_index: low_element_next_indices[i] as usize,
            };

            let mut new_element = IndexedElement {
                index: new_element_index,
                value: BigUint::from_bytes_be(&addresses[i]),
                next_index: low_element_next_indices[i] as usize,
            };

            let mut low_element_next_value = BigUint::from_bytes_be(&low_element_next_values[i]);

            let mut low_element_proof = low_element_proofs[i].clone();
            let mut _changelog_index = 0;
            patch_indexed_changelogs(
                0,
                &mut _changelog_index,
                &mut self.indexed_changelog,
                &mut low_element,
                &mut new_element,
                &mut low_element_next_value,
                &mut low_element_proof,
            )
            .map_err(|e| {
                ForesterUtilsError::AddressStagingTree(format!(
                    "Failed to patch indexed changelog: {}",
                    e
                ))
            })?;

            // Get the low element proof.
            // IMPORTANT: Always use the patched proof from patch_indexed_changelogs and then
            // apply proof_cache updates on top. This matches the reference implementation.
            // We CANNOT use self.get_proof() for low elements because:
            // 1. The merkle tree might have been updated by previous iterations in this batch
            // 2. Using proof_cache.update_proof() after get_proof() would double-apply updates
            // The reference implementation never updates the merkle tree for low elements -
            // it only tracks changes via proof_cache/changelog.
            let mut low_element_proof_arr: [[u8; 32]; HEIGHT] =
                low_element_proof.try_into().map_err(|v: Vec<[u8; 32]>| {
                    ForesterUtilsError::AddressStagingTree(format!(
                        "Proof length mismatch: expected {}, got {}",
                        HEIGHT,
                        v.len()
                    ))
                })?;

            // Apply any cached updates to the proof from previous iterations in this batch
            proof_cache.update_proof(low_element.index, &mut low_element_proof_arr);

            circuit_low_element_values.push(
                bigint_to_be_bytes_array::<32>(&low_element.value).map_err(|e| {
                    ForesterUtilsError::AddressStagingTree(format!(
                        "BigInt conversion error: {}",
                        e
                    ))
                })?,
            );
            circuit_low_element_indices.push(low_element.index as u64);
            circuit_low_element_next_indices.push(low_element.next_index as u64);
            circuit_low_element_next_values.push(
                bigint_to_be_bytes_array::<32>(&low_element_next_value).map_err(|e| {
                    ForesterUtilsError::AddressStagingTree(format!(
                        "BigInt conversion error: {}",
                        e
                    ))
                })?,
            );
            circuit_low_element_proofs.push(low_element_proof_arr.to_vec());

            let new_low_element = IndexedElement {
                index: low_element.index,
                value: low_element.value.clone(),
                next_index: new_element.index,
            };

            let new_low_leaf = new_low_element
                .hash::<Poseidon>(&new_element.value)
                .map_err(|e| {
                    ForesterUtilsError::AddressStagingTree(format!(
                        "Failed to hash new low element: {}",
                        e
                    ))
                })?;
            let new_element_leaf = new_element
                .hash::<Poseidon>(&low_element_next_value)
                .map_err(|e| {
                    ForesterUtilsError::AddressStagingTree(format!(
                        "Failed to hash new element: {}",
                        e
                    ))
                })?;

            let new_low_element_raw = RawIndexedElement {
                value: bigint_to_be_bytes_array::<32>(&new_low_element.value).unwrap(),
                next_index: new_low_element.next_index,
                next_value: bigint_to_be_bytes_array::<32>(&new_element.value).unwrap(),
                index: new_low_element.index,
            };
            self.indexed_changelog.push(IndexedChangelogEntry {
                element: new_low_element_raw,
                proof: low_element_proof_arr,
                changelog_index: self.indexed_changelog.len(),
            });

            // Compute the changelog entry for the low element update using the same
            // approach as the reference implementation. This ensures the proof cache
            // has correct entries for subsequent operations.
            let (_low_element_root, low_element_changelog_entry) =
                compute_root_from_merkle_proof::<HEIGHT>(
                    new_low_leaf,
                    &low_element_proof_arr,
                    low_element.index as u32,
                );
            proof_cache.add_entry(&low_element_changelog_entry);
            // Store the changelog entry for use in subsequent batches
            self.changelog.push(low_element_changelog_entry);

            // NOTE: We intentionally do NOT update the merkle tree for low elements.
            // Following the reference implementation, low element updates are tracked
            // only via proof_cache/changelog. The merkle tree is only updated for
            // new element appends. This avoids double-application of updates when
            // using proof_cache.update_proof() on subsequent proofs.

            if use_direct_proofs {
                let mut new_element_proof = self.get_append_proof(new_element_index)?;
                // Apply cached updates to the new element proof
                proof_cache.update_proof(new_element_index, &mut new_element_proof);
                circuit_new_element_proofs.push(new_element_proof.to_vec());

                self.merkle_tree.append(&new_element_leaf).map_err(|e| {
                    ForesterUtilsError::AddressStagingTree(format!(
                        "Failed to append new element: {}",
                        e
                    ))
                })?;

                // Compute the changelog entry for the new element append
                let (updated_root, new_element_changelog_entry) =
                    compute_root_from_merkle_proof::<HEIGHT>(
                        new_element_leaf,
                        &new_element_proof,
                        new_element_index as u32,
                    );
                proof_cache.add_entry(&new_element_changelog_entry);
                // Store the changelog entry for use in subsequent batches
                self.changelog.push(new_element_changelog_entry);

                new_root = updated_root;
            } else {
                circuit_new_element_proofs.push(vec![[0u8; 32]; HEIGHT]);
            }

            let new_element_raw = RawIndexedElement {
                value: bigint_to_be_bytes_array::<32>(&new_element.value).unwrap(),
                next_index: new_element.next_index,
                next_value: bigint_to_be_bytes_array::<32>(&low_element_next_value).unwrap(),
                index: new_element.index,
            };
            let new_element_proof_arr: [[u8; 32]; HEIGHT] = if use_direct_proofs {
                self.get_proof(new_element_index)?
            } else {
                [[0u8; 32]; HEIGHT]
            };
            self.indexed_changelog.push(IndexedChangelogEntry {
                element: new_element_raw,
                proof: new_element_proof_arr,
                changelog_index: self.indexed_changelog.len(),
            });
        }

        self.current_root = new_root;
        self.next_index += zkp_batch_size;

        let circuit_inputs = BatchAddressAppendInputs::new::<HEIGHT>(
            zkp_batch_size,
            leaves_hashchain,
            &circuit_low_element_values,
            &circuit_low_element_indices,
            &circuit_low_element_next_indices,
            &circuit_low_element_next_values,
            circuit_low_element_proofs,
            &addresses[..zkp_batch_size],
            circuit_new_element_proofs,
            new_root,
            old_root,
            start_index,
        )
        .map_err(|e| {
            ForesterUtilsError::AddressStagingTree(format!(
                "Failed to create circuit inputs: {}",
                e
            ))
        })?;

        tracing::debug!(
            "ADDRESS_APPEND batch root transition: {:?}[..4] -> {:?}[..4] (batch_size={}, next_index={}, epoch={}, tree={})",
            &old_root[..4],
            &new_root[..4],
            circuit_inputs.batch_size,
            self.next_index,
            epoch,
            tree
        );

        Ok(AddressBatchResult {
            circuit_inputs,
            new_root,
            old_root,
        })
    }

    fn process_batch_sparse(
        &mut self,
        sparse_tree: &mut SparseMerkleTree<Poseidon, HEIGHT>,
        addresses: &[[u8; 32]],
        low_element_values: &[[u8; 32]],
        low_element_next_values: &[[u8; 32]],
        low_element_indices: &[u64],
        low_element_next_indices: &[u64],
        low_element_proofs: &[Vec<[u8; 32]>],
        leaves_hashchain: [u8; 32],
        zkp_batch_size: usize,
        epoch: u64,
        tree: &str,
    ) -> Result<AddressBatchResult, ForesterUtilsError> {
        use light_prover_client::proof_types::batch_address_append::get_batch_address_append_circuit_inputs;

        let old_root = self.current_root;
        let next_index = self.next_index;

        let inputs = get_batch_address_append_circuit_inputs::<HEIGHT>(
            next_index,
            old_root,
            low_element_values.to_vec(),
            low_element_next_values.to_vec(),
            low_element_indices.iter().map(|v| *v as usize).collect(),
            low_element_next_indices
                .iter()
                .map(|v| *v as usize)
                .collect(),
            low_element_proofs.to_vec(),
            addresses.to_vec(),
            sparse_tree,
            leaves_hashchain,
            zkp_batch_size,
            &mut self.sparse_changelog,
            &mut self.sparse_indexed_changelog,
        )
        .map_err(|e| {
            ForesterUtilsError::AddressStagingTree(format!(
                "Failed to build sparse circuit inputs: {}",
                e
            ))
        })?;

        self.current_root = bigint_to_be_bytes_array::<32>(&inputs.new_root).map_err(|e| {
            ForesterUtilsError::AddressStagingTree(format!("Failed to serialize new root: {}", e))
        })?;
        self.next_index += zkp_batch_size;

        tracing::debug!(
            "ADDRESS_APPEND batch root transition: {:?}[..4] -> {:?}[..4] (sparse, batch_size={}, next_index={}, epoch={}, tree={})",
            &old_root[..4],
            &self.current_root[..4],
            zkp_batch_size,
            self.next_index,
            epoch,
            tree
        );

        Ok(AddressBatchResult {
            circuit_inputs: inputs,
            new_root: self.current_root,
            old_root,
        })
    }
}

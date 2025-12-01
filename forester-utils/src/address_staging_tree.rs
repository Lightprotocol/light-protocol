use light_batched_merkle_tree::constants::DEFAULT_BATCH_ADDRESS_TREE_HEIGHT;
use light_hasher::{bigint::bigint_to_be_bytes_array, Hasher, Poseidon};
use light_indexed_array::{array::IndexedElement, changelog::RawIndexedElement};
use light_merkle_tree_reference::MerkleTree;
use light_prover_client::proof_types::batch_address_append::BatchAddressAppendInputs;
use light_sparse_merkle_tree::indexed_changelog::{
    patch_indexed_changelogs, IndexedChangelogEntry,
};
use num_bigint::BigUint;

use crate::error::ForesterUtilsError;

const HEIGHT: usize = DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize;

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
    current_root: [u8; 32],
    next_index: usize,
}

impl AddressStagingTree {
    pub fn new(initial_root: [u8; 32], start_index: usize) -> Self {
        AddressStagingTree {
            merkle_tree: MerkleTree::<Poseidon>::new(HEIGHT, 0),
            indexed_changelog: Vec::new(),
            current_root: initial_root,
            next_index: start_index,
        }
    }

    pub fn from_nodes(
        nodes: &[u64],
        node_hashes: &[[u8; 32]],
        initial_root: [u8; 32],
        start_index: usize,
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

        let current_root = merkle_tree.root();

        Ok(Self {
            merkle_tree,
            indexed_changelog: Vec::new(),
            current_root,
            next_index: start_index,
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
        addresses: Vec<[u8; 32]>,
        low_element_values: Vec<[u8; 32]>,
        low_element_next_values: Vec<[u8; 32]>,
        low_element_indices: Vec<usize>,
        low_element_next_indices: Vec<usize>,
        low_element_proofs: Vec<Vec<[u8; 32]>>,
        leaves_hashchain: [u8; 32],
        zkp_batch_size: usize,
    ) -> Result<AddressBatchResult, ForesterUtilsError> {
        let old_root = self.current_root;
        let start_index = self.next_index;
        let use_direct_proofs = self.has_full_tree();

        let mut circuit_low_element_values: Vec<[u8; 32]> = Vec::with_capacity(zkp_batch_size);
        let mut circuit_low_element_indices: Vec<usize> = Vec::with_capacity(zkp_batch_size);
        let mut circuit_low_element_next_indices: Vec<usize> = Vec::with_capacity(zkp_batch_size);
        let mut circuit_low_element_next_values: Vec<[u8; 32]> = Vec::with_capacity(zkp_batch_size);
        let mut circuit_low_element_proofs: Vec<Vec<[u8; 32]>> = Vec::with_capacity(zkp_batch_size);
        let mut circuit_new_element_proofs: Vec<Vec<[u8; 32]>> = Vec::with_capacity(zkp_batch_size);

        let mut new_root = old_root;

        for i in 0..zkp_batch_size {
            let new_element_index = start_index + i;

            let mut low_element = IndexedElement {
                index: low_element_indices[i],
                value: BigUint::from_bytes_be(&low_element_values[i]),
                next_index: low_element_next_indices[i],
            };

            let mut new_element = IndexedElement {
                index: new_element_index,
                value: BigUint::from_bytes_be(&addresses[i]),
                next_index: low_element_next_indices[i],
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

            let low_element_proof_arr: [[u8; 32]; HEIGHT] = if use_direct_proofs {
                self.get_proof(low_element.index)?
            } else {
                low_element_proof.try_into().map_err(|v: Vec<[u8; 32]>| {
                    ForesterUtilsError::AddressStagingTree(format!(
                        "Proof length mismatch: expected {}, got {}",
                        HEIGHT,
                        v.len()
                    ))
                })?
            };

            circuit_low_element_values.push(
                bigint_to_be_bytes_array::<32>(&low_element.value).map_err(|e| {
                    ForesterUtilsError::AddressStagingTree(format!(
                        "BigInt conversion error: {}",
                        e
                    ))
                })?,
            );
            circuit_low_element_indices.push(low_element.index);
            circuit_low_element_next_indices.push(low_element.next_index);
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

            if use_direct_proofs {
                self.merkle_tree
                    .update(&new_low_leaf, low_element.index)
                    .map_err(|e| {
                        ForesterUtilsError::AddressStagingTree(format!(
                            "Failed to update low element: {}",
                            e
                        ))
                    })?;

                let new_element_proof = self.get_append_proof(new_element_index)?;
                circuit_new_element_proofs.push(new_element_proof.to_vec());

                self.merkle_tree.append(&new_element_leaf).map_err(|e| {
                    ForesterUtilsError::AddressStagingTree(format!(
                        "Failed to append new element: {}",
                        e
                    ))
                })?;

                new_root = self.merkle_tree.root();
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
            circuit_low_element_values,
            circuit_low_element_indices,
            circuit_low_element_next_indices,
            circuit_low_element_next_values,
            circuit_low_element_proofs,
            addresses[..zkp_batch_size].to_vec(),
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

        Ok(AddressBatchResult {
            circuit_inputs,
            new_root,
            old_root,
        })
    }
}

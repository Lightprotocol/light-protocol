use light_batched_merkle_tree::constants::DEFAULT_BATCH_ADDRESS_TREE_HEIGHT;
use light_hasher::Poseidon;
use light_prover_client::proof_types::batch_address_append::{
    get_batch_address_append_circuit_inputs, BatchAddressAppendInputs,
};
use light_sparse_merkle_tree::{
    changelog::ChangelogEntry, indexed_changelog::IndexedChangelogEntry, SparseMerkleTree,
};
use tracing::debug;

use crate::error::ForesterUtilsError;

const HEIGHT: usize = DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize;

/// Result of processing a batch of address appends
#[derive(Clone, Debug)]
pub struct AddressBatchResult {
    pub circuit_inputs: BatchAddressAppendInputs,
    pub new_root: [u8; 32],
    pub old_root: [u8; 32],
}

/// Staging tree for indexed (address) Merkle trees.
/// Uses SparseMerkleTree and changelogs to properly compute proofs
/// for batch address appends with concurrent updates.
#[derive(Clone, Debug)]
pub struct AddressStagingTree {
    sparse_tree: SparseMerkleTree<Poseidon, HEIGHT>,
    changelog: Vec<ChangelogEntry<HEIGHT>>,
    indexed_changelog: Vec<IndexedChangelogEntry<usize, HEIGHT>>,
    current_root: [u8; 32],
    next_index: usize,
}

impl AddressStagingTree {
    /// Creates a new AddressStagingTree from subtrees data.
    ///
    /// # Arguments
    /// * `subtrees` - Array of subtree hashes for SparseMerkleTree initialization
    /// * `start_index` - The tree's next_index where new leaves will be appended
    /// * `initial_root` - The current root of the tree
    pub fn new(subtrees: [[u8; 32]; HEIGHT], start_index: usize, initial_root: [u8; 32]) -> Self {
        debug!(
            "AddressStagingTree::new: start_index={}, initial_root={:?}[..4]",
            start_index,
            &initial_root[..4]
        );

        Self {
            sparse_tree: SparseMerkleTree::new(subtrees, start_index),
            changelog: Vec::new(),
            indexed_changelog: Vec::new(),
            current_root: initial_root,
            next_index: start_index,
        }
    }

    /// Creates a new AddressStagingTree from a Vec of subtrees.
    /// The subtrees Vec must have exactly HEIGHT elements.
    pub fn from_subtrees_vec(
        subtrees: Vec<[u8; 32]>,
        start_index: usize,
        initial_root: [u8; 32],
    ) -> Result<Self, ForesterUtilsError> {
        let subtrees_array: [[u8; 32]; HEIGHT] =
            subtrees.try_into().map_err(|v: Vec<[u8; 32]>| {
                ForesterUtilsError::AddressStagingTree(format!(
                    "Invalid subtrees length: expected {}, got {}",
                    HEIGHT,
                    v.len()
                ))
            })?;
        Ok(Self::new(subtrees_array, start_index, initial_root))
    }

    /// Returns the current root of the tree.
    pub fn current_root(&self) -> [u8; 32] {
        self.current_root
    }

    /// Returns the current next_index of the tree.
    pub fn next_index(&self) -> usize {
        self.next_index
    }

    /// Processes a batch of address appends and returns the circuit inputs.
    ///
    /// # Arguments
    /// * `addresses` - The new addresses (element values) to append
    /// * `low_element_values` - Values of low elements
    /// * `low_element_next_values` - Next values of low elements
    /// * `low_element_indices` - Indices of low elements
    /// * `low_element_next_indices` - Next indices of low elements
    /// * `low_element_proofs` - Merkle proofs for low elements
    /// * `leaves_hashchain` - Pre-computed hash chain of the addresses
    /// * `zkp_batch_size` - Number of addresses in this batch
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

        debug!(
            "AddressStagingTree::process_batch: {} addresses, start_index={}, old_root={:?}[..4]",
            addresses.len(),
            start_index,
            &old_root[..4]
        );

        let circuit_inputs = get_batch_address_append_circuit_inputs::<HEIGHT>(
            start_index,
            old_root,
            low_element_values,
            low_element_next_values,
            low_element_indices,
            low_element_next_indices,
            low_element_proofs,
            addresses,
            &mut self.sparse_tree,
            leaves_hashchain,
            zkp_batch_size,
            &mut self.changelog,
            &mut self.indexed_changelog,
        )
        .map_err(|e| {
            ForesterUtilsError::AddressStagingTree(format!("Circuit input error: {}", e))
        })?;

        // Update state
        let new_root =
            light_hasher::bigint::bigint_to_be_bytes_array::<32>(&circuit_inputs.new_root)
                .map_err(|e| {
                    ForesterUtilsError::AddressStagingTree(format!("Root conversion error: {}", e))
                })?;

        self.current_root = new_root;
        self.next_index += zkp_batch_size;

        debug!(
            "AddressStagingTree::process_batch complete: new_root={:?}[..4], next_index={}",
            &new_root[..4],
            self.next_index
        );

        Ok(AddressBatchResult {
            circuit_inputs,
            new_root,
            old_root,
        })
    }

    /// Clears the changelogs. Call this when resetting the staging tree.
    pub fn clear_changelogs(&mut self) {
        self.changelog.clear();
        self.indexed_changelog.clear();
    }
}

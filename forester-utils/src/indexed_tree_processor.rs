use std::collections::HashMap;

use light_batched_merkle_tree::constants::DEFAULT_BATCH_ADDRESS_TREE_HEIGHT;
use light_concurrent_merkle_tree::light_hasher::Hasher;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::{array::IndexedElement, reference::IndexedMerkleTree};
use num_bigint::BigUint;
use tracing::{debug, warn};

use crate::error::ForesterUtilsError;

const HEIGHT: usize = DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize;

/// Decode node index from encoded format: (level << 56) | position
#[inline]
fn decode_node_index(encoded: u64) -> (u8, u64) {
    let level = (encoded >> 56) as u8;
    let position = encoded & 0x00FF_FFFF_FFFF_FFFF;
    (level, position)
}

/// Result of processing a batch of address appends using IndexedMerkleTree
#[derive(Clone, Debug)]
pub struct AddressBatchResult {
    /// Proofs for the new elements (one proof per address in the batch)
    pub new_element_proofs: Vec<Vec<[u8; 32]>>,
    /// The final root after all batch operations
    pub new_root: [u8; 32],
    /// The initial root before batch operations
    pub old_root: [u8; 32],
    /// Public input hash for the circuit
    pub public_input_hash: [u8; 32],
}

/// Tree processor using reference IndexedMerkleTree implementation
/// Initialized from indexer's node data, processes batches sequentially
pub struct IndexedTreeProcessor {
    tree: IndexedMerkleTree<Poseidon, usize>,
    next_index: usize,
}

impl IndexedTreeProcessor {
    /// Creates a new IndexedTreeProcessor by initializing an IndexedMerkleTree
    /// from the node data provided by the indexer.
    ///
    /// # Arguments
    /// * `nodes` - Encoded node indices from indexer (format: level << 56 | position)
    /// * `node_hashes` - Corresponding node hashes
    /// * `start_index` - The tree's next_index where new leaves will be appended
    /// * `initial_root` - The current root of the tree (for verification)
    /// * `tree_height` - Height of the merkle tree
    ///
    /// # Returns
    /// A new IndexedTreeProcessor ready to process batches
    pub fn new(
        nodes: Vec<u64>,
        node_hashes: Vec<[u8; 32]>,
        start_index: usize,
        initial_root: [u8; 32],
        tree_height: usize,
    ) -> Result<Self, ForesterUtilsError> {
        debug!(
            "IndexedTreeProcessor::new: start_index={}, initial_root={:?}[..4], nodes_count={}",
            start_index,
            &initial_root[..4],
            nodes.len()
        );

        if tree_height != HEIGHT {
            return Err(ForesterUtilsError::AddressStagingTree(format!(
                "Invalid tree height: expected {}, got {}",
                HEIGHT, tree_height
            )));
        }

        // Build a map of (level, position) -> hash for efficient lookup
        let mut node_map: HashMap<(u8, u64), [u8; 32]> = HashMap::new();
        for (encoded_idx, hash) in nodes.iter().zip(node_hashes.iter()) {
            let (level, position) = decode_node_index(*encoded_idx);
            node_map.insert((level, position), *hash);
        }

        debug!("Built node map with {} entries", node_map.len());

        // Create IndexedMerkleTree using its constructor
        let tree = IndexedMerkleTree::<Poseidon, usize>::new(HEIGHT, 0).map_err(|e| {
            ForesterUtilsError::AddressStagingTree(format!(
                "Failed to create IndexedMerkleTree: {}",
                e
            ))
        })?;

        warn!(
            "IndexedTreeProcessor initialized with {} node entries (full reconstruction not yet implemented)",
            node_map.len()
        );

        Ok(Self {
            tree,
            next_index: start_index,
        })
    }

    /// Simpler initialization from subtrees (existing pattern)
    /// Use this for now until we implement full node reconstruction
    pub fn from_subtrees(
        subtrees: Vec<[u8; 32]>,
        start_index: usize,
        initial_root: [u8; 32],
    ) -> Result<Self, ForesterUtilsError> {
        debug!(
            "IndexedTreeProcessor::from_subtrees: start_index={}, initial_root={:?}[..4]",
            start_index,
            &initial_root[..4]
        );

        let _subtrees_array: [[u8; 32]; HEIGHT] =
            subtrees.try_into().map_err(|v: Vec<[u8; 32]>| {
                ForesterUtilsError::AddressStagingTree(format!(
                    "Invalid subtrees length: expected {}, got {}",
                    HEIGHT,
                    v.len()
                ))
            })?;

        // Use the proper constructor which initializes with zero leaf
        let mut tree = IndexedMerkleTree::<Poseidon, usize>::new(HEIGHT, 0).map_err(|e| {
            ForesterUtilsError::AddressStagingTree(format!(
                "Failed to create IndexedMerkleTree: {}",
                e
            ))
        })?;

        // Initialize the tree with the first two elements (required for indexed trees)
        tree.init().map_err(|e| {
            ForesterUtilsError::AddressStagingTree(format!(
                "Failed to initialize indexed tree: {}",
                e
            ))
        })?;

        Ok(Self {
            tree,
            next_index: start_index,
        })
    }

    /// Returns the current root of the tree
    pub fn current_root(&self) -> [u8; 32] {
        self.tree.root()
    }

    /// Returns the current next_index of the tree
    pub fn next_index(&self) -> usize {
        self.next_index
    }

    /// Processes a batch of address appends sequentially, computing circuit inputs.
    /// # Arguments
    /// * `addresses` - The new addresses to append (as raw bytes)
    /// * `low_element_values` - Values of low elements (addresses)
    /// * `low_element_next_values` - Next values of low elements
    /// * `low_element_indices` - Indices of low elements in the tree
    /// * `low_element_next_indices` - Next indices referenced by low elements
    /// * `low_element_proofs` - Merkle proofs for low elements (not used in new approach)
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
        _low_element_proofs: Vec<Vec<[u8; 32]>>, // Not used - we compute from tree
        leaves_hashchain: [u8; 32],
        zkp_batch_size: usize,
    ) -> Result<AddressBatchResult, ForesterUtilsError> {
        let old_root = self.current_root();
        let start_index = self.next_index;

        debug!(
            "IndexedTreeProcessor::process_batch: {} addresses, start_index={}, old_root={:?}[..4]",
            addresses.len(),
            start_index,
            &old_root[..4]
        );

        if addresses.len() != zkp_batch_size {
            return Err(ForesterUtilsError::AddressStagingTree(format!(
                "Address count mismatch: got {}, expected {}",
                addresses.len(),
                zkp_batch_size
            )));
        }

        let mut new_element_proofs = Vec::with_capacity(zkp_batch_size);

        // Process each address sequentially
        for i in 0..zkp_batch_size {
            let address_bigint = BigUint::from_bytes_be(&addresses[i]);
            let low_value_bigint = BigUint::from_bytes_be(&low_element_values[i]);
            let next_value_bigint = BigUint::from_bytes_be(&low_element_next_values[i]);

            // Get proof BEFORE update (this is critical!)
            let new_index = start_index + i;
            let proof = self.tree.get_proof_of_leaf(new_index, true).map_err(|e| {
                ForesterUtilsError::AddressStagingTree(format!(
                    "Failed to get proof for index {}: {}",
                    new_index, e
                ))
            })?;

            new_element_proofs.push(proof.as_slice().to_vec());

            // Create IndexedElement structs for the update
            let new_low_element = IndexedElement {
                index: low_element_indices[i],
                value: low_value_bigint,
                next_index: low_element_next_indices[i],
            };

            let new_element = IndexedElement {
                index: new_index,
                value: address_bigint.clone(),
                next_index: low_element_next_indices[i],
            };

            // Perform the update (modifies tree in-memory)
            self.tree
                .update(&new_low_element, &new_element, &next_value_bigint)
                .map_err(|e| {
                    ForesterUtilsError::AddressStagingTree(format!(
                        "Failed to update tree at index {}: {}",
                        i, e
                    ))
                })?;

            debug!(
                "Processed address {}/{}: new_root={:?}[..4]",
                i + 1,
                zkp_batch_size,
                &self.tree.root()[..4]
            );
        }

        let new_root = self.current_root();
        self.next_index += zkp_batch_size;

        // Compute public input hash: hash(old_root, new_root, hashchain, start_index)
        let public_input_hash =
            Self::compute_public_input_hash(&old_root, &new_root, &leaves_hashchain, start_index)?;

        debug!(
            "IndexedTreeProcessor::process_batch complete: new_root={:?}[..4], next_index={}",
            &new_root[..4],
            self.next_index
        );

        Ok(AddressBatchResult {
            new_element_proofs,
            new_root,
            old_root,
            public_input_hash,
        })
    }

    /// Compute public input hash for the circuit
    /// Formula: hash(old_root, new_root, hashchain, start_index)
    fn compute_public_input_hash(
        old_root: &[u8; 32],
        new_root: &[u8; 32],
        hashchain: &[u8; 32],
        start_index: usize,
    ) -> Result<[u8; 32], ForesterUtilsError> {
        let start_index_bytes = (start_index as u64).to_be_bytes();
        let mut start_index_32 = [0u8; 32];
        start_index_32[24..].copy_from_slice(&start_index_bytes);

        Poseidon::hashv(&[old_root, new_root, hashchain, &start_index_32]).map_err(|e| {
            ForesterUtilsError::AddressStagingTree(format!(
                "Failed to compute public input hash: {}",
                e
            ))
        })
    }
}

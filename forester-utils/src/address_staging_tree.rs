use light_batched_merkle_tree::constants::DEFAULT_BATCH_ADDRESS_TREE_HEIGHT;
use light_hasher::{bigint::bigint_to_be_bytes_array, Poseidon};
use light_prover_client::proof_types::batch_address_append::{
    get_batch_address_append_circuit_inputs, BatchAddressAppendInputs,
};
use light_sparse_merkle_tree::{
    changelog::ChangelogEntry, indexed_changelog::IndexedChangelogEntry, SparseMerkleTree,
};

use crate::error::ForesterUtilsError;

const HEIGHT: usize = DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize;

#[derive(Clone, Debug)]
pub struct AddressBatchResult {
    pub circuit_inputs: BatchAddressAppendInputs,
    pub new_root: [u8; 32],
    pub old_root: [u8; 32],
}

/// Simple address staging tree using SparseMerkleTree for new element proofs.
///
/// This implementation delegates to the reference `get_batch_address_append_circuit_inputs`
/// which has been battle-tested in the prover client. The key insight is:
///
/// - Low element proofs come from the indexer (accurate proofs for existing leaves)
/// - New element proofs come from the sparse tree's `append()` method
/// - ProofCache (inside the reference impl) patches both to account for batch updates
///
/// This is much simpler and more reliable than trying to maintain a full MerkleTree
/// and manually track all the intermediate states.
#[derive(Clone, Debug)]
pub struct AddressStagingTree {
    /// Sparse tree for generating new element (append) proofs.
    sparse_tree: SparseMerkleTree<Poseidon, HEIGHT>,
    /// Changelog for proof patching within and across batches.
    changelog: Vec<ChangelogEntry<HEIGHT>>,
    /// Indexed changelog for patching low element values.
    indexed_changelog: Vec<IndexedChangelogEntry<usize, HEIGHT>>,
    current_root: [u8; 32],
    next_index: usize,
}

impl AddressStagingTree {
    /// Create a new address staging tree from subtrees (frontier).
    ///
    /// The subtrees must come from the indexer and represent the current state
    /// of the on-chain merkle tree at `start_index`.
    pub fn new(
        subtrees: [[u8; 32]; HEIGHT],
        initial_root: [u8; 32],
        start_index: usize,
    ) -> Result<Self, ForesterUtilsError> {
        let sparse_tree = SparseMerkleTree::<Poseidon, HEIGHT>::new(subtrees, start_index);

        // Validate that the sparse tree's computed root matches the expected root
        let computed_root = sparse_tree.root();
        if computed_root != initial_root {
            return Err(ForesterUtilsError::AddressStagingTree(format!(
                "Sparse tree root mismatch: computed {:?}[..4] != expected {:?}[..4] \
                 (start_index={}). The subtrees from indexer may be stale.",
                &computed_root[..4],
                &initial_root[..4],
                start_index
            )));
        }

        tracing::debug!(
            "AddressStagingTree::new: start_index={}, root={:?}[..4]",
            start_index,
            &initial_root[..4]
        );

        Ok(Self {
            sparse_tree,
            changelog: Vec::new(),
            indexed_changelog: Vec::new(),
            current_root: initial_root,
            next_index: start_index,
        })
    }

    /// Create from indexer response with optional subtrees.
    ///
    /// This is the main entry point used by the forester. If subtrees are provided,
    /// we use them to initialize the sparse tree. The nodes/node_hashes parameters
    /// are ignored (they were used by the old MerkleTree-based approach).
    pub fn from_nodes(
        _nodes: &[u64],
        _node_hashes: &[[u8; 32]],
        initial_root: [u8; 32],
        start_index: usize,
        subtrees: Option<[[u8; 32]; HEIGHT]>,
    ) -> Result<Self, ForesterUtilsError> {
        match subtrees {
            Some(st) => Self::new(st, initial_root, start_index),
            None => Err(ForesterUtilsError::AddressStagingTree(
                "Subtrees are required for address staging tree. \
                 The indexer must provide subtrees (frontier) for efficient proof generation."
                    .to_string(),
            )),
        }
    }

    pub fn current_root(&self) -> [u8; 32] {
        self.current_root
    }

    pub fn next_index(&self) -> usize {
        self.next_index
    }

    pub fn clear_changelogs(&mut self) {
        self.changelog.clear();
        self.indexed_changelog.clear();
    }

    /// Process a batch of address appends using the reference implementation.
    ///
    /// This delegates to `get_batch_address_append_circuit_inputs` which handles:
    /// - Patching low element proofs via indexed_changelog
    /// - Getting new element proofs from sparse_tree.append()
    /// - Patching both proofs via proof_cache for batch updates
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
        let old_root = self.current_root;
        let next_index = self.next_index;

        tracing::debug!(
            "AddressStagingTree::process_batch: next_index={}, zkp_batch_size={}, \
             changelog_len={}, indexed_changelog_len={}, addresses_len={}, epoch={}, tree={}",
            next_index,
            zkp_batch_size,
            self.changelog.len(),
            self.indexed_changelog.len(),
            addresses.len(),
            epoch,
            tree
        );

        // Delegate to the reference implementation
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
            &mut self.sparse_tree,
            leaves_hashchain,
            zkp_batch_size,
            &mut self.changelog,
            &mut self.indexed_changelog,
        )
        .map_err(|e| {
            ForesterUtilsError::AddressStagingTree(format!(
                "Failed to build circuit inputs: {} (next_index={}, epoch={}, tree={})",
                e, next_index, epoch, tree
            ))
        })?;

        let new_root = bigint_to_be_bytes_array::<32>(&inputs.new_root).map_err(|e| {
            ForesterUtilsError::AddressStagingTree(format!("Failed to serialize new root: {}", e))
        })?;

        self.current_root = new_root;
        self.next_index += zkp_batch_size;

        tracing::debug!(
            "ADDRESS_APPEND batch complete: {:?}[..4] -> {:?}[..4] \
             (batch_size={}, next_index={}, epoch={}, tree={})",
            &old_root[..4],
            &new_root[..4],
            zkp_batch_size,
            self.next_index,
            epoch,
            tree
        );

        Ok(AddressBatchResult {
            circuit_inputs: inputs,
            new_root,
            old_root,
        })
    }
}

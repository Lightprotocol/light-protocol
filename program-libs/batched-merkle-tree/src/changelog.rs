use light_compressed_account::instruction_data::compressed_proof::CompressedProof;

use crate::{BorshDeserialize, BorshSerialize};

/// Changelog entry to support out-of-order merkle tree updates.
/// Holds the necessary information to update a tree when its turn comes.
#[repr(C)]
#[derive(
    Debug, 
    PartialEq, 
    Clone, 
    Copy, 
    BorshDeserialize, 
    BorshSerialize,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::KnownLayout,
    zerocopy::Immutable,
)]
#[cfg_attr(feature = "aligned-sized", aligned_sized::aligned_sized(anchor))]
pub struct BatchChangelog {
    /// Previous root of the tree (must match current root when applying)
    pub old_root: [u8; 32],
    /// New root to replace the old root
    pub new_root: [u8; 32],
    /// Leaf hash chain used to track the sequence of updates
    pub leaves_hash_chain: [u8; 32],
    /// Hash chain index for this update
    pub hash_chain_index: u16,
    /// Pending batch index for this update
    pub pending_batch_index: u8,
    /// Padding to ensure alignment
    pub _padding: [u8; 5],
    /// Expected sequence number for this update
    pub expected_seq: u64,
}

/// Public inputs for batched merkle tree updates, with support for out-of-order processing.
/// Adds fields required for changelog management to the standard instruction data.
#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy, BorshDeserialize, BorshSerialize, Default)]
pub struct ChangelogInstructionData {
    /// New root to update to
    pub new_root: [u8; 32],
    /// Previous root of the tree, used for validating ordered updates
    pub old_root: [u8; 32],
    /// Index in the hash chain array to identify which update this is for
    pub hash_chain_index: u16,
    /// ZKP proof data
    pub compressed_proof: CompressedProof,
}

// Create utility functions for working with changelogs

/// Check if the changelog entry is applicable to the current state
#[cfg(feature = "test-only")]
pub fn is_applicable_entry(
    entry: &BatchChangelog, 
    current_root: &[u8; 32], 
    current_seq: u64
) -> bool {
    entry.old_root == *current_root && entry.expected_seq == current_seq
}

/// Find applicable entries in a slice of changelog entries
#[cfg(feature = "test-only")]
pub fn find_applicable_entries<'a>(
    entries: &'a [BatchChangelog],
    current_root: &[u8; 32],
    current_seq: u64
) -> Vec<&'a BatchChangelog> {
    entries.iter()
        .filter(|entry| is_applicable_entry(entry, current_root, current_seq))
        .collect()
}

/// Create a changelog entry from instruction data and current state
#[cfg(feature = "test-only")]
pub fn create_entry(
    instruction_data: &ChangelogInstructionData,
    pending_batch_index: u8,
    leaves_hash_chain: [u8; 32],
    current_seq: u64,
) -> BatchChangelog {
    BatchChangelog {
        old_root: instruction_data.old_root,
        new_root: instruction_data.new_root,
        leaves_hash_chain,
        hash_chain_index: instruction_data.hash_chain_index,
        pending_batch_index,
        _padding: [0u8; 5],
        expected_seq: current_seq,
    }
}
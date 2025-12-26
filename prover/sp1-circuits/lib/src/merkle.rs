//! Merkle tree utilities for SP1 batch circuits.
//!
//! Implements the same logic as Gnark's circuit_utils.go.

use crate::poseidon::{poseidon2, Hash};
use light_poseidon::PoseidonError;

/// Compute parent hash from a node and its sibling.
///
/// Matches Gnark's `ProveParentHash` gadget.
///
/// # Arguments
/// * `bit` - Path bit: 0 means node is left child, 1 means node is right child
/// * `hash` - The current node hash
/// * `sibling` - The sibling node hash
///
/// # Returns
/// The parent hash computed as:
/// - If bit == 0: H(hash, sibling)
/// - If bit == 1: H(sibling, hash)
pub fn prove_parent_hash(bit: bool, hash: &Hash, sibling: &Hash) -> Result<Hash, PoseidonError> {
    if bit {
        // Node is right child: parent = H(sibling, hash)
        poseidon2(sibling, hash)
    } else {
        // Node is left child: parent = H(hash, sibling)
        poseidon2(hash, sibling)
    }
}

/// Compute Merkle root from leaf and proof path.
///
/// Matches Gnark's `MerkleRootGadget`.
///
/// # Arguments
/// * `leaf` - The leaf hash
/// * `path_indices` - Bits indicating left/right at each level (LSB first)
/// * `proof` - Sibling hashes at each level
///
/// # Returns
/// The computed Merkle root
pub fn compute_merkle_root(
    leaf: &Hash,
    path_indices: &[bool],
    proof: &[Hash],
) -> Result<Hash, PoseidonError> {
    assert_eq!(
        path_indices.len(),
        proof.len(),
        "path_indices and proof must have same length"
    );

    let mut current_hash = *leaf;
    for (i, &bit) in path_indices.iter().enumerate() {
        current_hash = prove_parent_hash(bit, &current_hash, &proof[i])?;
    }
    Ok(current_hash)
}

/// Verify old leaf and compute new root.
///
/// Matches Gnark's `MerkleRootUpdateGadget`.
///
/// # Arguments
/// * `old_root` - Expected old root
/// * `old_leaf` - The old leaf value at this position
/// * `new_leaf` - The new leaf value to insert
/// * `path_indices` - Bits indicating the position in the tree
/// * `proof` - Merkle proof (sibling hashes)
///
/// # Returns
/// * `Ok(new_root)` if old_leaf produces old_root with the given proof
/// * `Err` if verification fails or hash computation fails
pub fn merkle_root_update(
    old_root: &Hash,
    old_leaf: &Hash,
    new_leaf: &Hash,
    path_indices: &[bool],
    proof: &[Hash],
) -> Result<Hash, MerkleError> {
    // Verify old root
    let computed_old_root = compute_merkle_root(old_leaf, path_indices, proof)?;
    if computed_old_root != *old_root {
        return Err(MerkleError::RootMismatch {
            expected: *old_root,
            computed: computed_old_root,
        });
    }

    // Compute new root
    let new_root = compute_merkle_root(new_leaf, path_indices, proof)?;
    Ok(new_root)
}

/// Convert a u32 index to path bits (little-endian, LSB first).
///
/// # Arguments
/// * `index` - The leaf index
/// * `height` - The tree height (number of bits needed)
///
/// # Returns
/// Vector of bits representing the path from leaf to root
pub fn index_to_path_bits(index: u32, height: usize) -> Vec<bool> {
    (0..height).map(|i| ((index >> i) & 1) == 1).collect()
}

/// Compute leaf hash for indexed Merkle tree with range validation.
///
/// Matches Gnark's `LeafHashGadget`.
///
/// # Arguments
/// * `low_value` - The lower bound value
/// * `high_value` - The upper bound value (next value in the ordered list)
/// * `new_value` - The new value being inserted
///
/// # Returns
/// * `Ok(hash)` if low_value < new_value < high_value
/// * `Err` if range check fails
///
/// The returned hash is `H(low_value, high_value)`.
pub fn leaf_hash_with_range_check(
    low_value: &Hash,
    high_value: &Hash,
    new_value: &Hash,
) -> Result<Hash, MerkleError> {
    // Check: low_value < new_value < high_value
    // We compare as big-endian 256-bit integers
    if !is_less_than(low_value, new_value) {
        return Err(MerkleError::RangeCheckFailed {
            message: "low_value must be less than new_value".to_string(),
        });
    }
    if !is_less_than(new_value, high_value) {
        return Err(MerkleError::RangeCheckFailed {
            message: "new_value must be less than high_value".to_string(),
        });
    }

    // Return H(low_value, high_value)
    poseidon2(low_value, high_value).map_err(MerkleError::from)
}

/// Compare two 32-byte values as big-endian 256-bit unsigned integers.
/// Returns true if a < b.
fn is_less_than(a: &Hash, b: &Hash) -> bool {
    for i in 0..32 {
        if a[i] < b[i] {
            return true;
        }
        if a[i] > b[i] {
            return false;
        }
    }
    false // a == b
}

#[derive(Debug, Clone, PartialEq)]
pub enum MerkleError {
    /// The computed root doesn't match the expected root
    RootMismatch { expected: Hash, computed: Hash },
    /// Range check failed for indexed Merkle tree
    RangeCheckFailed { message: String },
    /// Poseidon hash error
    PoseidonError(String),
}

impl From<PoseidonError> for MerkleError {
    fn from(e: PoseidonError) -> Self {
        MerkleError::PoseidonError(format!("{:?}", e))
    }
}

impl std::fmt::Display for MerkleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MerkleError::RootMismatch { expected, computed } => {
                write!(
                    f,
                    "Root mismatch: expected {:?}, computed {:?}",
                    expected, computed
                )
            }
            MerkleError::RangeCheckFailed { message } => {
                write!(f, "Range check failed: {}", message)
            }
            MerkleError::PoseidonError(e) => write!(f, "Poseidon error: {}", e),
        }
    }
}

impl std::error::Error for MerkleError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_to_path_bits() {
        // Index 5 = 0b101 at height 4 = [1, 0, 1, 0] (LSB first)
        let bits = index_to_path_bits(5, 4);
        assert_eq!(bits, vec![true, false, true, false]);

        // Index 0 at height 3 = [0, 0, 0]
        let bits = index_to_path_bits(0, 3);
        assert_eq!(bits, vec![false, false, false]);

        // Index 7 = 0b111 at height 3 = [1, 1, 1]
        let bits = index_to_path_bits(7, 3);
        assert_eq!(bits, vec![true, true, true]);
    }

    #[test]
    fn test_prove_parent_hash() {
        let hash = [1u8; 32];
        let sibling = [2u8; 32];

        // Test left child (bit = 0): H(hash, sibling)
        let parent_left = prove_parent_hash(false, &hash, &sibling).unwrap();
        let expected_left = poseidon2(&hash, &sibling).unwrap();
        assert_eq!(parent_left, expected_left);

        // Test right child (bit = 1): H(sibling, hash)
        let parent_right = prove_parent_hash(true, &hash, &sibling).unwrap();
        let expected_right = poseidon2(&sibling, &hash).unwrap();
        assert_eq!(parent_right, expected_right);

        // Parents should be different since order matters
        assert_ne!(parent_left, parent_right);
    }

    #[test]
    fn test_merkle_root_update_valid() {
        // Create a simple tree with height 2
        let leaf = [1u8; 32];
        let sibling_l0 = [2u8; 32];
        let sibling_l1 = [3u8; 32];

        let proof = vec![sibling_l0, sibling_l1];
        let path_indices = vec![false, false]; // Index 0

        // Compute the old root
        let old_root = compute_merkle_root(&leaf, &path_indices, &proof).unwrap();

        // Update with new leaf
        let new_leaf = [4u8; 32];
        let new_root =
            merkle_root_update(&old_root, &leaf, &new_leaf, &path_indices, &proof).unwrap();

        // Verify new root is different
        assert_ne!(old_root, new_root);

        // Verify new root is correct
        let expected_new_root = compute_merkle_root(&new_leaf, &path_indices, &proof).unwrap();
        assert_eq!(new_root, expected_new_root);
    }

    #[test]
    fn test_merkle_root_update_invalid() {
        let leaf = [1u8; 32];
        // Use a small value to stay within BN254 modulus
        let wrong_leaf = [5u8; 32];
        let sibling = [2u8; 32];

        let proof = vec![sibling];
        let path_indices = vec![false];

        // Compute the old root with correct leaf
        let old_root = compute_merkle_root(&leaf, &path_indices, &proof).unwrap();

        // Try to update with wrong old leaf - should fail
        let new_leaf = [4u8; 32];
        let result =
            merkle_root_update(&old_root, &wrong_leaf, &new_leaf, &path_indices, &proof);

        assert!(matches!(result, Err(MerkleError::RootMismatch { .. })));
    }

    #[test]
    fn test_leaf_hash_with_range_check_valid() {
        // low < new < high
        let low = [1u8; 32];
        let new_val = [2u8; 32];
        let high = [3u8; 32];

        let result = leaf_hash_with_range_check(&low, &high, &new_val);
        assert!(result.is_ok());

        // The hash should be H(low, high)
        let expected = poseidon2(&low, &high).unwrap();
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_leaf_hash_with_range_check_low_violation() {
        // new < low - should fail
        let low = [2u8; 32];
        let new_val = [1u8; 32];
        let high = [3u8; 32];

        let result = leaf_hash_with_range_check(&low, &high, &new_val);
        assert!(matches!(result, Err(MerkleError::RangeCheckFailed { .. })));
    }

    #[test]
    fn test_leaf_hash_with_range_check_high_violation() {
        // new > high - should fail
        let low = [1u8; 32];
        let new_val = [4u8; 32];
        let high = [3u8; 32];

        let result = leaf_hash_with_range_check(&low, &high, &new_val);
        assert!(matches!(result, Err(MerkleError::RangeCheckFailed { .. })));
    }

    #[test]
    fn test_is_less_than() {
        // Create values where comparison matters
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];

        // a[0] = 0, b[0] = 1 => a < b
        b[0] = 1;
        assert!(is_less_than(&a, &b));
        assert!(!is_less_than(&b, &a));

        // Equal values
        assert!(!is_less_than(&a, &a));

        // Different at last byte
        a[31] = 5;
        b = [0u8; 32];
        b[31] = 10;
        assert!(is_less_than(&a, &b));
    }
}

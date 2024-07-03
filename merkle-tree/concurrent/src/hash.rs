use light_bounded_vec::BoundedVec;
use light_hasher::Hasher;

use crate::errors::ConcurrentMerkleTreeError;

/// Returns the hash of the parent node based on the provided `node` (with its
/// `node_index`) and `sibling` (with its `sibling_index`).
pub fn compute_parent_node<H>(
    node: &[u8; 32],
    sibling: &[u8; 32],
    node_index: usize,
    level: usize,
) -> Result<[u8; 32], ConcurrentMerkleTreeError>
where
    H: Hasher,
{
    let is_left = (node_index >> level) & 1 == 0;
    let hash = if is_left {
        H::hashv(&[node, sibling])?
    } else {
        H::hashv(&[sibling, node])?
    };
    Ok(hash)
}

/// Computes the root for the given `leaf` (with index `i`) and `proof`. It
/// doesn't perform the validation of the provided `proof`.
pub fn compute_root<H>(
    leaf: &[u8; 32],
    leaf_index: usize,
    proof: &BoundedVec<[u8; 32]>,
) -> Result<[u8; 32], ConcurrentMerkleTreeError>
where
    H: Hasher,
{
    let mut node = *leaf;
    for (level, sibling) in proof.iter().enumerate() {
        node = compute_parent_node::<H>(&node, sibling, leaf_index, level)?;
    }
    Ok(node)
}

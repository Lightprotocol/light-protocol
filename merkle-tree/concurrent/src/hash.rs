use light_hasher::Hasher;

use crate::errors::ConcurrentMerkleTreeError;

/// Returns the hash of the parent node based on the provided `node` (with its
/// `node_index`) and `sibling` (with its `sibling_index`).
pub fn compute_parent_node<H>(
    node: &[u8; 32],
    sibling: &[u8; 32],
    node_index: usize,
    sibling_index: usize,
) -> Result<[u8; 32], ConcurrentMerkleTreeError>
where
    H: Hasher,
{
    let is_left = (node_index >> sibling_index) & 1 == 0;
    let hash = if is_left {
        H::hashv(&[node, sibling])?
    } else {
        H::hashv(&[sibling, node])?
    };
    Ok(hash)
}

/// Computes the root for the given `leaf` (with index `i`) and `proof`. It
/// doesn't perform the validation of the provided `proof`.
pub fn compute_root<H, const HEIGHT: usize>(
    leaf: &[u8; 32],
    leaf_index: usize,
    proof: &[[u8; 32]; HEIGHT],
) -> Result<[u8; 32], ConcurrentMerkleTreeError>
where
    H: Hasher,
{
    let mut node = *leaf;
    for (j, sibling) in proof.iter().enumerate() {
        node = compute_parent_node::<H>(&node, sibling, leaf_index, j)?;
    }
    Ok(node)
}

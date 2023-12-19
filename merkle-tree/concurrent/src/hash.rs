use light_hasher::{errors::HasherError, Hasher};

/// Returns the hash of the parent node based on the provided `node` (and its
/// index `i_node`) and `sibling` (and its index `i_sibling`).
pub fn compute_parent_node<H>(
    leaf: &[u8; 32],
    sibling: &[u8; 32],
    leaf_index: usize,
    sibling_index: usize,
) -> Result<[u8; 32], HasherError>
where
    H: Hasher,
{
    let is_left = (leaf_index >> sibling_index) & 1 == 0;
    if is_left {
        H::hashv(&[leaf, sibling])
    } else {
        H::hashv(&[sibling, leaf])
    }
}

/// Computes the root for the given `leaf` (with index `i`) and `proof`. It
/// doesn't perform the validation of the provided `proof`.
pub fn compute_root<H, const MAX_HEIGHT: usize>(
    leaf: &[u8; 32],
    leaf_index: usize,
    proof: &[[u8; 32]; MAX_HEIGHT],
) -> Result<[u8; 32], HasherError>
where
    H: Hasher,
{
    let mut leaf = *leaf;
    for (j, sibling) in proof.iter().enumerate() {
        leaf = compute_parent_node::<H>(&leaf, sibling, leaf_index, j)?;
    }
    Ok(leaf)
}

/// Checks whether the given Merkle `proof` for the given `node` (with index
/// `i`) is valid. The proof is valid when computing parent node hashes using
/// the whole path of the proof gives the same result as the given `root`.
pub fn validate_proof<H, const MAX_HEIGHT: usize>(
    root: &[u8; 32],
    leaf: &[u8; 32],
    leaf_index: usize,
    proof: &[[u8; 32]; MAX_HEIGHT],
) -> Result<(), HasherError>
where
    H: Hasher,
{
    let computed_root = compute_root::<H, MAX_HEIGHT>(leaf, leaf_index, proof)?;
    if computed_root == *root {
        Ok(())
    } else {
        Err(HasherError::InvalidProof)
    }
}

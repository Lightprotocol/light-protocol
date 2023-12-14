use light_hasher::{errors::HasherError, Hasher};

/// Returns the hash of the parent node based on the provided `node` (and its
/// index `i_node`) and `sibling` (and its index `i_sibling`).
pub fn compute_parent_node<H>(
    node: &[u8; 32],
    sibling: &[u8; 32],
    i_node: usize,
    i_sibling: usize,
) -> Result<[u8; 32], HasherError>
where
    H: Hasher,
{
    let is_left = (i_node >> i_sibling) & 1 == 0;
    if is_left {
        H::hashv(&[node, sibling])
    } else {
        H::hashv(&[sibling, node])
    }
}

/// Computes the root for the given `leaf` (with index `i`) and `proof`. It
/// doesn't perform the validation of the provided `proof`.
pub fn compute_root<H, const MAX_HEIGHT: usize>(
    mut leaf: [u8; 32],
    i: usize,
    proof: &[[u8; 32]; MAX_HEIGHT],
) -> Result<[u8; 32], HasherError>
where
    H: Hasher,
{
    for (j, sibling) in proof.iter().enumerate() {
        leaf = compute_parent_node::<H>(&leaf, sibling, i, j)?;
    }
    Ok(leaf)
}

/// Checks whether the given Merkle `proof` for the given `node` (with index
/// `i`) is valid. The proof is valid when computing parent node hashes using
/// the whole path of the proof gives the same result as the given `root`.
pub fn is_valid_proof<H, const MAX_HEIGHT: usize>(
    root: [u8; 32],
    node: [u8; 32],
    i: usize,
    proof: &[[u8; 32]; MAX_HEIGHT],
) -> Result<bool, HasherError>
where
    H: Hasher,
{
    let computed_root = compute_root::<H, MAX_HEIGHT>(node, i, proof)?;
    Ok(computed_root == root)
}

/// Fills up proof with empty nodes.
pub fn full_proof<H, const MAX_HEIGHT: usize>(proof: &[[u8; 32]]) -> [[u8; 32]; MAX_HEIGHT]
where
    H: Hasher,
{
    let mut full_proof: [[u8; 32]; MAX_HEIGHT] = [[0u8; 32]; MAX_HEIGHT];

    full_proof[..proof.len()].copy_from_slice(proof);
    for (i, item) in full_proof
        .iter_mut()
        .enumerate()
        .take(MAX_HEIGHT)
        .skip(proof.len())
    {
        *item = H::zero_bytes()[i];
    }

    full_proof
}

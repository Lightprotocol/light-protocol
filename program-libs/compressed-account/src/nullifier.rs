use light_hasher::{Hasher, Poseidon};

use crate::CompressedAccountError;

/// Nullifer is a poseidon hash:
/// H(account_hash, leaf_index, tx_hash)
pub fn create_nullifier(
    account_hash: &[u8; 32],
    leaf_index: u64,
    tx_hash: &[u8; 32],
) -> Result<[u8; 32], CompressedAccountError> {
    let mut leaf_index_bytes = [0u8; 32];
    leaf_index_bytes[24..].copy_from_slice(leaf_index.to_be_bytes().as_slice());
    // Inclusion of the tx_hash enables zk proofs of how a value was spent.
    let nullifier = Poseidon::hashv(&[account_hash.as_slice(), &leaf_index_bytes, tx_hash])?;
    Ok(nullifier)
}

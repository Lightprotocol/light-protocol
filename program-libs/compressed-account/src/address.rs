use light_hasher::hash_to_field_size::hashv_to_bn254_field_size_be_const_array;

use crate::{CompressedAccountError, Pubkey};

pub fn derive_address_legacy(
    merkle_tree_pubkey: &Pubkey,
    seed: &[u8; 32],
) -> Result<[u8; 32], CompressedAccountError> {
    let slices = [merkle_tree_pubkey.as_ref(), seed.as_ref()];
    let hash = hashv_to_bn254_field_size_be_const_array::<3>(&slices)?;
    Ok(hash)
}

pub fn derive_address(
    seed: &[u8; 32],
    merkle_tree_pubkey: &[u8; 32],
    program_id_bytes: &[u8; 32],
) -> [u8; 32] {
    let slices = [
        seed.as_slice(),
        merkle_tree_pubkey.as_slice(),
        program_id_bytes.as_slice(),
    ];
    hashv_to_bn254_field_size_be_const_array::<4>(&slices)
        .expect("hashv_to_bn254_field_size_be_const_array::<4> should be infallible for Keccak")
}

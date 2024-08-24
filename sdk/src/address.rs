use anchor_lang::solana_program::pubkey::Pubkey;
use light_utils::hashv_to_bn254_field_size_be;

use crate::merkle_context::AddressMerkleContext;

/// Derives a single address seed for a compressed account, based on the
/// provided multiple `seeds`, `program_id` and `merkle_tree_pubkey`.
///
/// # Examples
///
/// ```ignore
/// use light_sdk::{address::derive_address, pubkey};
///
/// let address = derive_address(
///     &[b"my_compressed_account"],
///     &crate::ID,
///     &address_merkle_context,
/// );
/// ```
pub fn derive_address_seed(
    seeds: &[&[u8]],
    program_id: &Pubkey,
    address_merkle_context: &AddressMerkleContext,
) -> [u8; 32] {
    let mut inputs = Vec::with_capacity(seeds.len() + 2);

    let program_id = program_id.to_bytes();
    inputs.push(program_id.as_slice());

    let merkle_tree_pubkey = address_merkle_context.address_merkle_tree_pubkey.to_bytes();
    inputs.push(merkle_tree_pubkey.as_slice());

    inputs.extend(seeds);

    let address = hashv_to_bn254_field_size_be(inputs.as_slice());
    address
}

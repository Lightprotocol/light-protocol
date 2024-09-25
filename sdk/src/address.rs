use anchor_lang::{solana_program::pubkey::Pubkey, AnchorDeserialize, AnchorSerialize};
use light_utils::{hash_to_bn254_field_size_be, hashv_to_bn254_field_size_be};

use crate::merkle_context::{AddressMerkleContext, RemainingAccounts};

#[derive(Debug, PartialEq, Default, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct NewAddressParams {
    pub seed: [u8; 32],
    pub address_queue_pubkey: Pubkey,
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_merkle_tree_root_index: u16,
}

#[derive(Debug, PartialEq, Default, Clone, Copy, AnchorDeserialize, AnchorSerialize)]
pub struct NewAddressParamsPacked {
    pub seed: [u8; 32],
    pub address_queue_account_index: u8,
    pub address_merkle_tree_account_index: u8,
    pub address_merkle_tree_root_index: u16,
}

pub struct AddressWithMerkleContext {
    pub address: [u8; 32],
    pub address_merkle_context: AddressMerkleContext,
}

pub fn pack_new_addresses_params(
    addresses_params: &[NewAddressParams],
    remaining_accounts: &mut RemainingAccounts,
) -> Vec<NewAddressParamsPacked> {
    addresses_params
        .iter()
        .map(|x| {
            let address_queue_account_index =
                remaining_accounts.insert_or_get(x.address_queue_pubkey);
            let address_merkle_tree_account_index =
                remaining_accounts.insert_or_get(x.address_merkle_tree_pubkey);
            NewAddressParamsPacked {
                seed: x.seed,
                address_queue_account_index,
                address_merkle_tree_account_index,
                address_merkle_tree_root_index: x.address_merkle_tree_root_index,
            }
        })
        .collect::<Vec<_>>()
}

pub fn pack_new_address_params(
    address_params: NewAddressParams,
    remaining_accounts: &mut RemainingAccounts,
) -> NewAddressParamsPacked {
    pack_new_addresses_params(&[address_params], remaining_accounts)[0]
}

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

/// Derives an address for a compressed account, based on the provided singular
/// `seed` and `address_merkle_context`:
pub fn derive_address(
    address_seed: &[u8; 32],
    address_merkle_context: &AddressMerkleContext,
) -> [u8; 32] {
    let merkle_tree_pubkey = address_merkle_context.address_merkle_tree_pubkey.to_bytes();
    let input = [merkle_tree_pubkey, *address_seed].concat();

    // PANICS: Not being able to find the bump for truncating the hash is
    // practically impossible. Quite frankly, we should just remove that error
    // inside.
    hash_to_bn254_field_size_be(input.as_slice()).unwrap().0
}

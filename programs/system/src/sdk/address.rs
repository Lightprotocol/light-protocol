use anchor_lang::{solana_program::pubkey::Pubkey, Result};
use light_utils::hash_to_bn254_field_size_be;
use std::collections::HashMap;

use crate::{errors::CompressedPdaError, NewAddressParams, NewAddressParamsPacked};
pub fn derive_address(merkle_tree_pubkey: &Pubkey, seed: &[u8]) -> Result<[u8; 32]> {
    let pubkey_bytes = merkle_tree_pubkey.to_bytes();
    let total_length = pubkey_bytes.len() + seed.len();
    let mut bytes_to_hash = Vec::with_capacity(total_length);

    bytes_to_hash.extend_from_slice(&pubkey_bytes);
    bytes_to_hash.extend_from_slice(seed);

    let hash = match hash_to_bn254_field_size_be(bytes_to_hash.as_slice()) {
        Some(hash) => Ok::<[u8; 32], CompressedPdaError>(hash.0),
        None => return Err(CompressedPdaError::DeriveAddressError.into()),
    }?;

    Ok(hash)
}

pub fn add_and_get_remaining_account_indices(
    pubkeys: &[Pubkey],
    remaining_accounts: &mut HashMap<Pubkey, usize>,
) -> Vec<u8> {
    let mut indices = Vec::new();
    let mut next_index: usize = remaining_accounts.len();
    for pubkey in pubkeys.iter() {
        let index = remaining_accounts.entry(*pubkey).or_insert_with(|| {
            let current_index = next_index;
            next_index += 1;
            current_index
        });
        indices.push(*index as u8);
    }
    indices
}

// TODO: Remove from System Program. It is not used in the System Program.
// Helper function to pack new address params for instruction data in rust
pub fn pack_new_address_params(
    new_address_params: &[NewAddressParams],
    remaining_accounts: &mut HashMap<Pubkey, usize>,
) -> Vec<NewAddressParamsPacked> {
    let mut new_address_params_packed = new_address_params
        .iter()
        .map(|x| NewAddressParamsPacked {
            seed: x.seed.clone(),
            address_merkle_tree_root_index: x.address_merkle_tree_root_index,
            address_merkle_tree_account_index: 0, // will be assigned later
            address_queue_account_index: 0,       // will be assigned later
        })
        .collect::<Vec<NewAddressParamsPacked>>();
    let mut next_index: usize = remaining_accounts.len();
    for (i, params) in new_address_params.iter().enumerate() {
        match remaining_accounts.get(&params.address_merkle_tree_pubkey) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(params.address_merkle_tree_pubkey, next_index);
                next_index += 1;
            }
        };
        new_address_params_packed[i].address_merkle_tree_account_index = *remaining_accounts
            .get(&params.address_merkle_tree_pubkey)
            .unwrap()
            as u8;
    }

    for (i, params) in new_address_params.iter().enumerate() {
        match remaining_accounts.get(&params.address_queue_pubkey) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(params.address_queue_pubkey, next_index);
                next_index += 1;
            }
        };
        new_address_params_packed[i].address_queue_account_index = *remaining_accounts
            .get(&params.address_queue_pubkey)
            .unwrap() as u8;
    }
    new_address_params_packed
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::solana_program::pubkey::Pubkey;

    #[test]
    fn test_derive_address_with_valid_input() {
        let merkle_tree_pubkey = Pubkey::new_unique();
        let seeds = [&b"seed1"[..], &b"seed2"[..]];
        let result = derive_address(&merkle_tree_pubkey, &seeds.concat());
        let result_2 = derive_address(&merkle_tree_pubkey, &seeds.concat());
        assert_eq!(result, result_2);
    }

    #[test]
    fn test_derive_address_with_empty_seeds() {
        let merkle_tree_pubkey = Pubkey::new_unique();
        let seeds: Vec<u8> = vec![];
        let result = derive_address(&merkle_tree_pubkey, &seeds);
        let result_2 = derive_address(&merkle_tree_pubkey, &seeds);

        assert_eq!(result, result_2);
    }

    #[test]
    fn test_derive_address_no_collision_same_seeds_diff_order() {
        let merkle_tree_pubkey = Pubkey::new_unique();
        let seeds = [&b"seed1"[..], &b"seed2"[..]];
        let seeds_2 = [&b"seed2"[..], &b"seed1"[..]];

        let result = derive_address(&merkle_tree_pubkey, &seeds.concat());
        let result_2 = derive_address(&merkle_tree_pubkey, &seeds_2.concat());
        assert_ne!(result, result_2);
    }
    #[test]
    fn test_derive_address_no_collision_same_seeds_diff_pubkey() {
        let merkle_tree_pubkey = Pubkey::new_unique();
        let merkle_tree_pubkey_2 = Pubkey::new_unique();
        let seeds = [&b"seed1"[..], &b"seed2"[..]];

        let result = derive_address(&merkle_tree_pubkey, &seeds.concat());
        let result_2 = derive_address(&merkle_tree_pubkey_2, &seeds.concat());
        assert_ne!(result, result_2);
    }

    #[test]
    fn test_add_and_get_remaining_account_indices_empty() {
        let pubkeys = vec![];
        let mut remaining_accounts = HashMap::new();
        let result = add_and_get_remaining_account_indices(&pubkeys, &mut remaining_accounts);
        assert!(result.is_empty());
    }

    #[test]
    fn test_add_and_get_remaining_account_indices_single() {
        let pubkey = Pubkey::new_unique();
        let pubkeys = vec![pubkey];
        let mut remaining_accounts = HashMap::new();
        let result = add_and_get_remaining_account_indices(&pubkeys, &mut remaining_accounts);
        assert_eq!(result, vec![0]);
        assert_eq!(remaining_accounts.get(&pubkey), Some(&0));
    }

    #[test]
    fn test_add_and_get_remaining_account_indices_multiple() {
        let pubkey1 = Pubkey::new_unique();
        let pubkey2 = Pubkey::new_unique();
        let pubkeys = vec![pubkey1, pubkey2];
        let mut remaining_accounts = HashMap::new();
        let result = add_and_get_remaining_account_indices(&pubkeys, &mut remaining_accounts);
        assert_eq!(result, vec![0, 1]);
        assert_eq!(remaining_accounts.get(&pubkey1), Some(&0));
        assert_eq!(remaining_accounts.get(&pubkey2), Some(&1));
    }

    #[test]
    fn test_add_and_get_remaining_account_indices_duplicates() {
        let pubkey = Pubkey::new_unique();
        let pubkeys = vec![pubkey, pubkey];
        let mut remaining_accounts = HashMap::new();
        let result = add_and_get_remaining_account_indices(&pubkeys, &mut remaining_accounts);
        assert_eq!(result, vec![0, 0]);
        assert_eq!(remaining_accounts.get(&pubkey), Some(&0));
        assert_eq!(remaining_accounts.len(), 1);
    }

    #[test]
    fn test_add_and_get_remaining_account_indices_multiple_duplicates() {
        let pubkey1 = Pubkey::new_unique();
        let pubkey2 = Pubkey::new_unique();
        let pubkey3 = Pubkey::new_unique();
        let pubkeys = vec![pubkey1, pubkey2, pubkey1, pubkey3, pubkey2, pubkey1];
        let mut remaining_accounts = HashMap::new();
        let result = add_and_get_remaining_account_indices(&pubkeys, &mut remaining_accounts);
        assert_eq!(result, vec![0, 1, 0, 2, 1, 0]);
        assert_eq!(remaining_accounts.get(&pubkey1), Some(&0));
        assert_eq!(remaining_accounts.get(&pubkey2), Some(&1));
        assert_eq!(remaining_accounts.get(&pubkey3), Some(&2));
        assert_eq!(remaining_accounts.len(), 3);
    }
}

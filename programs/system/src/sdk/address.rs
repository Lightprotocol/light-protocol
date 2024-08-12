use std::collections::HashMap;

use anchor_lang::{err, solana_program::pubkey::Pubkey, Result};
use light_utils::hash_to_bn254_field_size_be;

use crate::{errors::SystemProgramError, NewAddressParams, NewAddressParamsPacked};

use super::compressed_account::PackedAddressMerkleContext;
pub fn derive_address(merkle_tree_pubkey: &Pubkey, seed: &[u8; 32]) -> Result<[u8; 32]> {
    let hash = match hash_to_bn254_field_size_be(
        [merkle_tree_pubkey.to_bytes(), *seed].concat().as_slice(),
    ) {
        Some(hash) => Ok::<[u8; 32], SystemProgramError>(hash.0),
        None => return err!(SystemProgramError::DeriveAddressError),
    }?;

    Ok(hash)
}

pub fn add_and_get_remaining_account_indices(
    pubkeys: &[Pubkey],
    remaining_accounts: &mut HashMap<Pubkey, usize>,
) -> Vec<u8> {
    let mut vec = Vec::new();
    let mut next_index: usize = remaining_accounts.len();
    for pubkey in pubkeys.iter() {
        match remaining_accounts.get(pubkey) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(*pubkey, next_index);
                next_index += 1;
            }
        };
        vec.push(*remaining_accounts.get(pubkey).unwrap() as u8);
    }
    vec
}

// Helper function to pack new address params for instruction data in rust clients
pub fn pack_new_address_params(
    new_address_params: &[NewAddressParams],
    remaining_accounts: &mut HashMap<Pubkey, usize>,
) -> Vec<NewAddressParamsPacked> {
    let mut next_index: usize = remaining_accounts.len();
    new_address_params
        .iter()
        .map(|new_address_params| {
            let address_merkle_tree_pubkey_index = *remaining_accounts
                .entry(
                    new_address_params
                        .address_merkle_context
                        .address_merkle_tree_pubkey,
                )
                .or_insert_with(|| {
                    let index = next_index;
                    next_index += 1;
                    index
                }) as u8;
            let address_queue_pubkey_index = *remaining_accounts
                .entry(
                    new_address_params
                        .address_merkle_context
                        .address_queue_pubkey,
                )
                .or_insert_with(|| {
                    let index = next_index;
                    next_index += 1;
                    index
                }) as u8;
            NewAddressParamsPacked {
                seed: new_address_params.seed,
                address_merkle_context: PackedAddressMerkleContext {
                    address_merkle_tree_pubkey_index,
                    address_queue_pubkey_index,
                    root_index: new_address_params.address_merkle_context.root_index,
                },
            }
        })
        .collect::<Vec<NewAddressParamsPacked>>()
}

#[cfg(test)]
mod tests {
    use solana_sdk::{signature::Keypair, signer::Signer};

    use super::*;

    #[test]
    fn test_derive_address_with_valid_input() {
        let merkle_tree_pubkey = Keypair::new().pubkey();
        let seeds = [1u8; 32];
        let result = derive_address(&merkle_tree_pubkey, &seeds);
        let result_2 = derive_address(&merkle_tree_pubkey, &seeds);
        assert_eq!(result, result_2);
    }

    #[test]
    fn test_derive_address_no_collision_same_seeds_diff_pubkey() {
        let merkle_tree_pubkey = Keypair::new().pubkey();
        let merkle_tree_pubkey_2 = Keypair::new().pubkey();
        let seed = [2u8; 32];

        let result = derive_address(&merkle_tree_pubkey, &seed);
        let result_2 = derive_address(&merkle_tree_pubkey_2, &seed);
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
        let pubkey = Keypair::new().pubkey();
        let pubkeys = vec![pubkey];
        let mut remaining_accounts = HashMap::new();
        let result = add_and_get_remaining_account_indices(&pubkeys, &mut remaining_accounts);
        assert_eq!(result, vec![0]);
        assert_eq!(remaining_accounts.get(&pubkey), Some(&0));
    }

    #[test]
    fn test_add_and_get_remaining_account_indices_multiple() {
        let pubkey1 = Keypair::new().pubkey();
        let pubkey2 = Keypair::new().pubkey();
        let pubkeys = vec![pubkey1, pubkey2];
        let mut remaining_accounts = HashMap::new();
        let result = add_and_get_remaining_account_indices(&pubkeys, &mut remaining_accounts);
        assert_eq!(result, vec![0, 1]);
        assert_eq!(remaining_accounts.get(&pubkey1), Some(&0));
        assert_eq!(remaining_accounts.get(&pubkey2), Some(&1));
    }

    #[test]
    fn test_add_and_get_remaining_account_indices_duplicates() {
        let pubkey = Keypair::new().pubkey();
        let pubkeys = vec![pubkey, pubkey];
        let mut remaining_accounts = HashMap::new();
        let result = add_and_get_remaining_account_indices(&pubkeys, &mut remaining_accounts);
        assert_eq!(result, vec![0, 0]);
        assert_eq!(remaining_accounts.get(&pubkey), Some(&0));
        assert_eq!(remaining_accounts.len(), 1);
    }

    #[test]
    fn test_add_and_get_remaining_account_indices_multiple_duplicates() {
        let pubkey1 = Keypair::new().pubkey();
        let pubkey2 = Keypair::new().pubkey();
        let pubkey3 = Keypair::new().pubkey();
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

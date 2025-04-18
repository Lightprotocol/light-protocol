use std::collections::HashMap;

use light_hasher::hash_to_field_size::hashv_to_bn254_field_size_be_const_array;

use super::compressed_account::{
    pack_merkle_context, PackedReadOnlyCompressedAccount, ReadOnlyCompressedAccount,
};
use crate::{
    hash_to_bn254_field_size_be,
    instruction_data::data::{
        pack_pubkey_usize, NewAddressParams, NewAddressParamsAssigned,
        NewAddressParamsAssignedPacked, NewAddressParamsPacked, PackedReadOnlyAddress,
        ReadOnlyAddress,
    },
    CompressedAccountError, Pubkey,
};

pub fn derive_address_legacy(
    merkle_tree_pubkey: &Pubkey,
    seed: &[u8; 32],
) -> Result<[u8; 32], CompressedAccountError> {
    let hash = hash_to_bn254_field_size_be(
        [merkle_tree_pubkey.as_ref(), seed.as_ref()]
            .concat()
            .as_slice(),
    );
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
    hashv_to_bn254_field_size_be_const_array::<4>(&slices).unwrap()
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

pub fn pack_new_address_params_assigned(
    new_address_params: &[NewAddressParamsAssigned],
    remaining_accounts: &mut HashMap<Pubkey, usize>,
) -> Vec<NewAddressParamsAssignedPacked> {
    let mut vec = Vec::new();
    for new_address_param in new_address_params.iter() {
        let address_merkle_tree_account_index = pack_pubkey_usize(
            &new_address_param.address_merkle_tree_pubkey,
            remaining_accounts,
        );
        let address_queue_account_index =
            pack_pubkey_usize(&new_address_param.address_queue_pubkey, remaining_accounts);
        println!("new_address_param {:?}", new_address_param);
        vec.push(NewAddressParamsAssignedPacked {
            seed: new_address_param.seed,
            address_queue_account_index,
            address_merkle_tree_root_index: new_address_param.address_merkle_tree_root_index,
            address_merkle_tree_account_index,
            assigned_to_account: new_address_param.assigned_account_index.is_some(),
            assigned_account_index: new_address_param.assigned_account_index.unwrap_or_default(),
        });
    }

    vec
}

// Helper function to pack new address params for instruction data in rust clients
pub fn pack_new_address_params(
    new_address_params: &[NewAddressParams],
    remaining_accounts: &mut HashMap<Pubkey, usize>,
) -> Vec<NewAddressParamsPacked> {
    let mut new_address_params_packed = new_address_params
        .iter()
        .map(|x| NewAddressParamsPacked {
            seed: x.seed,
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

pub fn pack_read_only_address_params(
    new_address_params: &[ReadOnlyAddress],
    remaining_accounts: &mut HashMap<Pubkey, usize>,
) -> Vec<PackedReadOnlyAddress> {
    new_address_params
        .iter()
        .map(|x| PackedReadOnlyAddress {
            address: x.address,
            address_merkle_tree_root_index: x.address_merkle_tree_root_index,
            address_merkle_tree_account_index: pack_account(
                &x.address_merkle_tree_pubkey,
                remaining_accounts,
            ),
        })
        .collect::<Vec<PackedReadOnlyAddress>>()
}

pub fn pack_account(pubkey: &Pubkey, remaining_accounts: &mut HashMap<Pubkey, usize>) -> u8 {
    match remaining_accounts.get(pubkey) {
        Some(index) => *index as u8,
        None => {
            let next_index = remaining_accounts.len();
            remaining_accounts.insert(*pubkey, next_index);
            next_index as u8
        }
    }
}

pub fn pack_read_only_accounts(
    accounts: &[ReadOnlyCompressedAccount],
    remaining_accounts: &mut HashMap<Pubkey, usize>,
) -> Vec<PackedReadOnlyCompressedAccount> {
    accounts
        .iter()
        .map(|x| PackedReadOnlyCompressedAccount {
            account_hash: x.account_hash,
            merkle_context: pack_merkle_context(&[x.merkle_context], remaining_accounts)[0],
            root_index: x.root_index,
        })
        .collect::<Vec<PackedReadOnlyCompressedAccount>>()
}

#[cfg(not(feature = "pinocchio"))]
#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_derive_address_with_valid_input() {
        let merkle_tree_pubkey = crate::Pubkey::new_unique();
        let seeds = [1u8; 32];
        let result = derive_address_legacy(&merkle_tree_pubkey, &seeds);
        let result_2 = derive_address_legacy(&merkle_tree_pubkey, &seeds);
        assert_eq!(result, result_2);
    }

    #[test]
    fn test_derive_address_no_collision_same_seeds_diff_pubkey() {
        let merkle_tree_pubkey = crate::Pubkey::new_unique();
        let merkle_tree_pubkey_2 = crate::Pubkey::new_unique();
        let seed = [2u8; 32];

        let result = derive_address_legacy(&merkle_tree_pubkey, &seed);
        let result_2 = derive_address_legacy(&merkle_tree_pubkey_2, &seed);
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

use std::collections::HashMap;

use light_compressed_account::{
    compressed_account::{
        CompressedAccount, CompressedAccountWithMerkleContext,
        PackedCompressedAccountWithMerkleContext, PackedReadOnlyCompressedAccount,
        ReadOnlyCompressedAccount,
    },
    instruction_data::data::OutputCompressedAccountWithPackedContext,
    CompressedAccountError, Pubkey,
};
use light_sdk::{
    address::{
        NewAddressParams, NewAddressParamsAssigned, NewAddressParamsAssignedPacked,
        PackedReadOnlyAddress, ReadOnlyAddress,
    },
    instruction::{MerkleContext, PackedMerkleContext},
};

pub fn pack_compressed_account(
    account: &CompressedAccountWithMerkleContext,
    root_index: Option<u16>,
    remaining_accounts: &mut HashMap<Pubkey, usize>,
) -> Result<PackedCompressedAccountWithMerkleContext, CompressedAccountError> {
    Ok(PackedCompressedAccountWithMerkleContext {
        compressed_account: account.compressed_account.clone(),
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: pack_account(
                &account.merkle_context.merkle_tree_pubkey,
                remaining_accounts,
            ),
            queue_pubkey_index: pack_account(
                &account.merkle_context.queue_pubkey,
                remaining_accounts,
            ),
            leaf_index: account.merkle_context.leaf_index,
            prove_by_index: root_index.is_none(),
        },
        root_index: root_index.unwrap_or_default(),
        read_only: false,
    })
}
pub fn pack_pubkey_usize(pubkey: &Pubkey, hash_set: &mut HashMap<Pubkey, usize>) -> u8 {
    match hash_set.get(pubkey) {
        Some(index) => (*index) as u8,
        None => {
            let index = hash_set.len();
            hash_set.insert(*pubkey, index);
            index as u8
        }
    }
}
pub fn pack_compressed_accounts(
    compressed_accounts: &[CompressedAccountWithMerkleContext],
    root_indices: &[Option<u16>],
    remaining_accounts: &mut HashMap<Pubkey, usize>,
) -> Vec<PackedCompressedAccountWithMerkleContext> {
    compressed_accounts
        .iter()
        .zip(root_indices.iter())
        .map(|(x, root_index)| {
            let mut merkle_context = x.merkle_context;
            let root_index = if let Some(root) = root_index {
                *root
            } else {
                merkle_context.prove_by_index = true;
                0
            };

            PackedCompressedAccountWithMerkleContext {
                compressed_account: x.compressed_account.clone(),
                merkle_context: pack_merkle_context(&[merkle_context], remaining_accounts)[0],
                root_index,
                read_only: false,
            }
        })
        .collect::<Vec<_>>()
}

pub fn pack_output_compressed_accounts(
    compressed_accounts: &[CompressedAccount],
    merkle_trees: &[Pubkey],
    remaining_accounts: &mut HashMap<Pubkey, usize>,
) -> Vec<OutputCompressedAccountWithPackedContext> {
    compressed_accounts
        .iter()
        .zip(merkle_trees.iter())
        .map(|(x, tree)| OutputCompressedAccountWithPackedContext {
            compressed_account: x.clone(),
            merkle_tree_index: pack_account(tree, remaining_accounts),
        })
        .collect::<Vec<_>>()
}

pub fn pack_merkle_context(
    merkle_context: &[MerkleContext],
    remaining_accounts: &mut HashMap<Pubkey, usize>,
) -> Vec<PackedMerkleContext> {
    merkle_context
        .iter()
        .map(|merkle_context| PackedMerkleContext {
            leaf_index: merkle_context.leaf_index,
            merkle_tree_pubkey_index: pack_account(
                &merkle_context.merkle_tree_pubkey,
                remaining_accounts,
            ),
            queue_pubkey_index: pack_account(&merkle_context.queue_pubkey, remaining_accounts),
            prove_by_index: merkle_context.prove_by_index,
        })
        .collect::<Vec<_>>()
}
// Helper function to pack new address params for instruction data in rust clients
pub fn pack_new_address_params(
    new_address_params: &[NewAddressParams],
    remaining_accounts: &mut HashMap<light_compressed_account::Pubkey, usize>,
) -> Vec<light_compressed_account::instruction_data::data::NewAddressParamsPacked> {
    let mut new_address_params_packed = new_address_params
        .iter()
        .map(
            |x| light_compressed_account::instruction_data::data::NewAddressParamsPacked {
                seed: x.seed,
                address_merkle_tree_root_index: x.address_merkle_tree_root_index,
                address_merkle_tree_account_index: 0, // will be assigned later
                address_queue_account_index: 0,       // will be assigned later
            },
        )
        .collect::<Vec<light_compressed_account::instruction_data::data::NewAddressParamsPacked>>();
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

#[cfg(test)]
mod tests {

    use light_compressed_account::address::derive_address_legacy;

    use super::*;

    #[test]
    fn test_derive_address_with_valid_input() {
        let merkle_tree_pubkey = Pubkey::new_unique();
        let seeds = [1u8; 32];
        let result = derive_address_legacy(&merkle_tree_pubkey, &seeds);
        let result_2 = derive_address_legacy(&merkle_tree_pubkey, &seeds);
        assert_eq!(result, result_2);
    }

    #[test]
    fn test_derive_address_no_collision_same_seeds_diff_pubkey() {
        let merkle_tree_pubkey = Pubkey::new_unique();
        let merkle_tree_pubkey_2 = Pubkey::new_unique();
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

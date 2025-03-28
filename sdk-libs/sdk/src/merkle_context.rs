use std::collections::HashMap;

use light_compressed_account::compressed_account::{MerkleContext, PackedMerkleContext};
use solana_program::{instruction::AccountMeta, pubkey::Pubkey};

use crate::{
    system_accounts::{get_light_system_account_metas, SystemAccountMetaConfig},
    BorshDeserialize, BorshSerialize,
};

/// Collection of remaining accounts which are sent to the program.
#[derive(Default)]
pub struct CpiAccounts {
    next_index: u8,
    map: HashMap<Pubkey, (u8, AccountMeta)>,
}

impl CpiAccounts {
    pub fn new_with_system_accounts(config: SystemAccountMetaConfig) -> Self {
        let mut remaining_accounts = CpiAccounts::default();
        remaining_accounts.add_system_accounts(config);
        remaining_accounts
    }

    pub fn add_system_accounts(&mut self, config: SystemAccountMetaConfig) {
        for account in get_light_system_account_metas(config) {
            self.insert_or_get_config(account.pubkey, account.is_signer, account.is_writable);
        }
    }

    /// Returns the index of the provided `pubkey` in the collection.
    ///
    /// If the provided `pubkey` is not a part of the collection, it gets
    /// inserted with a `next_index`.
    ///
    /// If the privided `pubkey` already exists in the collection, its already
    /// existing index is returned.
    pub fn insert_or_get(&mut self, pubkey: Pubkey) -> u8 {
        self.insert_or_get_config(pubkey, false, true)
    }

    pub fn insert_or_get_signer(&mut self, pubkey: Pubkey) -> u8 {
        self.insert_or_get_config(pubkey, true, false)
    }

    pub fn insert_or_get_signer_mut(&mut self, pubkey: Pubkey) -> u8 {
        self.insert_or_get_config(pubkey, true, true)
    }

    pub fn insert_or_get_config(
        &mut self,
        pubkey: Pubkey,
        is_signer: bool,
        is_writable: bool,
    ) -> u8 {
        self.map
            .entry(pubkey)
            .or_insert_with(|| {
                let index = self.next_index;
                self.next_index += 1;
                (
                    index,
                    AccountMeta {
                        pubkey,
                        is_signer,
                        is_writable,
                    },
                )
            })
            .0
    }

    /// Converts the collection of accounts to a vector of
    /// [`AccountMeta`](solana_sdk::instruction::AccountMeta), which can be used
    /// as remaining accounts in instructions or CPI calls.
    pub fn to_account_metas(&self) -> Vec<AccountMeta> {
        let mut remaining_accounts = self.map.iter().collect::<Vec<_>>();
        // hash maps are not sorted so we need to sort manually and collect into a vector again
        remaining_accounts.sort_by(|a, b| a.1 .0.cmp(&b.1 .0));
        let remaining_accounts = remaining_accounts
            .iter()
            .map(|(_, (_, k))| k.clone())
            .collect::<Vec<AccountMeta>>();
        remaining_accounts
    }
}

pub fn pack_merkle_contexts<'a, I>(
    merkle_contexts: I,
    remaining_accounts: &'a mut CpiAccounts,
) -> impl Iterator<Item = PackedMerkleContext> + 'a
where
    I: Iterator<Item = &'a MerkleContext> + 'a,
{
    merkle_contexts.map(|x| pack_merkle_context(x, remaining_accounts))
}

pub fn pack_merkle_context(
    merkle_context: &MerkleContext,
    remaining_accounts: &mut CpiAccounts,
) -> PackedMerkleContext {
    let MerkleContext {
        merkle_tree_pubkey,
        nullifier_queue_pubkey,
        leaf_index,
        prove_by_index,
        ..
    } = merkle_context;
    let merkle_tree_pubkey_index = remaining_accounts.insert_or_get(*merkle_tree_pubkey);
    let nullifier_queue_pubkey_index = remaining_accounts.insert_or_get(*nullifier_queue_pubkey);

    PackedMerkleContext {
        merkle_tree_pubkey_index,
        nullifier_queue_pubkey_index,
        leaf_index: *leaf_index,
        prove_by_index: *prove_by_index,
    }
}

#[derive(Debug, Clone, Copy, BorshDeserialize, BorshSerialize, PartialEq, Default)]
pub struct AddressMerkleContext {
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_queue_pubkey: Pubkey,
}

#[derive(Debug, Clone, Copy, BorshDeserialize, BorshSerialize, PartialEq, Default)]
pub struct PackedAddressMerkleContext {
    pub address_merkle_tree_pubkey_index: u8,
    pub address_queue_pubkey_index: u8,
    pub root_index: u16,
}

/// Returns an iterator of [`PackedAddressMerkleContext`] and fills up
/// `remaining_accounts` based on the given `merkle_contexts`.
pub fn pack_address_merkle_contexts<'a, I>(
    address_merkle_contexts: I,
    root_index: &'a [u16],
    remaining_accounts: &'a mut CpiAccounts,
) -> impl Iterator<Item = PackedAddressMerkleContext> + 'a
where
    I: Iterator<Item = &'a AddressMerkleContext> + 'a,
{
    address_merkle_contexts
        .zip(root_index)
        .map(|(x, root_index)| pack_address_merkle_context(x, remaining_accounts, *root_index))
}

/// Returns a [`PackedAddressMerkleContext`] and fills up `remaining_accounts`
/// based on the given `merkle_context`.
pub fn pack_address_merkle_context(
    address_merkle_context: &AddressMerkleContext,
    remaining_accounts: &mut CpiAccounts,
    root_index: u16,
) -> PackedAddressMerkleContext {
    let AddressMerkleContext {
        address_merkle_tree_pubkey,
        address_queue_pubkey,
    } = address_merkle_context;
    let address_merkle_tree_pubkey_index =
        remaining_accounts.insert_or_get(*address_merkle_tree_pubkey);
    let address_queue_pubkey_index = remaining_accounts.insert_or_get(*address_queue_pubkey);

    PackedAddressMerkleContext {
        address_merkle_tree_pubkey_index,
        address_queue_pubkey_index,
        root_index,
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;

//     #[test]
//     fn test_remaining_accounts() {
//         let mut remaining_accounts = CpiAccounts::default();

//         let pubkey_1 = Pubkey::new_unique();
//         let pubkey_2 = Pubkey::new_unique();
//         let pubkey_3 = Pubkey::new_unique();
//         let pubkey_4 = Pubkey::new_unique();

//         // Initial insertion.
//         assert_eq!(remaining_accounts.insert_or_get(pubkey_1), 0);
//         assert_eq!(remaining_accounts.insert_or_get(pubkey_2), 1);
//         assert_eq!(remaining_accounts.insert_or_get(pubkey_3), 2);

//         assert_eq!(
//             remaining_accounts.to_account_metas().as_slice(),
//             &[
//                 AccountMeta {
//                     pubkey: pubkey_1,
//                     is_signer: false,
//                     is_writable: true,
//                 },
//                 AccountMeta {
//                     pubkey: pubkey_2,
//                     is_signer: false,
//                     is_writable: true,
//                 },
//                 AccountMeta {
//                     pubkey: pubkey_3,
//                     is_signer: false,
//                     is_writable: true,
//                 }
//             ]
//         );

//         // Insertion of already existing pubkeys.
//         assert_eq!(remaining_accounts.insert_or_get(pubkey_1), 0);
//         assert_eq!(remaining_accounts.insert_or_get(pubkey_2), 1);
//         assert_eq!(remaining_accounts.insert_or_get(pubkey_3), 2);

//         assert_eq!(
//             remaining_accounts.to_account_metas().as_slice(),
//             &[
//                 AccountMeta {
//                     pubkey: pubkey_1,
//                     is_signer: false,
//                     is_writable: true,
//                 },
//                 AccountMeta {
//                     pubkey: pubkey_2,
//                     is_signer: false,
//                     is_writable: true,
//                 },
//                 AccountMeta {
//                     pubkey: pubkey_3,
//                     is_signer: false,
//                     is_writable: true,
//                 }
//             ]
//         );

//         // Again, initial insertion.
//         assert_eq!(remaining_accounts.insert_or_get(pubkey_4), 3);

//         assert_eq!(
//             remaining_accounts.to_account_metas().as_slice(),
//             &[
//                 AccountMeta {
//                     pubkey: pubkey_1,
//                     is_signer: false,
//                     is_writable: true,
//                 },
//                 AccountMeta {
//                     pubkey: pubkey_2,
//                     is_signer: false,
//                     is_writable: true,
//                 },
//                 AccountMeta {
//                     pubkey: pubkey_3,
//                     is_signer: false,
//                     is_writable: true,
//                 },
//                 AccountMeta {
//                     pubkey: pubkey_4,
//                     is_signer: false,
//                     is_writable: true,
//                 }
//             ]
//         );
//     }

//     #[test]
//     fn test_pack_merkle_context() {
//         let mut remaining_accounts = CpiAccounts::default();

//         let merkle_tree_pubkey = Pubkey::new_unique();
//         let nullifier_queue_pubkey = Pubkey::new_unique();
//         let merkle_context = MerkleContext {
//             merkle_tree_pubkey,
//             nullifier_queue_pubkey,
//             leaf_index: 69,
//             prove_by_index: false,
//             ..Default::default()
//         };

//         let packed_merkle_context = pack_merkle_context(&merkle_context, &mut remaining_accounts);
//         assert_eq!(
//             packed_merkle_context,
//             PackedMerkleContext {
//                 merkle_tree_pubkey_index: 0,
//                 nullifier_queue_pubkey_index: 1,
//                 leaf_index: 69,
//                 prove_by_index: false,
//             }
//         )
//     }

//     #[test]
//     fn test_pack_merkle_contexts() {
//         let mut remaining_accounts = CpiAccounts::default();

//         let merkle_contexts = &[
//             MerkleContext {
//                 merkle_tree_pubkey: Pubkey::new_unique(),
//                 nullifier_queue_pubkey: Pubkey::new_unique(),
//                 leaf_index: 10,
//                 prove_by_index: false,
//                 ..Default::default()
//             },
//             MerkleContext {
//                 merkle_tree_pubkey: Pubkey::new_unique(),
//                 nullifier_queue_pubkey: Pubkey::new_unique(),
//                 leaf_index: 11,
//                 prove_by_index: true,
//                 ..Default::default()
//             },
//             MerkleContext {
//                 merkle_tree_pubkey: Pubkey::new_unique(),
//                 nullifier_queue_pubkey: Pubkey::new_unique(),
//                 leaf_index: 12,
//                 prove_by_index: false,
//                 ..Default::default()
//             },
//         ];

//         let packed_merkle_contexts =
//             pack_merkle_contexts(merkle_contexts.iter(), &mut remaining_accounts);
//         assert_eq!(
//             packed_merkle_contexts.collect::<Vec<_>>(),
//             &[
//                 PackedMerkleContext {
//                     merkle_tree_pubkey_index: 0,
//                     nullifier_queue_pubkey_index: 1,
//                     leaf_index: 10,
//                     prove_by_index: false
//                 },
//                 PackedMerkleContext {
//                     merkle_tree_pubkey_index: 2,
//                     nullifier_queue_pubkey_index: 3,
//                     leaf_index: 11,
//                     prove_by_index: true
//                 },
//                 PackedMerkleContext {
//                     merkle_tree_pubkey_index: 4,
//                     nullifier_queue_pubkey_index: 5,
//                     leaf_index: 12,
//                     prove_by_index: false,
//                 }
//             ]
//         );
//     }

//     #[test]
//     fn test_pack_address_merkle_context() {
//         let mut remaining_accounts = CpiAccounts::default();

//         let address_merkle_context = AddressMerkleContext {
//             address_merkle_tree_pubkey: Pubkey::new_unique(),
//             address_queue_pubkey: Pubkey::new_unique(),
//         };

//         let packed_address_merkle_context =
//             pack_address_merkle_context(&address_merkle_context, &mut remaining_accounts);
//         assert_eq!(
//             packed_address_merkle_context,
//             PackedAddressMerkleContext {
//                 address_merkle_tree_pubkey_index: 0,
//                 address_queue_pubkey_index: 1,
//             }
//         )
//     }

//     #[test]
//     fn test_pack_address_merkle_contexts() {
//         let mut remaining_accounts = CpiAccounts::default();

//         let address_merkle_contexts = &[
//             AddressMerkleContext {
//                 address_merkle_tree_pubkey: Pubkey::new_unique(),
//                 address_queue_pubkey: Pubkey::new_unique(),
//             },
//             AddressMerkleContext {
//                 address_merkle_tree_pubkey: Pubkey::new_unique(),
//                 address_queue_pubkey: Pubkey::new_unique(),
//             },
//             AddressMerkleContext {
//                 address_merkle_tree_pubkey: Pubkey::new_unique(),
//                 address_queue_pubkey: Pubkey::new_unique(),
//             },
//         ];

//         let packed_address_merkle_contexts =
//             pack_address_merkle_contexts(address_merkle_contexts.iter(), &mut remaining_accounts);
//         assert_eq!(
//             packed_address_merkle_contexts.collect::<Vec<_>>(),
//             &[
//                 PackedAddressMerkleContext {
//                     address_merkle_tree_pubkey_index: 0,
//                     address_queue_pubkey_index: 1,
//                 },
//                 PackedAddressMerkleContext {
//                     address_merkle_tree_pubkey_index: 2,
//                     address_queue_pubkey_index: 3,
//                 },
//                 PackedAddressMerkleContext {
//                     address_merkle_tree_pubkey_index: 4,
//                     address_queue_pubkey_index: 5,
//                 }
//             ]
//         );
//     }
// }

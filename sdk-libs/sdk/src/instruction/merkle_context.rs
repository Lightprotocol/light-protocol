pub use light_compressed_account::compressed_account::{MerkleContext, PackedMerkleContext};

use super::pack_accounts::PackedAccounts;
use crate::{AccountInfo, AnchorDeserialize, AnchorSerialize, Pubkey};

#[derive(Debug, Clone, Copy, AnchorDeserialize, AnchorSerialize, PartialEq, Default)]
pub struct AddressMerkleContext {
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_queue_pubkey: Pubkey,
}

#[derive(Debug, Clone, Copy, AnchorDeserialize, AnchorSerialize, PartialEq, Default)]
pub struct PackedAddressMerkleContext {
    pub address_merkle_tree_pubkey_index: u8,
    pub address_queue_pubkey_index: u8,
    pub root_index: u16,
}

pub fn pack_merkle_contexts<'a, I>(
    merkle_contexts: I,
    remaining_accounts: &'a mut PackedAccounts,
) -> impl Iterator<Item = PackedMerkleContext> + 'a
where
    I: Iterator<Item = &'a MerkleContext> + 'a,
{
    merkle_contexts.map(|x| pack_merkle_context(x, remaining_accounts))
}

pub fn pack_merkle_context(
    merkle_context: &MerkleContext,
    remaining_accounts: &mut PackedAccounts,
) -> PackedMerkleContext {
    let MerkleContext {
        merkle_tree_pubkey,
        queue_pubkey,
        leaf_index,
        prove_by_index,
        ..
    } = merkle_context;
    let merkle_tree_pubkey_index = remaining_accounts.insert_or_get(*merkle_tree_pubkey);
    let queue_pubkey_index = remaining_accounts.insert_or_get(*queue_pubkey);

    PackedMerkleContext {
        merkle_tree_pubkey_index,
        queue_pubkey_index,
        leaf_index: *leaf_index,
        prove_by_index: *prove_by_index,
    }
}

/// Returns an iterator of [`PackedAddressMerkleContext`] and fills up
/// `remaining_accounts` based on the given `merkle_contexts`.
pub fn pack_address_merkle_contexts<'a>(
    address_merkle_contexts: &'a [AddressMerkleContext],
    root_index: &'a [u16],
    remaining_accounts: &'a mut PackedAccounts,
) -> impl Iterator<Item = PackedAddressMerkleContext> + 'a {
    address_merkle_contexts
        .iter()
        .zip(root_index)
        .map(move |(x, root_index)| pack_address_merkle_context(x, remaining_accounts, *root_index))
}

/// Returns a [`PackedAddressMerkleContext`] and fills up `remaining_accounts`
/// based on the given `merkle_context`.
pub fn pack_address_merkle_context(
    address_merkle_context: &AddressMerkleContext,
    remaining_accounts: &mut PackedAccounts,
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

pub fn unpack_address_merkle_contexts(
    address_merkle_contexts: &[PackedAddressMerkleContext],
    remaining_accounts: &[AccountInfo],
) -> Vec<AddressMerkleContext> {
    let mut result = Vec::with_capacity(address_merkle_contexts.len());
    for x in address_merkle_contexts {
        let address_merkle_tree_pubkey =
            *remaining_accounts[x.address_merkle_tree_pubkey_index as usize].key;
        let address_queue_pubkey = *remaining_accounts[x.address_queue_pubkey_index as usize].key;
        result.push(AddressMerkleContext {
            address_merkle_tree_pubkey,
            address_queue_pubkey,
        });
    }
    result
}

pub fn unpack_address_merkle_context(
    address_merkle_context: PackedAddressMerkleContext,
    remaining_accounts: &[AccountInfo],
) -> AddressMerkleContext {
    unpack_address_merkle_contexts(&[address_merkle_context], remaining_accounts)[0]
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_pack_merkle_context() {
        let mut remaining_accounts = PackedAccounts::default();

        let merkle_tree_pubkey = Pubkey::new_unique();
        let queue_pubkey = Pubkey::new_unique();
        let merkle_context = MerkleContext {
            merkle_tree_pubkey,
            queue_pubkey,
            leaf_index: 69,
            prove_by_index: false,
            ..Default::default()
        };

        let packed_merkle_context = pack_merkle_context(&merkle_context, &mut remaining_accounts);
        assert_eq!(
            packed_merkle_context,
            PackedMerkleContext {
                merkle_tree_pubkey_index: 0,
                queue_pubkey_index: 1,
                leaf_index: 69,
                prove_by_index: false,
            }
        )
    }

    #[test]
    fn test_pack_merkle_contexts() {
        let mut remaining_accounts = PackedAccounts::default();

        let merkle_contexts = &[
            MerkleContext {
                merkle_tree_pubkey: Pubkey::new_unique(),
                queue_pubkey: Pubkey::new_unique(),
                leaf_index: 10,
                prove_by_index: false,
                ..Default::default()
            },
            MerkleContext {
                merkle_tree_pubkey: Pubkey::new_unique(),
                queue_pubkey: Pubkey::new_unique(),
                leaf_index: 11,
                prove_by_index: true,
                ..Default::default()
            },
            MerkleContext {
                merkle_tree_pubkey: Pubkey::new_unique(),
                queue_pubkey: Pubkey::new_unique(),
                leaf_index: 12,
                prove_by_index: false,
                ..Default::default()
            },
        ];

        let packed_merkle_contexts =
            pack_merkle_contexts(merkle_contexts.iter(), &mut remaining_accounts);
        assert_eq!(
            packed_merkle_contexts.collect::<Vec<_>>(),
            &[
                PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    queue_pubkey_index: 1,
                    leaf_index: 10,
                    prove_by_index: false
                },
                PackedMerkleContext {
                    merkle_tree_pubkey_index: 2,
                    queue_pubkey_index: 3,
                    leaf_index: 11,
                    prove_by_index: true
                },
                PackedMerkleContext {
                    merkle_tree_pubkey_index: 4,
                    queue_pubkey_index: 5,
                    leaf_index: 12,
                    prove_by_index: false,
                }
            ]
        );
    }

    #[test]
    fn test_pack_address_merkle_context() {
        let mut remaining_accounts = PackedAccounts::default();

        let address_merkle_context = AddressMerkleContext {
            address_merkle_tree_pubkey: Pubkey::new_unique(),
            address_queue_pubkey: Pubkey::new_unique(),
        };

        let packed_address_merkle_context =
            pack_address_merkle_context(&address_merkle_context, &mut remaining_accounts, 2);
        assert_eq!(
            packed_address_merkle_context,
            PackedAddressMerkleContext {
                address_merkle_tree_pubkey_index: 0,
                address_queue_pubkey_index: 1,
                root_index: 2,
            }
        )
    }

    #[test]
    fn test_pack_address_merkle_contexts() {
        let mut remaining_accounts = PackedAccounts::default();

        let address_merkle_contexts = [
            AddressMerkleContext {
                address_merkle_tree_pubkey: Pubkey::new_unique(),
                address_queue_pubkey: Pubkey::new_unique(),
            },
            AddressMerkleContext {
                address_merkle_tree_pubkey: Pubkey::new_unique(),
                address_queue_pubkey: Pubkey::new_unique(),
            },
            AddressMerkleContext {
                address_merkle_tree_pubkey: Pubkey::new_unique(),
                address_queue_pubkey: Pubkey::new_unique(),
            },
        ];

        let packed_address_merkle_contexts = pack_address_merkle_contexts(
            &address_merkle_contexts,
            &[6, 7, 8],
            &mut remaining_accounts,
        );
        assert_eq!(
            packed_address_merkle_contexts.collect::<Vec<_>>(),
            &[
                PackedAddressMerkleContext {
                    address_merkle_tree_pubkey_index: 0,
                    address_queue_pubkey_index: 1,
                    root_index: 6,
                },
                PackedAddressMerkleContext {
                    address_merkle_tree_pubkey_index: 2,
                    address_queue_pubkey_index: 3,
                    root_index: 7,
                },
                PackedAddressMerkleContext {
                    address_merkle_tree_pubkey_index: 4,
                    address_queue_pubkey_index: 5,
                    root_index: 8,
                }
            ]
        );
    }
}

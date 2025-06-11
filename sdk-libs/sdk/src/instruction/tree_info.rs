pub use light_compressed_account::compressed_account::{MerkleContext, PackedMerkleContext};
use light_sdk_types::instruction::PackedAddressTreeInfo;

use super::PackedAccounts;
use crate::{AccountInfo, AnchorDeserialize, AnchorSerialize, Pubkey};

#[derive(Debug, Clone, Copy, AnchorDeserialize, AnchorSerialize, PartialEq, Default)]
pub struct AddressTreeInfo {
    pub tree: Pubkey,
    pub queue: Pubkey,
}

#[deprecated(since = "0.13.0", note = "please use PackedStateTreeInfo")]
pub fn pack_merkle_contexts<'a, I>(
    merkle_contexts: I,
    remaining_accounts: &'a mut PackedAccounts,
) -> impl Iterator<Item = PackedMerkleContext> + 'a
where
    I: Iterator<Item = &'a MerkleContext> + 'a,
{
    #[allow(deprecated)]
    merkle_contexts.map(|x| pack_merkle_context(x, remaining_accounts))
}

#[deprecated(since = "0.13.0", note = "please use PackedStateTreeInfo")]
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
    let merkle_tree_pubkey_index =
        remaining_accounts.insert_or_get(merkle_tree_pubkey.to_bytes().into());
    let queue_pubkey_index = remaining_accounts.insert_or_get(queue_pubkey.to_bytes().into());

    PackedMerkleContext {
        merkle_tree_pubkey_index,
        queue_pubkey_index,
        leaf_index: *leaf_index,
        prove_by_index: *prove_by_index,
    }
}

/// Returns an iterator of [`PackedAddressTreeInfo`] and fills up
/// `remaining_accounts` based on the given `merkle_contexts`.
pub fn pack_address_tree_infos<'a>(
    address_tree_infos: &'a [AddressTreeInfo],
    root_index: &'a [u16],
    remaining_accounts: &'a mut PackedAccounts,
) -> impl Iterator<Item = PackedAddressTreeInfo> + 'a {
    address_tree_infos
        .iter()
        .zip(root_index)
        .map(move |(x, root_index)| pack_address_tree_info(x, remaining_accounts, *root_index))
}

/// Returns a [`PackedAddressTreeInfo`] and fills up `remaining_accounts`
/// based on the given `merkle_context`.
/// Packs Merkle tree account first.
/// Packs queue account second.
pub fn pack_address_tree_info(
    address_tree_info: &AddressTreeInfo,
    remaining_accounts: &mut PackedAccounts,
    root_index: u16,
) -> PackedAddressTreeInfo {
    let AddressTreeInfo { tree, queue } = address_tree_info;
    let address_merkle_tree_pubkey_index = remaining_accounts.insert_or_get(*tree);
    let address_queue_pubkey_index = remaining_accounts.insert_or_get(*queue);

    PackedAddressTreeInfo {
        address_merkle_tree_pubkey_index,
        address_queue_pubkey_index,
        root_index,
    }
}

pub fn unpack_address_tree_infos(
    address_tree_infos: &[PackedAddressTreeInfo],
    remaining_accounts: &[AccountInfo],
) -> Vec<AddressTreeInfo> {
    let mut result = Vec::with_capacity(address_tree_infos.len());
    for x in address_tree_infos {
        let address_merkle_tree_pubkey =
            *remaining_accounts[x.address_merkle_tree_pubkey_index as usize].key;
        let address_queue_pubkey = *remaining_accounts[x.address_queue_pubkey_index as usize].key;
        result.push(AddressTreeInfo {
            tree: address_merkle_tree_pubkey,
            queue: address_queue_pubkey,
        });
    }
    result
}

pub fn unpack_address_tree_info(
    address_tree_info: PackedAddressTreeInfo,
    remaining_accounts: &[AccountInfo],
) -> AddressTreeInfo {
    unpack_address_tree_infos(&[address_tree_info], remaining_accounts)[0]
}

#[cfg(test)]
mod test {

    use light_compressed_account::Pubkey;

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

        #[allow(deprecated)]
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
        use light_compressed_account::Pubkey;
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

        #[allow(deprecated)]
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
    fn test_pack_address_tree_info() {
        use solana_pubkey::Pubkey;
        let mut remaining_accounts = PackedAccounts::default();

        let address_tree_info = AddressTreeInfo {
            tree: Pubkey::new_unique(),
            queue: Pubkey::new_unique(),
        };

        let packed_address_tree_info =
            pack_address_tree_info(&address_tree_info, &mut remaining_accounts, 2);
        assert_eq!(
            packed_address_tree_info,
            PackedAddressTreeInfo {
                address_merkle_tree_pubkey_index: 0,
                address_queue_pubkey_index: 1,
                root_index: 2,
            }
        )
    }

    #[test]
    fn test_pack_address_tree_infos() {
        let mut remaining_accounts = PackedAccounts::default();
        use solana_pubkey::Pubkey;
        let address_tree_infos = [
            AddressTreeInfo {
                tree: Pubkey::new_unique(),
                queue: Pubkey::new_unique(),
            },
            AddressTreeInfo {
                tree: Pubkey::new_unique(),
                queue: Pubkey::new_unique(),
            },
            AddressTreeInfo {
                tree: Pubkey::new_unique(),
                queue: Pubkey::new_unique(),
            },
        ];

        let packed_address_tree_infos =
            pack_address_tree_infos(&address_tree_infos, &[6, 7, 8], &mut remaining_accounts);
        assert_eq!(
            packed_address_tree_infos.collect::<Vec<_>>(),
            &[
                PackedAddressTreeInfo {
                    address_merkle_tree_pubkey_index: 0,
                    address_queue_pubkey_index: 1,
                    root_index: 6,
                },
                PackedAddressTreeInfo {
                    address_merkle_tree_pubkey_index: 2,
                    address_queue_pubkey_index: 3,
                    root_index: 7,
                },
                PackedAddressTreeInfo {
                    address_merkle_tree_pubkey_index: 4,
                    address_queue_pubkey_index: 5,
                    root_index: 8,
                }
            ]
        );
    }
}

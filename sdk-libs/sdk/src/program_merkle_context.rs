use solana_program::account_info::AccountInfo;

use crate::merkle_context::{AddressMerkleContext, PackedAddressMerkleContext};

pub fn pack_address_merkle_contexts(
    address_merkle_contexts: &[AddressMerkleContext],
    root_index: u16,
    remaining_accounts: &[AccountInfo],
) -> Vec<PackedAddressMerkleContext> {
    address_merkle_contexts
        .iter()
        .map(|x| {
            let address_merkle_tree_pubkey_index = remaining_accounts
                .iter()
                .position(|account| *account.key == x.address_merkle_tree_pubkey)
                .unwrap() as u8;
            let address_queue_pubkey_index = remaining_accounts
                .iter()
                .position(|account| *account.key == x.address_queue_pubkey)
                .unwrap() as u8;
            PackedAddressMerkleContext {
                address_merkle_tree_pubkey_index,
                address_queue_pubkey_index,
                root_index,
            }
        })
        .collect::<Vec<_>>()
}

pub fn pack_address_merkle_context(
    address_merkle_context: AddressMerkleContext,
    root_index: u16,
    remaining_accounts: &[AccountInfo],
) -> PackedAddressMerkleContext {
    pack_address_merkle_contexts(&[address_merkle_context], root_index, remaining_accounts)[0]
}

pub fn unpack_address_merkle_contexts(
    address_merkle_contexts: &[PackedAddressMerkleContext],
    remaining_accounts: &[AccountInfo],
) -> Vec<AddressMerkleContext> {
    address_merkle_contexts
        .iter()
        .map(|x| {
            let address_merkle_tree_pubkey =
                *remaining_accounts[x.address_merkle_tree_pubkey_index as usize].key;
            let address_queue_pubkey =
                *remaining_accounts[x.address_queue_pubkey_index as usize].key;
            AddressMerkleContext {
                address_merkle_tree_pubkey,
                address_queue_pubkey,
            }
        })
        .collect::<Vec<_>>()
}

pub fn unpack_address_merkle_context(
    address_merkle_context: PackedAddressMerkleContext,
    remaining_accounts: &[AccountInfo],
) -> AddressMerkleContext {
    unpack_address_merkle_contexts(&[address_merkle_context], remaining_accounts)[0]
}

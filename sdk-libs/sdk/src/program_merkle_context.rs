use crate::merkle_context::{AddressMerkleContext, PackedAddressMerkleContext};
use anchor_lang::{prelude::AccountInfo, Key};
use solana_program::pubkey::Pubkey;
use std::cell::RefCell;

pub fn pack_address_merkle_contexts(
    address_merkle_contexts: &[AddressMerkleContext],
    remaining_accounts: &[AccountInfo],
) -> Vec<PackedAddressMerkleContext> {
    address_merkle_contexts
        .iter()
        .map(|x| {
            let address_merkle_tree_pubkey_index = remaining_accounts
                .iter()
                .position(|account| account.key() == x.address_merkle_tree_pubkey)
                .unwrap() as u8;
            let address_queue_pubkey_index = remaining_accounts
                .iter()
                .position(|account| account.key() == x.address_queue_pubkey)
                .unwrap() as u8;
            PackedAddressMerkleContext {
                address_merkle_tree_pubkey_index,
                address_queue_pubkey_index,
            }
        })
        .collect::<Vec<_>>()
}

pub fn pack_address_merkle_context(
    address_merkle_context: AddressMerkleContext,
    remaining_accounts: &[AccountInfo],
) -> PackedAddressMerkleContext {
    pack_address_merkle_contexts(&[address_merkle_context], remaining_accounts)[0]
}

pub fn unpack_address_merkle_contexts(
    address_merkle_contexts: &[PackedAddressMerkleContext],
    remaining_accounts: &[AccountInfo],
) -> Vec<AddressMerkleContext> {
    address_merkle_contexts
        .iter()
        .map(|x| {
            let address_merkle_tree_pubkey =
                remaining_accounts[x.address_merkle_tree_pubkey_index as usize].key();
            let address_queue_pubkey =
                remaining_accounts[x.address_queue_pubkey_index as usize].key();
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

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::prelude::AccountInfo;
    use solana_program::pubkey::Pubkey;
    use std::cell::RefCell;

    fn create_mock_account_info(pubkey: Pubkey) -> AccountInfo<'static> {
        let lamports = Box::new(RefCell::new(0));
        let data = Box::new(RefCell::new(vec![]));
        AccountInfo::new(
            &pubkey,
            false, // is_signer
            false, // is_writable
            Box::leak(lamports).borrow_mut(),
            Box::leak(data).borrow_mut(),
            &Pubkey::default(), // owner
            false,              // executable
            0,                  // rent_epoch
        )
    }

    #[test]
    fn test_pack_unpack_address_merkle_contexts() {
        let pubkey1 = Pubkey::new_unique();
        let pubkey2 = Pubkey::new_unique();
        let account_info1 = create_mock_account_info(pubkey1);
        let account_info2 = create_mock_account_info(pubkey2);

        let address_merkle_context = AddressMerkleContext {
            address_merkle_tree_pubkey: pubkey1,
            address_queue_pubkey: pubkey2,
        };

        let remaining_accounts = vec![account_info1.clone(), account_info2.clone()];

        let packed = pack_address_merkle_contexts(&[address_merkle_context], &remaining_accounts);
        let unpacked = unpack_address_merkle_contexts(&packed, &remaining_accounts);

        assert_eq!(unpacked[0].address_merkle_tree_pubkey, pubkey1);
        assert_eq!(unpacked[0].address_queue_pubkey, pubkey2);
    }

    #[test]
    fn test_pack_unpack_single_address_merkle_context() {
        let pubkey1 = Pubkey::new_unique();
        let pubkey2 = Pubkey::new_unique();
        let account_info1 = create_mock_account_info(pubkey1);
        let account_info2 = create_mock_account_info(pubkey2);

        let address_merkle_context = AddressMerkleContext {
            address_merkle_tree_pubkey: pubkey1,
            address_queue_pubkey: pubkey2,
        };

        let remaining_accounts = vec![account_info1.clone(), account_info2.clone()];

        let packed = pack_address_merkle_context(address_merkle_context, &remaining_accounts);
        let unpacked = unpack_address_merkle_context(packed, &remaining_accounts);

        assert_eq!(unpacked.address_merkle_tree_pubkey, pubkey1);
        assert_eq!(unpacked.address_queue_pubkey, pubkey2);
    }
}

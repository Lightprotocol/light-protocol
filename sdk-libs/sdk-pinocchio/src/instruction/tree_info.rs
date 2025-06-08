pub use crate::compressed_account::PackedMerkleContext;

#[derive(Debug, Clone, Copy, BorshDeserialize, BorshSerialize, PartialEq, Default)]
pub struct MerkleContext {
    pub merkle_tree_pubkey: [u8; 32],
    pub queue_pubkey: [u8; 32],
    pub leaf_index: u32,
    pub tree_type: u8, // Simplified TreeType as u8
}
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};
use crate::{BorshDeserialize, BorshSerialize};

#[derive(Debug, Clone, Copy, BorshDeserialize, BorshSerialize, PartialEq, Default)]
pub struct PackedStateTreeInfo {
    pub root_index: u16,
    pub prove_by_index: bool,
    pub merkle_tree_pubkey_index: u8,
    pub queue_pubkey_index: u8,
    pub leaf_index: u32,
}

#[derive(Debug, Clone, Copy, BorshDeserialize, BorshSerialize, PartialEq, Default)]
pub struct AddressTreeInfo {
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_queue_pubkey: Pubkey,
}

#[derive(Debug, Clone, Copy, BorshDeserialize, BorshSerialize, PartialEq, Default)]
pub struct PackedAddressTreeInfo {
    pub address_merkle_tree_pubkey_index: u8,
    pub address_queue_pubkey_index: u8,
    pub root_index: u16,
}

impl PackedAddressTreeInfo {
    pub fn into_new_address_params_packed(self, seed: [u8; 32]) -> crate::NewAddressParamsPacked {
        crate::NewAddressParamsPacked {
            address_merkle_tree_account_index: self.address_merkle_tree_pubkey_index,
            address_queue_account_index: self.address_queue_pubkey_index,
            address_merkle_tree_root_index: self.root_index,
            seed,
        }
    }
}

pub fn unpack_address_tree_infos(
    address_tree_infos: &[PackedAddressTreeInfo],
    remaining_accounts: &[AccountInfo],
) -> Vec<AddressTreeInfo> {
    let mut result = Vec::with_capacity(address_tree_infos.len());
    for x in address_tree_infos {
        let address_merkle_tree_pubkey =
            *remaining_accounts[x.address_merkle_tree_pubkey_index as usize].key();
        let address_queue_pubkey = *remaining_accounts[x.address_queue_pubkey_index as usize].key();
        result.push(AddressTreeInfo {
            address_merkle_tree_pubkey,
            address_queue_pubkey,
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
use std::collections::HashMap;

use light_zero_copy::ZeroCopyMut;

use crate::{
    compressed_account::{CompressedAccount, PackedCompressedAccountWithMerkleContext},
    instruction_data::compressed_proof::CompressedProof,
    AnchorDeserialize, AnchorSerialize, Pubkey,
};

#[derive(Debug, PartialEq, Default, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct InstructionDataInvoke {
    pub proof: Option<CompressedProof>,
    pub input_compressed_accounts_with_merkle_context:
        Vec<PackedCompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    pub relay_fee: Option<u64>,
    pub new_address_params: Vec<NewAddressParamsPacked>,
    pub compress_or_decompress_lamports: Option<u64>,
    pub is_compress: bool,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct OutputCompressedAccountWithContext {
    pub compressed_account: CompressedAccount,
    pub merkle_tree: Pubkey,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorDeserialize, AnchorSerialize, ZeroCopyMut)]
pub struct OutputCompressedAccountWithPackedContext {
    pub compressed_account: CompressedAccount,
    pub merkle_tree_index: u8,
}

#[derive(
    Debug, PartialEq, Default, Clone, Copy, AnchorDeserialize, AnchorSerialize, ZeroCopyMut,
)]
pub struct NewAddressParamsPacked {
    pub seed: [u8; 32],
    pub address_queue_account_index: u8,
    pub address_merkle_tree_account_index: u8,
    pub address_merkle_tree_root_index: u16,
}

#[derive(
    Debug, PartialEq, Default, Clone, Copy, AnchorDeserialize, AnchorSerialize, ZeroCopyMut,
)]
pub struct NewAddressParamsAssignedPacked {
    pub seed: [u8; 32],
    pub address_queue_account_index: u8,
    pub address_merkle_tree_account_index: u8,
    pub address_merkle_tree_root_index: u16,
    pub assigned_to_account: bool,
    pub assigned_account_index: u8,
}

impl NewAddressParamsAssignedPacked {
    pub fn new(address_params: NewAddressParamsPacked, index: Option<u8>) -> Self {
        Self {
            seed: address_params.seed,
            address_queue_account_index: address_params.address_queue_account_index,
            address_merkle_tree_account_index: address_params.address_merkle_tree_account_index,
            address_merkle_tree_root_index: address_params.address_merkle_tree_root_index,
            assigned_to_account: index.is_some(),
            assigned_account_index: index.unwrap_or_default(),
        }
    }

    pub fn assigned_account_index(&self) -> Option<u8> {
        if self.assigned_to_account {
            Some(self.assigned_account_index)
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq, Default, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct NewAddressParams {
    pub seed: [u8; 32],
    pub address_queue_pubkey: Pubkey,
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_merkle_tree_root_index: u16,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct NewAddressParamsAssigned {
    pub seed: [u8; 32],
    pub address_queue_pubkey: Pubkey,
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_merkle_tree_root_index: u16,
    pub assigned_account_index: Option<u8>,
}

#[derive(
    Debug, PartialEq, Default, Clone, Copy, AnchorDeserialize, AnchorSerialize, ZeroCopyMut,
)]
pub struct PackedReadOnlyAddress {
    pub address: [u8; 32],
    pub address_merkle_tree_root_index: u16,
    pub address_merkle_tree_account_index: u8,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct ReadOnlyAddress {
    pub address: [u8; 32],
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_merkle_tree_root_index: u16,
}
// TODO: move
pub fn pack_pubkey(pubkey: &Pubkey, hash_set: &mut HashMap<Pubkey, u8>) -> u8 {
    match hash_set.get(pubkey) {
        Some(index) => *index,
        None => {
            let index = hash_set.len() as u8;
            hash_set.insert(*pubkey, index);
            index
        }
    }
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

//! Types used

use anchor_lang::{AnchorDeserialize, AnchorSerialize};
use solana_program::pubkey::Pubkey;

use crate::merkle_context::{QueueIndex, RemainingAccounts};

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct MerkleTreeMeta {
    pub merkle_tree_pubkey: Pubkey,
    pub nullifier_queue_pubkey: Option<Pubkey>,
    pub leaf_index: Option<u32>,
    pub queue_index: Option<QueueIndex>,
}

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct PackedMerkleTreeMeta {
    pub merkle_tree_pubkey_index: u8,
    pub nullifier_queue_pubkey_index: Option<u8>,
    pub leaf_index: Option<u32>,
    pub queue_index: Option<QueueIndex>,
}

pub fn pack_merkle_tree_meta(
    merkle_tree_meta: &MerkleTreeMeta,
    remaining_acounts: &mut RemainingAccounts,
) -> PackedMerkleTreeMeta {
    let merkle_tree_pubkey_index =
        remaining_acounts.insert_or_get(merkle_tree_meta.merkle_tree_pubkey);
    let nullifier_queue_pubkey_index = merkle_tree_meta
        .nullifier_queue_pubkey
        .map(|pubkey| remaining_acounts.insert_or_get(pubkey));
    PackedMerkleTreeMeta {
        merkle_tree_pubkey_index,
        nullifier_queue_pubkey_index,
        leaf_index: merkle_tree_meta.leaf_index,
        queue_index: merkle_tree_meta.queue_index,
    }
}

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct AddressMerkleTreeMeta {
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_queue_pubkey: Pubkey,
}

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct PackedAddressMerkleTreeMeta {
    pub address_merkle_tree_pubkey_index: u8,
    pub address_queue_pubkey_index: u8,
}

pub fn pack_address_merkle_tree_meta(
    address_merkle_tree_meta: &AddressMerkleTreeMeta,
    remaining_acounts: &mut RemainingAccounts,
) -> PackedAddressMerkleTreeMeta {
    let address_merkle_tree_pubkey_index =
        remaining_acounts.insert_or_get(address_merkle_tree_meta.address_merkle_tree_pubkey);
    let address_queue_pubkey_index =
        remaining_acounts.insert_or_get(address_merkle_tree_meta.address_queue_pubkey);
    PackedAddressMerkleTreeMeta {
        address_merkle_tree_pubkey_index,
        address_queue_pubkey_index,
    }
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct LightAccountMeta {
    /// Lamports.
    pub lamports: Option<u64>,
    /// Address of the account (the address can change).
    pub address: Option<[u8; 32]>,
    /// Data of the account.
    pub data: Option<Vec<u8>>,
    /// Merkle tree.
    pub merkle_tree_meta: PackedMerkleTreeMeta,
    /// Merkle tree root index.
    pub merkle_tree_root_index: Option<u16>,
    /// Address Merkle tree. Set only when adding or updating the address.
    pub address_merkle_tree_meta: Option<PackedAddressMerkleTreeMeta>,
    /// Address Merkle tree root index. Set only when adding or updating the
    /// address.
    pub address_merkle_tree_root_index: Option<u16>,
}

impl LightAccountMeta {
    pub fn new_init(
        lamports: Option<u64>,
        address: Option<[u8; 32]>,
        data: Option<Vec<u8>>,
        merkle_tree_meta: &MerkleTreeMeta,
        address_merkle_tree_meta: Option<&AddressMerkleTreeMeta>,
        address_merkle_tree_root_index: Option<u16>,
        remaining_accounts: &mut RemainingAccounts,
    ) -> Self {
        let merkle_tree_meta = pack_merkle_tree_meta(merkle_tree_meta, remaining_accounts);
        let address_merkle_tree_meta = address_merkle_tree_meta
            .map(|ctx| pack_address_merkle_tree_meta(ctx, remaining_accounts));
        Self {
            lamports,
            address,
            data,
            merkle_tree_meta,
            merkle_tree_root_index: None,
            address_merkle_tree_meta,
            address_merkle_tree_root_index,
        }
    }

    pub fn new_mut(
        lamports: Option<u64>,
        address: Option<[u8; 32]>,
        data: Option<Vec<u8>>,
        merkle_tree_meta: &MerkleTreeMeta,
        merkle_tree_root_index: u16,
        address_merkle_tree_meta: Option<&AddressMerkleTreeMeta>,
        address_merkle_tree_root_index: Option<u16>,
        remaining_accounts: &mut RemainingAccounts,
    ) -> Self {
        let merkle_tree_meta = pack_merkle_tree_meta(merkle_tree_meta, remaining_accounts);
        let address_merkle_tree_meta = address_merkle_tree_meta
            .map(|ctx| pack_address_merkle_tree_meta(ctx, remaining_accounts));
        Self {
            lamports,
            address,
            data,
            merkle_tree_meta,
            merkle_tree_root_index: Some(merkle_tree_root_index),
            address_merkle_tree_meta,
            address_merkle_tree_root_index,
        }
    }

    pub fn new_close(
        lamports: Option<u64>,
        address: Option<[u8; 32]>,
        data: Option<Vec<u8>>,
        merkle_tree_meta: &MerkleTreeMeta,
        merkle_tree_root_index: u16,
        remaining_accounts: &mut RemainingAccounts,
    ) -> Self {
        let merkle_tree_meta = pack_merkle_tree_meta(merkle_tree_meta, remaining_accounts);
        Self {
            lamports,
            address,
            data,
            merkle_tree_meta,
            merkle_tree_root_index: Some(merkle_tree_root_index),
            address_merkle_tree_meta: None,
            address_merkle_tree_root_index: None,
        }
    }
}

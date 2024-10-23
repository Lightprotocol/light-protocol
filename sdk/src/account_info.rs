use anchor_lang::{AnchorDeserialize, AnchorSerialize};
use solana_program::pubkey::Pubkey;

use crate::{
    compressed_account::{pack_compressed_account, CompressedAccountWithMerkleContext},
    merkle_context::{
        pack_address_merkle_context, AddressMerkleContext, PackedAddressMerkleContext,
        PackedMerkleContext, RemainingAccounts,
    },
};

/// Compressed account information.
#[derive(AnchorDeserialize, AnchorSerialize, Debug)]
pub enum LightAccountInfo {
    Init(LightInitAccountInfo),
    Mut(LightMutAccountInfo),
    Close(LightCloseAccountInfo),
}

impl LightAccountInfo {
    /// Creates a new `LightAccountInfo` representing a new compressed account.
    pub fn new(
        owner: Option<&Pubkey>,
        lamports: Option<u64>,
        merkle_tree_pubkey: &Pubkey,
        address_merkle_context: &AddressMerkleContext,
        address_merkle_tree_root_index: u16,
        remaining_accounts: &mut RemainingAccounts,
    ) -> Self {
        let merkle_tree_index = remaining_accounts.insert_or_get(*merkle_tree_pubkey);
        let address_merkle_context =
            pack_address_merkle_context(*address_merkle_context, remaining_accounts);
        Self::Init(LightInitAccountInfo {
            owner: owner.copied(),
            lamports,
            merkle_tree_index,
            // address_params,
            address_merkle_context,
            address_merkle_tree_root_index,
        })
    }

    pub fn mut_from(
        compressed_account: &CompressedAccountWithMerkleContext,
        root_index: u16,
        remaining_accounts: &mut RemainingAccounts,
    ) -> Self {
        let compressed_account =
            pack_compressed_account(compressed_account.clone(), root_index, remaining_accounts);
        Self::Mut(LightMutAccountInfo {
            owner: Some(compressed_account.compressed_account.owner),
            data_hash: compressed_account
                .compressed_account
                .data
                .as_ref()
                .map(|data| data.data_hash),
            data: compressed_account
                .compressed_account
                .data
                .map(|data| data.data),
            lamports: Some(compressed_account.compressed_account.lamports),
            merkle_context: compressed_account.merkle_context,
            root_index,
            address: compressed_account.compressed_account.address,
            new_address_merkle_context: None,
            address_merkle_tree_root_index: None,
            new_merkle_tree_index: None,
        })
    }

    pub fn mut_with_new_parameters(
        compressed_account: &CompressedAccountWithMerkleContext,
        root_index: u16,
        address: Option<[u8; 32]>,
        new_address_merkle_context: AddressMerkleContext,
        address_merkle_tree_root_index: u16,
        new_merkle_tree_index: u8,
        remaining_accounts: &mut RemainingAccounts,
    ) -> Self {
        let compressed_account =
            pack_compressed_account(compressed_account.clone(), root_index, remaining_accounts);
        let new_address_merkle_context =
            pack_address_merkle_context(new_address_merkle_context, remaining_accounts);
        Self::Mut(LightMutAccountInfo {
            owner: Some(compressed_account.compressed_account.owner),
            data_hash: compressed_account
                .compressed_account
                .data
                .as_ref()
                .map(|data| data.data_hash),
            data: compressed_account
                .compressed_account
                .data
                .map(|data| data.data),
            lamports: Some(compressed_account.compressed_account.lamports),
            merkle_context: compressed_account.merkle_context,
            root_index,
            address,
            new_address_merkle_context: Some(new_address_merkle_context),
            address_merkle_tree_root_index: Some(address_merkle_tree_root_index),
            new_merkle_tree_index: Some(new_merkle_tree_index),
        })
    }

    pub fn close_from(
        compressed_account: &CompressedAccountWithMerkleContext,
        root_index: u16,
        remaining_accounts: &mut RemainingAccounts,
    ) -> Self {
        let compressed_account =
            pack_compressed_account(compressed_account.clone(), root_index, remaining_accounts);
        Self::Close(LightCloseAccountInfo {
            owner: Some(compressed_account.compressed_account.owner),
            data_hash: compressed_account
                .compressed_account
                .data
                .as_ref()
                .map(|data| data.data_hash),
            data: compressed_account
                .compressed_account
                .data
                .map(|data| data.data),
            lamports: Some(compressed_account.compressed_account.lamports),
            merkle_context: compressed_account.merkle_context,
            root_index,
            address: compressed_account.compressed_account.address,
        })
    }
}

/// Information about compressed account which is being initialized.
#[derive(AnchorDeserialize, AnchorSerialize, Debug)]
pub struct LightInitAccountInfo {
    /// Owner of the account.
    ///
    /// Defaults to the program ID.
    pub owner: Option<Pubkey>,
    /// Lamports.
    pub lamports: Option<u64>,
    /// Merkle tree index.
    pub merkle_tree_index: u8,
    /// Address Merkle tree context.
    pub address_merkle_context: PackedAddressMerkleContext,
    /// Address Merkle tree root index.
    pub address_merkle_tree_root_index: u16,
}

/// Information about compressed account which is being mutated.
#[derive(AnchorDeserialize, AnchorSerialize, Debug)]
pub struct LightMutAccountInfo {
    /// Owner of the account.
    ///
    /// Defaults to the program ID.
    pub owner: Option<Pubkey>,
    /// Hash of the account data.
    pub data_hash: Option<[u8; 32]>,
    /// Account data.
    pub data: Option<Vec<u8>>,
    /// Lamports.
    pub lamports: Option<u64>,
    /// Merkle context.
    pub merkle_context: PackedMerkleContext,
    /// Root index.
    pub root_index: u16,
    /// Address.
    pub address: Option<[u8; 32]>,
    /// New address Merkle tree context. Set only if you want to change the
    /// address.
    pub new_address_merkle_context: Option<PackedAddressMerkleContext>,
    /// Address Merkle tree root index. Set only if you want to change the
    /// address.
    pub address_merkle_tree_root_index: Option<u16>,
    /// New Merkle tree index. Set only if you want to change the tree.
    pub new_merkle_tree_index: Option<u8>,
}

/// Information about compressed account which is being closed.
#[derive(AnchorDeserialize, AnchorSerialize, Debug)]
pub struct LightCloseAccountInfo {
    // Owner of the account.
    //
    // Defaults to the program ID.
    pub owner: Option<Pubkey>,
    // Hash of the account data.
    pub data_hash: Option<[u8; 32]>,
    /// Account data.
    pub data: Option<Vec<u8>>,
    // Lamports.
    pub lamports: Option<u64>,
    // Merkle context.
    pub merkle_context: PackedMerkleContext,
    /// Root index.
    pub root_index: u16,
    /// Address.
    pub address: Option<[u8; 32]>,
}

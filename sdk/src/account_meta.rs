//! Types used

use anchor_lang::{AnchorDeserialize, AnchorSerialize};
use solana_program::pubkey::Pubkey;

use crate::{
    compressed_account::CompressedAccountWithMerkleContext,
    error::LightSdkError,
    merkle_context::{
        pack_address_merkle_context, pack_merkle_context, AddressMerkleContext,
        PackedAddressMerkleContext, PackedMerkleContext, RemainingAccounts,
    },
};

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct LightAccountMeta {
    /// Lamports.
    pub lamports: Option<u64>,
    /// Address of the account (the address can change).
    pub address: Option<[u8; 32]>,
    /// Data of the account.
    pub data: Option<Vec<u8>>,
    /// Merkle tree.
    pub merkle_context: Option<PackedMerkleContext>,
    /// Merkle tree root index.
    pub merkle_tree_root_index: Option<u16>,
    /// Output Merkle tree.
    pub output_merkle_tree_index: Option<u8>,
    /// Address Merkle tree. Set only when adding or updating the address.
    pub address_merkle_context: Option<PackedAddressMerkleContext>,
    /// Address Merkle tree root index. Set only when adding or updating the
    /// address.
    pub address_merkle_tree_root_index: Option<u16>,
    /// Account is read only.
    /// (not used for now, just a placeholder)
    pub read_only: bool,
}

impl LightAccountMeta {
    #[allow(clippy::too_many_arguments)]
    pub fn new_init(
        output_merkle_tree: &Pubkey,
        address_merkle_context: Option<&AddressMerkleContext>,
        address_merkle_tree_root_index: Option<u16>,
        remaining_accounts: &mut RemainingAccounts,
    ) -> Result<Self, LightSdkError> {
        let output_merkle_tree_index = remaining_accounts.insert_or_get(*output_merkle_tree);
        let address_merkle_context =
            address_merkle_context.map(|ctx| pack_address_merkle_context(ctx, remaining_accounts));
        Ok(Self {
            lamports: None,
            address: None,
            data: None,
            merkle_context: None,
            merkle_tree_root_index: None,
            output_merkle_tree_index: Some(output_merkle_tree_index),
            address_merkle_context,
            address_merkle_tree_root_index,
            read_only: false,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_mut(
        compressed_account: &CompressedAccountWithMerkleContext,
        merkle_tree_root_index: u16,
        output_merkle_tree: &Pubkey,
        remaining_accounts: &mut RemainingAccounts,
    ) -> Self {
        let merkle_context =
            pack_merkle_context(&compressed_account.merkle_context, remaining_accounts);

        // If no output Merkle tree was specified, use the one used for the
        // input account.
        let output_merkle_tree_index = remaining_accounts.insert_or_get(*output_merkle_tree);

        Self {
            lamports: Some(compressed_account.compressed_account.lamports),
            address: compressed_account.compressed_account.address,
            data: compressed_account
                .compressed_account
                .data
                .as_ref()
                .map(|data| data.data.clone()),
            merkle_context: Some(merkle_context),
            merkle_tree_root_index: Some(merkle_tree_root_index),
            output_merkle_tree_index: Some(output_merkle_tree_index),
            address_merkle_context: None,
            address_merkle_tree_root_index: None,
            read_only: false,
        }
    }

    pub fn new_close(
        compressed_account: &CompressedAccountWithMerkleContext,
        merkle_tree_root_index: u16,
        remaining_accounts: &mut RemainingAccounts,
    ) -> Self {
        let merkle_context =
            pack_merkle_context(&compressed_account.merkle_context, remaining_accounts);
        Self {
            lamports: Some(compressed_account.compressed_account.lamports),
            address: compressed_account.compressed_account.address,
            data: compressed_account
                .compressed_account
                .data
                .as_ref()
                .map(|data| data.data.clone()),
            merkle_context: Some(merkle_context),
            merkle_tree_root_index: Some(merkle_tree_root_index),
            output_merkle_tree_index: None,
            address_merkle_context: None,
            address_merkle_tree_root_index: None,
            read_only: false,
        }
    }
}

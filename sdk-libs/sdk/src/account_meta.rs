//! Types used

use anchor_lang::{AnchorDeserialize, AnchorSerialize};
use solana_program::pubkey::Pubkey;

use crate::{
    compressed_account::CompressedAccountWithMerkleContext,
    error::LightSdkError,
    merkle_context::{
        pack_address_merkle_context, pack_merkle_context, AddressMerkleContext, MerkleContext,
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
    pub merkle_context: Option<MerkleContext>,
    /// Merkle tree root index.
    pub merkle_tree_root_index: Option<u16>,
    /// Output Merkle tree.
    pub output_merkle_tree: Option<Pubkey>,
    /// Address Merkle tree. Set only when adding or updating the address.
    pub address_merkle_context: Option<AddressMerkleContext>,
    /// Address Merkle tree root index. Set only when adding or updating the
    /// address.
    pub address_merkle_tree_root_index: Option<u16>,
    /// Account is read only.
    /// (not used for now, just a placeholder)
    pub read_only: bool,
}

impl LightAccountMeta {
    /// Create LightAccountMeta for initializing a compressed account.
    pub fn new_init(
        output_merkle_tree: &Pubkey,
        address_merkle_context: Option<&AddressMerkleContext>,
        address_merkle_tree_root_index: Option<u16>,
    ) -> Self {
        Self {
            output_merkle_tree: Some(*output_merkle_tree),
            address_merkle_context: address_merkle_context.cloned(),
            address_merkle_tree_root_index,
            ..Default::default()
        }
    }

    /// Create LightAccountMeta for mutating a compressed account.
    #[allow(clippy::too_many_arguments)]
    pub fn new_mut(
        compressed_account: &CompressedAccountWithMerkleContext,
        merkle_tree_root_index: u16,
        output_merkle_tree: &Pubkey,
    ) -> Self {
        Self {
            lamports: Some(compressed_account.compressed_account.lamports),
            address: compressed_account.compressed_account.address,
            data: compressed_account
                .compressed_account
                .data
                .as_ref()
                .map(|data| data.data.clone()),
            merkle_context: Some(compressed_account.merkle_context.clone()),
            merkle_tree_root_index: Some(merkle_tree_root_index),
            output_merkle_tree: Some(*output_merkle_tree),
            ..Default::default()
        }
    }

    /// Create LightAccountMeta for closing a compressed account.
    pub fn new_close(
        compressed_account: &CompressedAccountWithMerkleContext,
        merkle_tree_root_index: u16,
    ) -> Self {
        Self {
            lamports: Some(compressed_account.compressed_account.lamports),
            address: compressed_account.compressed_account.address,
            data: compressed_account
                .compressed_account
                .data
                .as_ref()
                .map(|data| data.data.clone()),
            merkle_context: Some(compressed_account.merkle_context.clone()),
            merkle_tree_root_index: Some(merkle_tree_root_index),
            ..Default::default()
        }
    }

    /// Pack LightAccountMeta into a PackedLightAccountMeta.
    ///
    /// Stores index pointers for merkle context and address merkle context
    /// pubkeys to the remaining accounts.
    pub fn pack(
        self,
        remaining_accounts: &mut RemainingAccounts,
    ) -> Result<PackedLightAccountMeta, LightSdkError> {
        let output_merkle_tree_index = match self.output_merkle_tree {
            Some(tree) => Some(remaining_accounts.insert_or_get(tree)),
            // new_close doesn't have output_merkle_tree
            None => None,
        };

        let packed_merkle_context = self
            .merkle_context
            .map(|ctx| pack_merkle_context(&ctx, remaining_accounts));

        let packed_address_merkle_context = self
            .address_merkle_context
            .map(|ctx| pack_address_merkle_context(&ctx, remaining_accounts));

        // Currently, read_only is not used for anything.
        if self.read_only {
            return Err(LightSdkError::ExpectedReadOnly);
        }

        Ok(PackedLightAccountMeta {
            lamports: self.lamports,
            address: self.address,
            data: self.data,
            merkle_context: packed_merkle_context,
            merkle_tree_root_index: self.merkle_tree_root_index,
            output_merkle_tree_index,
            address_merkle_context: packed_address_merkle_context,
            address_merkle_tree_root_index: self.address_merkle_tree_root_index,
            read_only: false,
        })
    }
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct PackedLightAccountMeta {
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

impl PackedLightAccountMeta {
    /// Create PackedLightAccountMeta for initializing a compressed account.
    ///
    /// Directly stores index pointers for merkle context and address merkle context
    /// pubkeys to the remaining accounts.
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

    /// Create PackedLightAccountMeta for mutating a compressed account.
    ///
    /// Directly stores index pointers for merkle context and address merkle context
    /// pubkeys to the remaining accounts.
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

    /// Create PackedLightAccountMeta for closing a compressed account.
    ///
    /// Directly stores index pointers for merkle context and address merkle context
    /// pubkeys to the remaining accounts.
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

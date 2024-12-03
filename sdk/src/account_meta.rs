//! Types used

#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
use solana_program::pubkey::Pubkey;

use crate::{
    address::NewAddressParams,
    compressed_account::CompressedAccountWithMerkleContext,
    error::LightSdkError,
    instruction_accounts::LightInstructionAccounts,
    merkle_context::{
        pack_address_merkle_context, pack_merkle_context, AddressMerkleContext, MerkleContext,
        PackedAddressMerkleContext, PackedMerkleContext,
    },
};
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct PackedInitMeta {
    pub output_merkle_tree_index: u8,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct PackedMutMeta {
    pub lamports: u64,
    pub address: [u8; 32],
    pub data: Vec<u8>,
    pub merkle_context: PackedMerkleContext,
    pub merkle_tree_root_index: u16,
    pub output_merkle_tree_index: u8,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct PackedCloseMeta {
    pub lamports: u64,
    pub address: [u8; 32],
    pub data: Vec<u8>,
    pub merkle_context: PackedMerkleContext,
    pub merkle_tree_root_index: u16,
}

// impl PackedLightAccountMeta {
//     #[allow(clippy::too_many_arguments)]
//     pub fn new_init(
//         output_merkle_tree: &Pubkey,
//         address_merkle_context: Option<&AddressMerkleContext>,
//         address_merkle_tree_root_index: Option<u16>,
//         remaining_accounts: &mut LightInstructionAccounts,
//     ) -> Result<Self, LightSdkError> {
//         let output_merkle_tree_index = remaining_accounts.insert_or_get(*output_merkle_tree);
//         let address_merkle_context =
//             address_merkle_context.map(|ctx| pack_address_merkle_context(ctx, remaining_accounts));
//         Ok(Self {
//             lamports: None,
//             address: None,
//             data: None,
//             merkle_context: None,
//             merkle_tree_root_index: None,
//             output_merkle_tree_index: Some(output_merkle_tree_index),
//             address_merkle_context,
//             address_merkle_tree_root_index,
//             read_only: false,
//         })
//     }

//     #[allow(clippy::too_many_arguments)]
//     pub fn new_mut(
//         compressed_account: &CompressedAccountWithMerkleContext,
//         merkle_tree_root_index: u16,
//         output_merkle_tree: &Pubkey,
//         accounts: &mut LightInstructionAccounts,
//     ) -> Self {
//         let merkle_context = pack_merkle_context(&compressed_account.merkle_context, accounts);

//         // If no output Merkle tree was specified, use the one used for the
//         // input account.
//         let output_merkle_tree_index = accounts.insert_or_get(*output_merkle_tree);

//         Self {
//             lamports: Some(compressed_account.compressed_account.lamports),
//             address: compressed_account.compressed_account.address,
//             data: compressed_account
//                 .compressed_account
//                 .data
//                 .as_ref()
//                 .map(|data| data.data.clone()),
//             merkle_context: Some(merkle_context),
//             merkle_tree_root_index: Some(merkle_tree_root_index),
//             output_merkle_tree_index: Some(output_merkle_tree_index),
//             address_merkle_context: None,
//             address_merkle_tree_root_index: None,
//             read_only: false,
//         }
//     }

//     pub fn new_close(
//         compressed_account: &CompressedAccountWithMerkleContext,
//         merkle_tree_root_index: u16,
//         accounts: &mut LightInstructionAccounts,
//     ) -> Self {
//         let merkle_context = pack_merkle_context(&compressed_account.merkle_context, accounts);
//         Self {
//             lamports: Some(compressed_account.compressed_account.lamports),
//             address: compressed_account.compressed_account.address,
//             data: compressed_account
//                 .compressed_account
//                 .data
//                 .as_ref()
//                 .map(|data| data.data.clone()),
//             merkle_context: Some(merkle_context),
//             merkle_tree_root_index: Some(merkle_tree_root_index),
//             output_merkle_tree_index: None,
//             address_merkle_context: None,
//             address_merkle_tree_root_index: None,
//             read_only: false,
//         }
//     }
// }

#[derive(Debug, Clone, PartialEq)]
pub struct LightAccountMeta {
    /// Lamports attached to the account.
    pub lamports: Option<u64>,
    /// Address of the account.
    pub address: Option<[u8; 32]>,
    /// Data of the account.
    pub data: Option<Vec<u8>>,
    /// State Merkle tree context of existing compressed account.
    pub merkle_context: Option<MerkleContext>,
    /// Index of recent state root for which the proof is valid. Expiry tied to
    /// proof.
    pub recent_state_root_index: Option<u16>,
    /// Output State Merkle tree that the account will be stored in.
    pub output_state_merkle_tree: Option<Pubkey>,
    /// Is read-only? Placeholder, not used for now.
    pub read_only: bool,
}

impl LightAccountMeta {
    /// Creates account meta and address params for initialization.
    /// Address params must be sent onchain for accounts requiring an address.
    pub fn new_init(
        output_state_merkle_tree: Pubkey,
        address_merkle_context: AddressMerkleContext,
        recent_address_merkle_tree_root_index: u16,
        seed: [u8; 32],
    ) -> (Self, NewAddressParams) {
        let account = Self {
            lamports: None,
            address: None,
            data: None,
            merkle_context: None,
            recent_state_root_index: None,
            output_state_merkle_tree: Some(output_state_merkle_tree),
            read_only: false,
        };

        let address_params = NewAddressParams {
            seed,
            address_queue_pubkey: address_merkle_context.address_queue_pubkey,
            address_merkle_tree_pubkey: address_merkle_context.address_merkle_tree_pubkey,
            address_merkle_tree_root_index: recent_address_merkle_tree_root_index,
        };

        (account, address_params)
    }
    /// Creates account meta for initialization without an address.
    ///
    /// Used for fungible account states where uniqueness guarantees are not
    /// needed. Without an address, the account cannot be used in uniqueness
    /// proofs for accounts or account data.
    pub fn new_init_without_address(output_state_merkle_tree: Pubkey) -> Self {
        Self {
            lamports: None,
            address: None,
            data: None,
            merkle_context: None,
            recent_state_root_index: None,
            output_state_merkle_tree: Some(output_state_merkle_tree),
            read_only: false,
        }
    }

    /// Creates account meta for mutating an existing account.
    pub fn new_mut(
        compressed_account: &CompressedAccountWithMerkleContext,
        recent_state_root_index: u16,
        output_state_merkle_tree: Pubkey,
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
            recent_state_root_index: Some(recent_state_root_index),
            output_state_merkle_tree: Some(output_state_merkle_tree),
            read_only: false,
        }
    }

    /// Creates account meta for closing an existing account.
    pub fn new_close(
        compressed_account: &CompressedAccountWithMerkleContext,
        recent_state_root_index: u16,
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
            recent_state_root_index: Some(recent_state_root_index),
            output_state_merkle_tree: None,
            read_only: false,
        }
    }
}

// pub fn pack_light_account_metas(
//     accounts: Option<Vec<LightAccountMeta>>,
//     remaining_accounts: &mut LightInstructionAccounts,
// ) -> Option<Vec<PackedLightAccountMeta>> {
//     accounts.map(|accounts| {
//         accounts
//             .iter()
//             .map(|account| {
//                 let output_merkle_tree_index = account
//                     .output_merkle_tree
//                     .map(|pubkey| remaining_accounts.insert_or_get(pubkey));

//                 let merkle_context = account
//                     .merkle_context
//                     .as_ref()
//                     .map(|ctx| pack_merkle_context(ctx, remaining_accounts));

//                 let address_merkle_context = account
//                     .address_merkle_context
//                     .as_ref()
//                     .map(|ctx| pack_address_merkle_context(ctx, remaining_accounts));

//                 PackedLightAccountMeta {
//                     lamports: account.lamports,
//                     address: account.address,
//                     data: account.data.clone(),
//                     merkle_context,
//                     merkle_tree_root_index: account.merkle_tree_root_index,
//                     output_merkle_tree_index,
//                     address_merkle_context,
//                     address_merkle_tree_root_index: account.address_merkle_tree_root_index,
//                     read_only: account.read_only,
//                 }
//             })
//             .collect()
//     })
// }

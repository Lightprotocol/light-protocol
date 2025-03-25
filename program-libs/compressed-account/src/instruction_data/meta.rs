//! Types used
use crate::compressed_account::{
    CompressedAccountWithMerkleContext, PackedMerkleContext, ZPackedMerkleContext,
};
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::{ZeroCopy, ZeroCopyEq};
use solana_program::pubkey::Pubkey;

// use crate::merkle_context::{
//     pack_address_merkle_context, pack_merkle_context, AddressMerkleContext,
//     PackedAddressMerkleContext, RemainingAccounts,
// };

#[derive(Debug, ZeroCopy)]
pub struct InputAccountMeta {
    pub merkle_context: PackedMerkleContext,
    pub lamports: u64,
    pub root_index: Option<u16>,
    pub output_merkle_tree_index: u8,
}

impl InputAccountMetaTrait for InputAccountMeta {
    fn get_merkle_context(&self) -> &PackedMerkleContext {
        &self.merkle_context
    }

    fn get_lamports(&self) -> Option<u64> {
        Some(self.lamports)
    }

    fn get_root_index(&self) -> Option<u16> {
        self.root_index
    }

    fn get_address(&self) -> Option<[u8; 32]> {
        None
    }
}

impl<'a> ZInputAccountMetaTrait<'a> for ZInputAccountMeta<'a> {
    fn get_merkle_context(&self) -> &ZPackedMerkleContext<'a> {
        &self.merkle_context
    }

    fn get_lamports(&self) -> Option<U64> {
        Some(*self.lamports)
    }

    fn get_root_index(&self) -> Option<U16> {
        self.root_index.map(|x| (*x).into())
    }

    fn get_address(&self) -> Option<[u8; 32]> {
        None
    }
}

#[derive(Debug, ZeroCopy)]
pub struct InputAccountMetaNoLamports {
    pub merkle_context: PackedMerkleContext,
    pub root_index: Option<u16>,
}

impl InputAccountMetaTrait for InputAccountMetaNoLamports {
    fn get_merkle_context(&self) -> &PackedMerkleContext {
        &self.merkle_context
    }

    fn get_lamports(&self) -> Option<u64> {
        None
    }

    fn get_root_index(&self) -> Option<u16> {
        self.root_index
    }

    fn get_address(&self) -> Option<[u8; 32]> {
        None
    }
}

impl<'a> ZInputAccountMetaTrait<'a> for ZInputAccountMetaNoLamports<'a> {
    fn get_merkle_context(&self) -> &ZPackedMerkleContext<'a> {
        &self.merkle_context
    }

    fn get_lamports(&self) -> Option<U64> {
        None
    }

    fn get_root_index(&self) -> Option<U16> {
        self.root_index.map(|x| (*x).into())
    }

    fn get_address(&self) -> Option<[u8; 32]> {
        None
    }
}

#[derive(Debug, ZeroCopy)]
pub struct InputAccountMetaWithAddressNoLamports {
    /// Merkle tree context.
    pub merkle_context: PackedMerkleContext,
    /// Address.
    pub address: [u8; 32],
    /// Root index.
    pub root_index: Option<u16>,
}

impl InputAccountMetaTrait for InputAccountMetaWithAddressNoLamports {
    fn get_merkle_context(&self) -> &PackedMerkleContext {
        &self.merkle_context
    }

    fn get_lamports(&self) -> Option<u64> {
        None
    }

    fn get_root_index(&self) -> Option<u16> {
        self.root_index
    }

    fn get_address(&self) -> Option<[u8; 32]> {
        Some(self.address)
    }
}

impl<'a> ZInputAccountMetaTrait<'a> for ZInputAccountMetaWithAddressNoLamports<'a> {
    fn get_merkle_context(&self) -> &ZPackedMerkleContext<'a> {
        &self.merkle_context
    }

    fn get_lamports(&self) -> Option<U64> {
        None
    }

    fn get_root_index(&self) -> Option<U16> {
        self.root_index.map(|x| (*x).into())
    }

    fn get_address(&self) -> Option<[u8; 32]> {
        None
    }
}

#[derive(Debug, ZeroCopy)]
pub struct InputAccountMetaWithAddress {
    /// Merkle tree context.
    pub merkle_context: PackedMerkleContext,
    /// Lamports.
    pub lamports: u64,
    /// Address.
    pub address: [u8; 32],
    /// Root index.
    pub root_index: Option<u16>,
}

impl InputAccountMetaTrait for InputAccountMetaWithAddress {
    fn get_merkle_context(&self) -> &PackedMerkleContext {
        &self.merkle_context
    }

    fn get_lamports(&self) -> Option<u64> {
        Some(self.lamports)
    }

    fn get_root_index(&self) -> Option<u16> {
        self.root_index
    }

    fn get_address(&self) -> Option<[u8; 32]> {
        Some(self.address)
    }
}

impl<'a> ZInputAccountMetaTrait<'a> for ZInputAccountMetaWithAddress<'a> {
    fn get_merkle_context(&self) -> &ZPackedMerkleContext<'a> {
        &self.merkle_context
    }

    fn get_lamports(&self) -> Option<U64> {
        Some(*self.lamports)
    }

    fn get_root_index(&self) -> Option<U16> {
        self.root_index.map(|x| (*x).into())
    }

    fn get_address(&self) -> Option<[u8; 32]> {
        Some(*self.address)
    }
}

pub trait InputAccountMetaTrait {
    fn get_merkle_context(&self) -> &PackedMerkleContext;
    fn get_lamports(&self) -> Option<u64>;
    fn get_root_index(&self) -> Option<u16>;
    fn get_address(&self) -> Option<[u8; 32]>;
}
use light_zero_copy::{U16, U64};

pub trait ZInputAccountMetaTrait<'a> {
    fn get_merkle_context(&self) -> &ZPackedMerkleContext<'a>;
    fn get_lamports(&self) -> Option<U64>;
    fn get_root_index(&self) -> Option<U16>;
    fn get_address(&self) -> Option<[u8; 32]>;
}

#[derive(
    Debug, Clone, Copy, BorshDeserialize, BorshSerialize, PartialEq, Default, ZeroCopy, ZeroCopyEq,
)]
pub struct PackedAddressMerkleContext {
    pub address_merkle_tree_pubkey_index: u8,
    pub address_queue_pubkey_index: u8,
    pub root_index: u16,
}
/// Client compressed account meta.
/// TODO: move to client and or consider to remove (could be returned by rpc)
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, PartialEq, Default, ZeroCopy)]
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

// impl LightAccountMeta {
//     #[allow(clippy::too_many_arguments)]
//     pub fn new_init(
//         output_merkle_tree: &Pubkey,
//         address_merkle_context: Option<&AddressMerkleContext>,
//         address_merkle_tree_root_index: Option<u16>,
//         remaining_accounts: &mut RemainingAccounts,
//     ) -> Self {
//         let output_merkle_tree_index = remaining_accounts.insert_or_get(*output_merkle_tree);
//         let address_merkle_context = address_merkle_context.map(|ctx| {
//             pack_address_merkle_context(
//                 ctx,
//                 address_merkle_tree_root_index.unwrap(),
//                 remaining_accounts,
//             )
//         });
//         Self {
//             lamports: None,
//             address: None,
//             data: None,
//             merkle_context: None,
//             merkle_tree_root_index: None,
//             output_merkle_tree_index: Some(output_merkle_tree_index),
//             address_merkle_context,
//             address_merkle_tree_root_index,
//             read_only: false,
//         }
//     }

//     #[allow(clippy::too_many_arguments)]
//     pub fn new_mut(
//         compressed_account: &CompressedAccountWithMerkleContext,
//         merkle_tree_root_index: u16,
//         output_merkle_tree: &Pubkey,
//         remaining_accounts: &mut RemainingAccounts,
//     ) -> Self {
//         let merkle_context =
//             pack_merkle_context(&compressed_account.merkle_context, remaining_accounts);

//         // If no output Merkle tree was specified, use the one used for the
//         // input account.
//         let output_merkle_tree_index = remaining_accounts.insert_or_get(*output_merkle_tree);

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
//         remaining_accounts: &mut RemainingAccounts,
//     ) -> Self {
//         let merkle_context =
//             pack_merkle_context(&compressed_account.merkle_context, remaining_accounts);
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

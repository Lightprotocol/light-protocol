use std::collections::HashMap;

use anchor_lang::prelude::{AccountMeta, AnchorDeserialize, AnchorSerialize, Pubkey};

// TODO(vadorovsky): Consider moving these structs here.
pub use light_system_program::sdk::compressed_account::{MerkleContext, PackedMerkleContext};

/// Collection of remaining accounts which are sent to the program.
#[derive(Default)]
pub struct RemainingAccounts {
    next_index: u8,
    map: HashMap<Pubkey, u8>,
}

impl RemainingAccounts {
    /// Returns the index of the provided `pubkey` in the collection.
    ///
    /// If the provided `pubkey` is not a part of the collection, it gets
    /// inserted with a `next_index`.
    ///
    /// If the privided `pubkey` already exists in the collection, its already
    /// existing index is returned.
    pub fn insert_or_get(&mut self, pubkey: Pubkey) -> u8 {
        *self.map.entry(pubkey).or_insert_with(|| {
            let index = self.next_index;
            self.next_index += 1;
            index
        })
    }

    /// Converts the collection of accounts to a vector of
    /// [`AccountMeta`](solana_sdk::instruction::AccountMeta), which can be used
    /// as remaining accounts in instructions or CPI calls.
    pub fn to_account_metas(&self) -> Vec<AccountMeta> {
        let mut remaining_accounts = self
            .map
            .iter()
            .map(|(k, i)| {
                (
                    AccountMeta {
                        pubkey: *k,
                        is_signer: false,
                        is_writable: true,
                    },
                    *i as usize,
                )
            })
            .collect::<Vec<(AccountMeta, usize)>>();
        // hash maps are not sorted so we need to sort manually and collect into a vector again
        remaining_accounts.sort_by(|a, b| a.1.cmp(&b.1));
        let remaining_accounts = remaining_accounts
            .iter()
            .map(|(k, _)| k.clone())
            .collect::<Vec<AccountMeta>>();
        remaining_accounts
    }
}

pub fn pack_merkle_contexts(
    merkle_contexts: &[MerkleContext],
    remaining_accounts: &mut RemainingAccounts,
) -> Vec<PackedMerkleContext> {
    merkle_contexts
        .iter()
        .map(|x| {
            let merkle_tree_pubkey_index = remaining_accounts.insert_or_get(x.merkle_tree_pubkey);
            let nullifier_queue_pubkey_index =
                remaining_accounts.insert_or_get(x.nullifier_queue_pubkey);
            PackedMerkleContext {
                merkle_tree_pubkey_index,
                nullifier_queue_pubkey_index,
                leaf_index: x.leaf_index,
                queue_index: x.queue_index,
            }
        })
        .collect::<Vec<_>>()
}

pub fn pack_merkle_context(
    merkle_context: MerkleContext,
    remaining_accounts: &mut RemainingAccounts,
) -> PackedMerkleContext {
    pack_merkle_contexts(&[merkle_context], remaining_accounts)[0]
}

/// Context which contains the accounts necessary for emitting the output
/// compressed account.
///
/// The difference between `MerkleOutputContext` and `MerkleContext` is that
/// the former can be used only for creating new accounts and therefore does
/// not contain:
///
/// - nullifier queue (because the output accout is just being created)
/// - `leaf_index` (because it does not exist yet)
#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct MerkleOutputContext {
    pub merkle_tree_pubkey: Pubkey,
}

/// Context which contains the indices of accounts necessary for emitting the
/// output compressed account.
///
/// The difference between `MerkleOutputContext` and `MerkleContext` is that
/// the former can be used only for creating new accounts and therefore does
/// not contain:
///
/// - nullifier queue (because the output accout is just being created)
/// - `leaf_index` (because it does not exist yet)
#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct PackedMerkleOutputContext {
    pub merkle_tree_pubkey_index: u8,
}

/// Returns a vector of [`PackedMerkleOutputContext`] and fills up `remaining_accounts`
/// based on the given `merkle_contexts`.
pub fn pack_merkle_output_contexts(
    merkle_contexts: &[MerkleOutputContext],
    remaining_accounts: &mut RemainingAccounts,
) -> Vec<PackedMerkleOutputContext> {
    merkle_contexts
        .iter()
        .map(|x| {
            let merkle_tree_pubkey_index = remaining_accounts.insert_or_get(x.merkle_tree_pubkey);
            PackedMerkleOutputContext {
                merkle_tree_pubkey_index,
            }
        })
        .collect::<Vec<_>>()
}

/// Returns a [`PackedMerkleOutputContext`] and fills up `remaining_accounts` based
/// on the given `merkle_output_context`.
pub fn pack_merkle_output_context(
    merkle_output_context: MerkleOutputContext,
    remaining_accounts: &mut RemainingAccounts,
) -> PackedMerkleOutputContext {
    pack_merkle_output_contexts(&[merkle_output_context], remaining_accounts)[0]
}

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct AddressMerkleContext {
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_queue_pubkey: Pubkey,
}

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct PackedAddressMerkleContext {
    pub address_merkle_tree_pubkey_index: u8,
    pub address_queue_pubkey_index: u8,
}

/// Returns a vector of [`PackedAddressMerkleContext`] and fills up
/// `remaining_accounts` based on the given `merkle_contexts`.
pub fn pack_address_merkle_contexts(
    address_merkle_contexts: &[AddressMerkleContext],
    remaining_accounts: &mut RemainingAccounts,
) -> Vec<PackedAddressMerkleContext> {
    address_merkle_contexts
        .iter()
        .map(|x| {
            let address_merkle_tree_pubkey_index =
                remaining_accounts.insert_or_get(x.address_merkle_tree_pubkey);
            let address_queue_pubkey_index =
                remaining_accounts.insert_or_get(x.address_queue_pubkey);
            PackedAddressMerkleContext {
                address_merkle_tree_pubkey_index,
                address_queue_pubkey_index,
            }
        })
        .collect::<Vec<_>>()
}

/// Returns a [`PackedAddressMerkleContext`] and fills up `remaining_accounts`
/// based on the given `merkle_context`.
pub fn pack_address_merkle_context(
    address_merkle_context: AddressMerkleContext,
    remaining_accounts: &mut RemainingAccounts,
) -> PackedAddressMerkleContext {
    pack_address_merkle_contexts(&[address_merkle_context], remaining_accounts)[0]
}

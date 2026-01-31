//! Utilities for packing accounts into instruction data.
//!
//! [`PackedAccounts`] is a builder for efficiently organizing accounts into the three categories
//! required for compressed account instructions:
//! 1. **Pre-accounts** - Custom accounts needed before system accounts
//! 2. **System accounts** - Static light system program accounts
//! 3. **Packed accounts** - Dynamically packed accounts (Merkle trees, address trees, queues) with automatic deduplication

use std::collections::HashMap;

use solana_instruction::AccountMeta;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// Builder to collect accounts for compressed account instructions.
///
/// Manages three categories of accounts:
/// - **Pre-accounts**: Signers and other custom accounts that come before system accounts.
/// - **System accounts**: Light system program accounts (authority, trees, queues).
/// - **Packed accounts**: Dynamically tracked deduplicated accounts.
#[derive(Default, Debug)]
pub struct PackedAccounts {
    /// Accounts that must come before system accounts (e.g., signers, fee payer).
    pub pre_accounts: Vec<AccountMeta>,
    /// Light system program accounts (authority, programs, trees, queues).
    system_accounts: Vec<AccountMeta>,
    /// Next available index for packed accounts.
    next_index: u8,
    /// Map of pubkey to (index, AccountMeta) for deduplication and index tracking.
    map: HashMap<Pubkey, (u8, AccountMeta)>,
    /// Field to sanity check
    system_accounts_set: bool,
}

impl PackedAccounts {
    pub fn system_accounts_set(&self) -> bool {
        self.system_accounts_set
    }

    pub fn add_pre_accounts_signer(&mut self, pubkey: Pubkey) {
        self.pre_accounts.push(AccountMeta {
            pubkey,
            is_signer: true,
            is_writable: false,
        });
    }

    pub fn add_pre_accounts_signer_mut(&mut self, pubkey: Pubkey) {
        self.pre_accounts.push(AccountMeta {
            pubkey,
            is_signer: true,
            is_writable: true,
        });
    }

    pub fn add_pre_accounts_meta(&mut self, account_meta: AccountMeta) {
        self.pre_accounts.push(account_meta);
    }

    pub fn add_pre_accounts_metas(&mut self, account_metas: &[AccountMeta]) {
        self.pre_accounts.extend_from_slice(account_metas);
    }

    pub fn add_system_accounts_raw(&mut self, system_accounts: Vec<AccountMeta>) {
        self.system_accounts.extend(system_accounts);
        self.system_accounts_set = true;
    }

    /// Returns the index of the provided `pubkey` in the collection.
    ///
    /// If the provided `pubkey` is not a part of the collection, it gets
    /// inserted with a `next_index`.
    ///
    /// If the provided `pubkey` already exists in the collection, its already
    /// existing index is returned.
    pub fn insert_or_get(&mut self, pubkey: Pubkey) -> u8 {
        self.insert_or_get_config(pubkey, false, true)
    }

    pub fn insert_or_get_read_only(&mut self, pubkey: Pubkey) -> u8 {
        self.insert_or_get_config(pubkey, false, false)
    }

    pub fn insert_or_get_config(
        &mut self,
        pubkey: Pubkey,
        is_signer: bool,
        is_writable: bool,
    ) -> u8 {
        match self.map.get_mut(&pubkey) {
            Some((index, entry)) => {
                if !entry.is_writable {
                    entry.is_writable = is_writable;
                }
                if !entry.is_signer {
                    entry.is_signer = is_signer;
                }
                *index
            }
            None => {
                let index = self.next_index;
                self.next_index += 1;
                self.map.insert(
                    pubkey,
                    (
                        index,
                        AccountMeta {
                            pubkey,
                            is_signer,
                            is_writable,
                        },
                    ),
                );
                index
            }
        }
    }

    fn hash_set_accounts_to_metas(&self) -> Vec<AccountMeta> {
        let mut packed_accounts = self.map.iter().collect::<Vec<_>>();
        // hash maps are not sorted so we need to sort manually and collect into a vector again
        packed_accounts.sort_by(|a, b| a.1 .0.cmp(&b.1 .0));
        let packed_accounts = packed_accounts
            .iter()
            .map(|(_, (_, k))| k.clone())
            .collect::<Vec<AccountMeta>>();
        packed_accounts
    }

    fn get_offsets(&self) -> (usize, usize) {
        let system_accounts_start_offset = self.pre_accounts.len();
        let packed_accounts_start_offset =
            system_accounts_start_offset + self.system_accounts.len();
        (system_accounts_start_offset, packed_accounts_start_offset)
    }

    /// Converts the collection of accounts to a vector of
    /// [`AccountMeta`](solana_instruction::AccountMeta), which can be used
    /// as remaining accounts in instructions or CPI calls.
    ///
    /// # Returns
    ///
    /// A tuple of `(account_metas, system_accounts_offset, packed_accounts_offset)`:
    /// - `account_metas`: All accounts concatenated in order: `[pre_accounts][system_accounts][packed_accounts]`
    /// - `system_accounts_offset`: Index where system accounts start (= pre_accounts.len())
    /// - `packed_accounts_offset`: Index where packed accounts start (= pre_accounts.len() + system_accounts.len())
    pub fn to_account_metas(&self) -> (Vec<AccountMeta>, usize, usize) {
        let packed_accounts = self.hash_set_accounts_to_metas();
        let (system_accounts_start_offset, packed_accounts_start_offset) = self.get_offsets();
        (
            [
                self.pre_accounts.clone(),
                self.system_accounts.clone(),
                packed_accounts,
            ]
            .concat(),
            system_accounts_start_offset,
            packed_accounts_start_offset,
        )
    }

    pub fn packed_pubkeys(&self) -> Vec<Pubkey> {
        self.hash_set_accounts_to_metas()
            .iter()
            .map(|meta| meta.pubkey)
            .collect()
    }

    pub fn add_custom_system_accounts<T: AccountMetasVec>(
        &mut self,
        accounts: T,
    ) -> Result<(), ProgramError> {
        accounts.get_account_metas_vec(self)
    }
}

pub trait AccountMetasVec {
    fn get_account_metas_vec(&self, accounts: &mut PackedAccounts) -> Result<(), ProgramError>;
}

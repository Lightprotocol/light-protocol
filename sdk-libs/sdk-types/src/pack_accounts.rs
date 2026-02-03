//! Utilities for packing accounts into instruction data.
//!
//! [`PackedAccounts`] is a builder for efficiently organizing accounts into the three categories
//! required for compressed account instructions:
//! 1. **Pre-accounts** - Custom accounts needed before system accounts
//! 2. **System accounts** - Static light system program accounts
//! 3. **Packed accounts** - Dynamically packed accounts (Merkle trees, address trees, queues) with automatic deduplication

use std::collections::HashMap;

use light_account_checks::AccountMetaTrait;

use crate::error::LightSdkTypesError;

/// Builder to collect accounts for compressed account instructions.
///
/// Generic over `AM: AccountMetaTrait` to work with both solana and pinocchio account metas.
///
/// Manages three categories of accounts:
/// - **Pre-accounts**: Signers and other custom accounts that come before system accounts.
/// - **System accounts**: Light system program accounts (authority, trees, queues).
/// - **Packed accounts**: Dynamically tracked deduplicated accounts.
#[derive(Debug)]
pub struct PackedAccounts<AM: AccountMetaTrait> {
    /// Accounts that must come before system accounts (e.g., signers, fee payer).
    pub pre_accounts: Vec<AM>,
    /// Light system program accounts (authority, programs, trees, queues).
    system_accounts: Vec<AM>,
    /// Next available index for packed accounts.
    next_index: u8,
    /// Map of pubkey bytes to (index, AccountMeta) for deduplication and index tracking.
    map: HashMap<[u8; 32], (u8, AM)>,
    /// Field to sanity check
    system_accounts_set: bool,
}

impl<AM: AccountMetaTrait> Default for PackedAccounts<AM> {
    fn default() -> Self {
        Self {
            pre_accounts: Vec::new(),
            system_accounts: Vec::new(),
            next_index: 0,
            map: HashMap::new(),
            system_accounts_set: false,
        }
    }
}

impl<AM: AccountMetaTrait> PackedAccounts<AM> {
    pub fn system_accounts_set(&self) -> bool {
        self.system_accounts_set
    }

    pub fn add_pre_accounts_signer(&mut self, pubkey: AM::Pubkey) {
        self.pre_accounts.push(AM::new(pubkey, true, false));
    }

    pub fn add_pre_accounts_signer_mut(&mut self, pubkey: AM::Pubkey) {
        self.pre_accounts.push(AM::new(pubkey, true, true));
    }

    pub fn add_pre_accounts_meta(&mut self, account_meta: AM) {
        self.pre_accounts.push(account_meta);
    }

    pub fn add_pre_accounts_metas(&mut self, account_metas: &[AM]) {
        self.pre_accounts.extend_from_slice(account_metas);
    }

    pub fn add_system_accounts_raw(&mut self, system_accounts: Vec<AM>) {
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
    pub fn insert_or_get(&mut self, pubkey: AM::Pubkey) -> u8 {
        self.insert_or_get_config(pubkey, false, true)
    }

    pub fn insert_or_get_read_only(&mut self, pubkey: AM::Pubkey) -> u8 {
        self.insert_or_get_config(pubkey, false, false)
    }

    pub fn insert_or_get_config(
        &mut self,
        pubkey: AM::Pubkey,
        is_signer: bool,
        is_writable: bool,
    ) -> u8 {
        let bytes = AM::pubkey_to_bytes(pubkey);
        match self.map.get_mut(&bytes) {
            Some((index, entry)) => {
                if !entry.is_writable() {
                    entry.set_is_writable(is_writable);
                }
                if !entry.is_signer() {
                    entry.set_is_signer(is_signer);
                }
                *index
            }
            None => {
                let index = self.next_index;
                self.next_index += 1;
                self.map
                    .insert(bytes, (index, AM::new(pubkey, is_signer, is_writable)));
                index
            }
        }
    }

    fn hash_set_accounts_to_metas(&self) -> Vec<AM> {
        let mut packed_accounts = self.map.iter().collect::<Vec<_>>();
        // hash maps are not sorted so we need to sort manually and collect into a vector again
        packed_accounts.sort_by(|a, b| a.1 .0.cmp(&b.1 .0));
        packed_accounts
            .iter()
            .map(|(_, (_, k))| k.clone())
            .collect::<Vec<AM>>()
    }

    fn get_offsets(&self) -> (usize, usize) {
        let system_accounts_start_offset = self.pre_accounts.len();
        let packed_accounts_start_offset =
            system_accounts_start_offset + self.system_accounts.len();
        (system_accounts_start_offset, packed_accounts_start_offset)
    }

    /// Converts the collection of accounts to a vector of account metas,
    /// which can be used as remaining accounts in instructions or CPI calls.
    ///
    /// # Returns
    ///
    /// A tuple of `(account_metas, system_accounts_offset, packed_accounts_offset)`:
    /// - `account_metas`: All accounts concatenated in order: `[pre_accounts][system_accounts][packed_accounts]`
    /// - `system_accounts_offset`: Index where system accounts start (= pre_accounts.len())
    /// - `packed_accounts_offset`: Index where packed accounts start (= pre_accounts.len() + system_accounts.len())
    pub fn to_account_metas(&self) -> (Vec<AM>, usize, usize) {
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

    pub fn packed_pubkeys(&self) -> Vec<[u8; 32]> {
        self.hash_set_accounts_to_metas()
            .iter()
            .map(|meta| meta.pubkey_bytes())
            .collect()
    }

    pub fn add_custom_system_accounts<T: AccountMetasVec<AM>>(
        &mut self,
        accounts: T,
    ) -> Result<(), LightSdkTypesError> {
        accounts.get_account_metas_vec(self)
    }
}

pub trait AccountMetasVec<AM: AccountMetaTrait> {
    fn get_account_metas_vec(
        &self,
        accounts: &mut PackedAccounts<AM>,
    ) -> Result<(), LightSdkTypesError>;
}

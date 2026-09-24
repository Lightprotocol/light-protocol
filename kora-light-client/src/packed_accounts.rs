//! Shared packed accounts builder for deduplicating pubkeys with flag upgrading.
//!
//! Used by Transfer2 and Decompress instruction builders to build the packed
//! accounts suffix in the accounts array.

use std::collections::HashMap;

use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

/// Builder that deduplicates pubkeys and upgrades `is_signer`/`is_writable` flags.
///
/// Packed accounts are referenced by u8 index in instruction data.
pub(crate) struct PackedAccountsBuilder {
    indices: HashMap<Pubkey, u8>,
    accounts: Vec<(Pubkey, bool, bool)>, // (pubkey, is_signer, is_writable)
}

impl PackedAccountsBuilder {
    pub fn new() -> Self {
        Self {
            indices: HashMap::new(),
            accounts: Vec::new(),
        }
    }

    /// Insert a pubkey or upgrade its flags if already present. Returns the u8 index.
    pub fn insert_or_get(&mut self, pubkey: Pubkey, is_signer: bool, is_writable: bool) -> u8 {
        if let Some(&idx) = self.indices.get(&pubkey) {
            if is_writable {
                self.accounts[idx as usize].2 = true;
            }
            if is_signer {
                self.accounts[idx as usize].1 = true;
            }
            idx
        } else {
            let idx = self.accounts.len() as u8;
            self.indices.insert(pubkey, idx);
            self.accounts.push((pubkey, is_signer, is_writable));
            idx
        }
    }

    /// Get the index of a previously inserted pubkey. Panics if not found.
    pub fn get_index(&self, pubkey: &Pubkey) -> u8 {
        self.indices[pubkey]
    }

    /// Build the final `AccountMeta` list for the packed accounts suffix.
    pub fn build_account_metas(&self) -> Vec<AccountMeta> {
        self.accounts
            .iter()
            .map(|(pubkey, is_signer, is_writable)| {
                if *is_writable {
                    AccountMeta::new(*pubkey, *is_signer)
                } else {
                    AccountMeta::new_readonly(*pubkey, *is_signer)
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_new() {
        let mut builder = PackedAccountsBuilder::new();
        let pk = Pubkey::new_unique();
        let idx = builder.insert_or_get(pk, false, true);
        assert_eq!(idx, 0);
        assert_eq!(builder.get_index(&pk), 0);
    }

    #[test]
    fn test_insert_duplicate_upgrades_flags() {
        let mut builder = PackedAccountsBuilder::new();
        let pk = Pubkey::new_unique();

        // Insert as readonly, non-signer
        builder.insert_or_get(pk, false, false);

        // Re-insert as writable signer — flags should upgrade
        let idx = builder.insert_or_get(pk, true, true);
        assert_eq!(idx, 0);

        let metas = builder.build_account_metas();
        assert_eq!(metas.len(), 1);
        assert!(metas[0].is_signer);
        assert!(metas[0].is_writable);
    }

    #[test]
    fn test_insert_duplicate_no_downgrade() {
        let mut builder = PackedAccountsBuilder::new();
        let pk = Pubkey::new_unique();

        // Insert as writable signer
        builder.insert_or_get(pk, true, true);

        // Re-insert as readonly non-signer — flags must NOT downgrade
        builder.insert_or_get(pk, false, false);

        let metas = builder.build_account_metas();
        assert!(metas[0].is_signer);
        assert!(metas[0].is_writable);
    }

    #[test]
    fn test_get_index() {
        let mut builder = PackedAccountsBuilder::new();
        let pk1 = Pubkey::new_unique();
        let pk2 = Pubkey::new_unique();
        let pk3 = Pubkey::new_unique();

        builder.insert_or_get(pk1, false, false);
        builder.insert_or_get(pk2, false, false);
        builder.insert_or_get(pk3, false, false);

        assert_eq!(builder.get_index(&pk1), 0);
        assert_eq!(builder.get_index(&pk2), 1);
        assert_eq!(builder.get_index(&pk3), 2);
    }

    #[test]
    #[should_panic]
    fn test_get_index_panics_on_missing() {
        let builder = PackedAccountsBuilder::new();
        let pk = Pubkey::new_unique();
        builder.get_index(&pk);
    }

    #[test]
    fn test_build_account_metas() {
        let mut builder = PackedAccountsBuilder::new();
        let pk1 = Pubkey::new_unique();
        let pk2 = Pubkey::new_unique();
        let pk3 = Pubkey::new_unique();

        builder.insert_or_get(pk1, true, false); // signer, readonly
        builder.insert_or_get(pk2, false, true); // non-signer, writable
        builder.insert_or_get(pk3, true, true); // signer, writable

        let metas = builder.build_account_metas();
        assert_eq!(metas.len(), 3);

        assert_eq!(metas[0].pubkey, pk1);
        assert!(metas[0].is_signer);
        assert!(!metas[0].is_writable);

        assert_eq!(metas[1].pubkey, pk2);
        assert!(!metas[1].is_signer);
        assert!(metas[1].is_writable);

        assert_eq!(metas[2].pubkey, pk3);
        assert!(metas[2].is_signer);
        assert!(metas[2].is_writable);
    }
}

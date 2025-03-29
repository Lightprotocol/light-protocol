use std::collections::HashMap;

use crate::{
    cpi::accounts::{get_light_system_account_metas, SystemAccountMetaConfig},
    AccountMeta, Pubkey,
};

/// Collection of remaining accounts which are sent to the program.
#[derive(Default)]
pub struct PackedAccounts {
    next_index: u8,
    map: HashMap<Pubkey, (u8, AccountMeta)>,
}

impl PackedAccounts {
    pub fn new_with_system_accounts(config: SystemAccountMetaConfig) -> Self {
        let mut remaining_accounts = PackedAccounts::default();
        remaining_accounts.add_system_accounts(config);
        remaining_accounts
    }

    pub fn add_system_accounts(&mut self, config: SystemAccountMetaConfig) {
        for account in get_light_system_account_metas(config) {
            self.insert_or_get_config(account.pubkey, account.is_signer, account.is_writable);
        }
    }

    /// Returns the index of the provided `pubkey` in the collection.
    ///
    /// If the provided `pubkey` is not a part of the collection, it gets
    /// inserted with a `next_index`.
    ///
    /// If the privided `pubkey` already exists in the collection, its already
    /// existing index is returned.
    pub fn insert_or_get(&mut self, pubkey: Pubkey) -> u8 {
        self.insert_or_get_config(pubkey, false, true)
    }

    pub fn insert_or_get_signer(&mut self, pubkey: Pubkey) -> u8 {
        self.insert_or_get_config(pubkey, true, false)
    }

    pub fn insert_or_get_signer_mut(&mut self, pubkey: Pubkey) -> u8 {
        self.insert_or_get_config(pubkey, true, true)
    }

    pub fn insert_or_get_config(
        &mut self,
        pubkey: Pubkey,
        is_signer: bool,
        is_writable: bool,
    ) -> u8 {
        self.map
            .entry(pubkey)
            .or_insert_with(|| {
                let index = self.next_index;
                self.next_index += 1;
                (
                    index,
                    AccountMeta {
                        pubkey,
                        is_signer,
                        is_writable,
                    },
                )
            })
            .0
    }

    /// Converts the collection of accounts to a vector of
    /// [`AccountMeta`](solana_sdk::instruction::AccountMeta), which can be used
    /// as remaining accounts in instructions or CPI calls.
    pub fn to_account_metas(&self) -> Vec<AccountMeta> {
        let mut remaining_accounts = self.map.iter().collect::<Vec<_>>();
        // hash maps are not sorted so we need to sort manually and collect into a vector again
        remaining_accounts.sort_by(|a, b| a.1 .0.cmp(&b.1 .0));
        let remaining_accounts = remaining_accounts
            .iter()
            .map(|(_, (_, k))| k.clone())
            .collect::<Vec<AccountMeta>>();
        remaining_accounts
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_remaining_accounts() {
        let mut remaining_accounts = PackedAccounts::default();

        let pubkey_1 = Pubkey::new_unique();
        let pubkey_2 = Pubkey::new_unique();
        let pubkey_3 = Pubkey::new_unique();
        let pubkey_4 = Pubkey::new_unique();

        // Initial insertion.
        assert_eq!(remaining_accounts.insert_or_get(pubkey_1), 0);
        assert_eq!(remaining_accounts.insert_or_get(pubkey_2), 1);
        assert_eq!(remaining_accounts.insert_or_get(pubkey_3), 2);

        assert_eq!(
            remaining_accounts.to_account_metas().as_slice(),
            &[
                AccountMeta {
                    pubkey: pubkey_1,
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: pubkey_2,
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: pubkey_3,
                    is_signer: false,
                    is_writable: true,
                }
            ]
        );

        // Insertion of already existing pubkeys.
        assert_eq!(remaining_accounts.insert_or_get(pubkey_1), 0);
        assert_eq!(remaining_accounts.insert_or_get(pubkey_2), 1);
        assert_eq!(remaining_accounts.insert_or_get(pubkey_3), 2);

        assert_eq!(
            remaining_accounts.to_account_metas().as_slice(),
            &[
                AccountMeta {
                    pubkey: pubkey_1,
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: pubkey_2,
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: pubkey_3,
                    is_signer: false,
                    is_writable: true,
                }
            ]
        );

        // Again, initial insertion.
        assert_eq!(remaining_accounts.insert_or_get(pubkey_4), 3);

        assert_eq!(
            remaining_accounts.to_account_metas().as_slice(),
            &[
                AccountMeta {
                    pubkey: pubkey_1,
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: pubkey_2,
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: pubkey_3,
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: pubkey_4,
                    is_signer: false,
                    is_writable: true,
                }
            ]
        );
    }
}

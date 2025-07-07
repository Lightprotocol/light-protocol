use std::collections::HashMap;

use crate::{
    instruction::system_accounts::{get_light_system_account_metas, SystemAccountMetaConfig},
    AccountMeta, Pubkey,
};

#[derive(Default, Debug)]
pub struct PackedAccounts {
    pub pre_accounts: Vec<AccountMeta>,
    system_accounts: Vec<AccountMeta>,
    next_index: u8,
    map: HashMap<Pubkey, (u8, AccountMeta)>,
}

impl PackedAccounts {
    pub fn new_with_system_accounts(config: SystemAccountMetaConfig) -> Self {
        let mut remaining_accounts = PackedAccounts::default();
        remaining_accounts.add_system_accounts(config);
        remaining_accounts
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

    pub fn add_system_accounts(&mut self, config: SystemAccountMetaConfig) {
        self.system_accounts
            .extend(get_light_system_account_metas(config));
        if let Some(pubkey) = config.cpi_context {
            if self.next_index != 0 {
                panic!("next index must be 0 when adding cpi context");
            }
            self.next_index += 1;
            self.system_accounts.push(AccountMeta::new(pubkey, false));
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

    pub fn insert_or_get_read_only(&mut self, pubkey: Pubkey) -> u8 {
        self.insert_or_get_config(pubkey, false, false)
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
            remaining_accounts.to_account_metas().0.as_slice(),
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
            remaining_accounts.to_account_metas().0.as_slice(),
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
            remaining_accounts.to_account_metas().0.as_slice(),
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

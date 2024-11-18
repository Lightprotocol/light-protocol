use std::collections::HashMap;

use solana_program::{instruction::AccountMeta, pubkey::Pubkey};

use crate::{
    system_accounts::SYSTEM_ACCOUNTS_LEN, PROGRAM_ID_ACCOUNT_COMPRESSION, PROGRAM_ID_LIGHT_SYSTEM,
    PROGRAM_ID_NOOP, PROGRAM_ID_SYSTEM,
};

/// Collection of Light Protocol system accounts which are sent to the program.
pub struct LightInstructionAccounts {
    next_index: u8,
    map: HashMap<Pubkey, u8>,
    // Optional accounts.
    //
    // We can't include them in the hash map, because there is no way to
    // handle the `None` case with it (it would mean inserting the same
    // element) twice. Instead, we insert the optional accounts at the
    // beginning.
    sol_pool_pda: Option<Pubkey>,
    decompression_recipient: Option<Pubkey>,
}

impl LightInstructionAccounts {
    pub fn new(
        registered_program_pda: &Pubkey,
        account_compression_authority: &Pubkey,
        program_id: &Pubkey,
        sol_pool_pda: Option<&Pubkey>,
        decompression_recipient: Option<&Pubkey>,
    ) -> Self {
        let mut accounts = Self {
            // We reserve the first two incides for `sol_pool_pd`
            next_index: 2,
            map: HashMap::new(),
            sol_pool_pda: sol_pool_pda.cloned(),
            decompression_recipient: decompression_recipient.cloned(),
        };

        accounts.insert_or_get(*registered_program_pda);
        accounts.insert_or_get(PROGRAM_ID_NOOP);
        accounts.insert_or_get(*account_compression_authority);
        accounts.insert_or_get(PROGRAM_ID_ACCOUNT_COMPRESSION);
        accounts.insert_or_get(*program_id);
        accounts.insert_or_get(PROGRAM_ID_SYSTEM);
        accounts.insert_or_get(PROGRAM_ID_LIGHT_SYSTEM);

        accounts
    }

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
        let mut account_metas = Vec::with_capacity(self.map.len() + 2);

        // The trick for having `None` accounts is to pass any repeating public
        // key, see https://github.com/coral-xyz/anchor/pull/2101
        let sol_pool_pda = self.sol_pool_pda.unwrap_or(PROGRAM_ID_LIGHT_SYSTEM);
        let decompression_recipient = self
            .decompression_recipient
            .unwrap_or(PROGRAM_ID_LIGHT_SYSTEM);

        let mut accounts = self
            .map
            .iter()
            .map(|(k, i)| {
                let i = *i as usize;
                // Only Merkle tree accouts (specified after the system ones)
                // are writable.
                let is_writable = i >= SYSTEM_ACCOUNTS_LEN - 2;
                (
                    AccountMeta {
                        pubkey: *k,
                        is_signer: false,
                        is_writable,
                    },
                    i,
                )
            })
            .collect::<Vec<(AccountMeta, usize)>>();

        // Hash maps are not sorted (and there is no flavor of hash maps which
        // are automatically sorted **by value**), so we need to sort manually
        // and collect into a vector again.
        accounts.sort_by(|a, b| a.1.cmp(&b.1));
        account_metas.extend(accounts.into_iter().map(|(k, _)| k));

        // Insert `sol_pool_pda` and `decompression_recipient` in indices
        // expected by light-system-program.
        account_metas.insert(
            // The expected index of `sol_pool_pda` is 7.
            // But we don't include `fee_payer` and `authority` here yet.
            // Therefore, 7-2 = 5.
            5,
            AccountMeta {
                pubkey: sol_pool_pda,
                is_signer: false,
                is_writable: false,
            },
        );
        account_metas.insert(
            // The expected index of `decompression_recipient` is 8.
            // But we don't include `fee_payer` and `authority` here yet.
            // Therefore, 8-2 = 6.
            6,
            AccountMeta {
                pubkey: decompression_recipient,
                is_signer: false,
                is_writable: false,
            },
        );

        println!("ACCOUNT METAS INIT: {account_metas:?}");

        account_metas
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_remaining_accounts() {
        let registered_program_pda = Pubkey::new_unique();
        let account_compression_authority = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();

        let mut accounts = LightInstructionAccounts::new(
            &registered_program_pda,
            &account_compression_authority,
            &program_id,
            None,
            None,
        );

        let pubkey_1 = Pubkey::new_unique();
        let pubkey_2 = Pubkey::new_unique();
        let pubkey_3 = Pubkey::new_unique();
        let pubkey_4 = Pubkey::new_unique();

        // Initial insertion.
        assert_eq!(accounts.insert_or_get(pubkey_1), 9);
        assert_eq!(accounts.insert_or_get(pubkey_2), 10);
        assert_eq!(accounts.insert_or_get(pubkey_3), 11);

        assert_eq!(
            accounts.to_account_metas().as_slice(),
            &[
                AccountMeta {
                    pubkey: PROGRAM_ID_LIGHT_SYSTEM,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: PROGRAM_ID_LIGHT_SYSTEM,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: registered_program_pda,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: PROGRAM_ID_NOOP,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: account_compression_authority,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: PROGRAM_ID_ACCOUNT_COMPRESSION,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: program_id,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: PROGRAM_ID_SYSTEM,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: PROGRAM_ID_LIGHT_SYSTEM,
                    is_signer: false,
                    is_writable: false,
                },
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
        assert_eq!(accounts.insert_or_get(pubkey_1), 9);
        assert_eq!(accounts.insert_or_get(pubkey_2), 10);
        assert_eq!(accounts.insert_or_get(pubkey_3), 11);

        assert_eq!(
            accounts.to_account_metas().as_slice(),
            &[
                AccountMeta {
                    pubkey: PROGRAM_ID_LIGHT_SYSTEM,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: PROGRAM_ID_LIGHT_SYSTEM,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: registered_program_pda,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: PROGRAM_ID_NOOP,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: account_compression_authority,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: PROGRAM_ID_ACCOUNT_COMPRESSION,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: program_id,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: PROGRAM_ID_SYSTEM,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: PROGRAM_ID_LIGHT_SYSTEM,
                    is_signer: false,
                    is_writable: false,
                },
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
        assert_eq!(accounts.insert_or_get(pubkey_4), 12);

        assert_eq!(
            accounts.to_account_metas().as_slice(),
            &[
                AccountMeta {
                    pubkey: PROGRAM_ID_LIGHT_SYSTEM,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: PROGRAM_ID_LIGHT_SYSTEM,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: registered_program_pda,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: PROGRAM_ID_NOOP,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: account_compression_authority,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: PROGRAM_ID_ACCOUNT_COMPRESSION,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: program_id,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: PROGRAM_ID_SYSTEM,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: PROGRAM_ID_LIGHT_SYSTEM,
                    is_signer: false,
                    is_writable: false,
                },
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

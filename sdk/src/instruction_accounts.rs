use std::collections::HashMap;

use solana_program::{instruction::AccountMeta, pubkey::Pubkey};

use crate::{
    system_accounts::SYSTEM_ACCOUNTS_LEN, PROGRAM_ID_ACCOUNT_COMPRESSION, PROGRAM_ID_LIGHT_SYSTEM,
    PROGRAM_ID_NOOP, PROGRAM_ID_SYSTEM,
};

/// Collection of Light Protocol system accounts which are sent to the program.
pub struct LightInstructionAccounts {
    next_index: u8,
    pub map: HashMap<Pubkey, u8>,
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
            // We reserve the first two indices for `sol_pool_pda` and
            // `decompression_recipient`.
            next_index: 2,
            map: HashMap::new(),
            sol_pool_pda: sol_pool_pda.cloned(),
            decompression_recipient: decompression_recipient.cloned(),
        };

        println!("1 REGISTERED_PROGRAM_PDA: {:?}", registered_program_pda);
        println!("2 PROGRAM_ID_NOOP: {:?}", PROGRAM_ID_NOOP);
        println!(
            "3 ACCOUNT_COMPRESSION_AUTHORITY: {:?}",
            account_compression_authority
        );
        println!(
            "4 PROGRAM_ID_ACCOUNT_COMPRESSION: {:?}",
            PROGRAM_ID_ACCOUNT_COMPRESSION
        );
        println!("5 SELF PROGRAM_ID: {:?}", program_id);
        println!("6 PROGRAM_ID_SYSTEM: {:?}", PROGRAM_ID_SYSTEM);
        println!("7 PROGRAM_ID_LIGHT_SYSTEM: {:?}", PROGRAM_ID_LIGHT_SYSTEM);
        println!("/ SOL_POOL_PDA: {:?}", sol_pool_pda);
        println!("/ DECOMPRESSION_RECIPIENT: {:?}", decompression_recipient);
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

        let sol_pool_pda = self.sol_pool_pda.unwrap_or(PROGRAM_ID_LIGHT_SYSTEM);
        let decompression_recipient = self
            .decompression_recipient
            .unwrap_or(PROGRAM_ID_LIGHT_SYSTEM);

        let mut accounts = self
            .map
            .iter()
            .map(|(k, i)| {
                let i = *i as usize;
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

        accounts.sort_by(|a, b| a.1.cmp(&b.1));
        account_metas.extend(accounts.into_iter().map(|(k, _)| k));

        account_metas.push(AccountMeta {
            pubkey: sol_pool_pda,
            is_signer: false,
            is_writable: false,
        });
        account_metas.push(AccountMeta {
            pubkey: decompression_recipient,
            is_signer: false,
            is_writable: false,
        });

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

    #[test]
    fn test_merkle_tree_account_indices() {
        let registered_program_pda = Pubkey::new_unique();
        let account_compression_authority = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();

        // Create merkle tree test accounts
        let merkle_tree = Pubkey::new_unique();
        let nullifier_queue = Pubkey::new_unique();

        let mut accounts = LightInstructionAccounts::new(
            &registered_program_pda,
            &account_compression_authority,
            &program_id,
            None,
            None,
        );

        // Add merkle tree accounts
        let merkle_tree_idx = accounts.insert_or_get(merkle_tree);
        let queue_idx = accounts.insert_or_get(nullifier_queue);

        // Get final account metas
        let account_metas = accounts.to_account_metas();

        // Verify indices
        assert_eq!(merkle_tree_idx, 9); // First merkle account should be after system accounts
        assert_eq!(queue_idx, 10); // Second merkle account follows

        // Verify writable flag is set correctly
        assert!(account_metas[merkle_tree_idx as usize].is_writable);
        assert!(account_metas[queue_idx as usize].is_writable);

        // Verify system accounts are not writable
        for i in 0..SYSTEM_ACCOUNTS_LEN - 2 {
            assert!(!account_metas[i].is_writable);
        }

        // Verify account ordering matches expected layout
        assert_eq!(account_metas[merkle_tree_idx as usize].pubkey, merkle_tree);
        assert_eq!(account_metas[queue_idx as usize].pubkey, nullifier_queue);
    }

    #[cfg(test)]
    mod test_to_account_metas {
        use super::*;

        #[test]
        fn test_account_ordering() {
            let registered_program_pda = Pubkey::new_unique();
            let account_compression_authority = Pubkey::new_unique();
            let program_id = Pubkey::new_unique();
            let sol_pool = Pubkey::new_unique();
            let recipient = Pubkey::new_unique();

            let accounts = LightInstructionAccounts::new(
                &registered_program_pda,
                &account_compression_authority,
                &program_id,
                Some(&sol_pool),
                Some(&recipient),
            );

            let metas = accounts.to_account_metas();

            // Check system accounts are in correct order
            assert_eq!(metas[0].pubkey, registered_program_pda);
            assert_eq!(metas[1].pubkey, PROGRAM_ID_NOOP);
            assert_eq!(metas[2].pubkey, account_compression_authority);
            assert_eq!(metas[3].pubkey, PROGRAM_ID_ACCOUNT_COMPRESSION);
            assert_eq!(metas[4].pubkey, program_id);
            assert_eq!(metas[5].pubkey, sol_pool);
            assert_eq!(metas[6].pubkey, recipient);
            assert_eq!(metas[7].pubkey, PROGRAM_ID_SYSTEM);
            assert_eq!(metas[8].pubkey, PROGRAM_ID_LIGHT_SYSTEM);
        }

        #[test]
        fn test_none_accounts_use_light_system() {
            let registered_program_pda = Pubkey::new_unique();
            let account_compression_authority = Pubkey::new_unique();
            let program_id = Pubkey::new_unique();

            let accounts = LightInstructionAccounts::new(
                &registered_program_pda,
                &account_compression_authority,
                &program_id,
                None,
                None,
            );

            let metas = accounts.to_account_metas();

            // Check None accounts use PROGRAM_ID_LIGHT_SYSTEM
            assert_eq!(metas[5].pubkey, PROGRAM_ID_LIGHT_SYSTEM);
            assert_eq!(metas[6].pubkey, PROGRAM_ID_LIGHT_SYSTEM);
        }

        #[test]
        fn test_writable_flags() {
            let registered_program_pda = Pubkey::new_unique();
            let account_compression_authority = Pubkey::new_unique();
            let program_id = Pubkey::new_unique();
            let merkle_account = Pubkey::new_unique();

            let mut accounts = LightInstructionAccounts::new(
                &registered_program_pda,
                &account_compression_authority,
                &program_id,
                None,
                None,
            );

            // Add a merkle account after system accounts
            accounts.insert_or_get(merkle_account);

            let metas = accounts.to_account_metas();

            // System accounts should not be writable
            for meta in &metas[0..SYSTEM_ACCOUNTS_LEN - 2] {
                assert!(!meta.is_writable);
            }

            // Merkle account should be writable
            assert!(metas[9].is_writable);
        }

        #[test]
        fn test_signer_flags() {
            let registered_program_pda = Pubkey::new_unique();
            let account_compression_authority = Pubkey::new_unique();
            let program_id = Pubkey::new_unique();

            let accounts = LightInstructionAccounts::new(
                &registered_program_pda,
                &account_compression_authority,
                &program_id,
                None,
                None,
            );

            let metas = accounts.to_account_metas();

            // No accounts should be signers
            for meta in metas {
                assert!(!meta.is_signer);
            }
        }
    }
}

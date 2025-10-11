//! Utilities for packing accounts into instruction data.
//!
//! [`PackedAccounts`] is a builder for efficiently organizing accounts into the three categories
//! required for compressed account instructions:
//! 1. **Pre-accounts** - Custom accounts needed before system accounts
//! 2. **System accounts** - Static light system program accounts
//! 3. **Packed accounts** - Dynamically packed accounts (Merkle trees, address trees, queues) with automatic deduplication
//!
//!
//! ## System Account Versioning
//!
//! **`add_system_accounts()` is complementary to [`cpi::v1::CpiAccounts`](crate::cpi::v1::CpiAccounts)**
//! **`add_system_accounts_v2()` is complementary to [`cpi::v2::CpiAccounts`](crate::cpi::v2::CpiAccounts)**
//!
//! Always use the matching version - v1 client-side account packing with v1 program-side CPI,
//! and v2 with v2. Mixing versions will cause account layout mismatches.
//!
//! # Example: Creating a compressed PDA
//!
//! ```rust
//! # use light_sdk::instruction::{PackedAccounts, SystemAccountMetaConfig};
//! # use solana_pubkey::Pubkey;
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let program_id = Pubkey::new_unique();
//! # let payer_pubkey = Pubkey::new_unique();
//! # let merkle_tree_pubkey = Pubkey::new_unique();
//! // Initialize with system accounts
//! let system_account_meta_config = SystemAccountMetaConfig::new(program_id);
//! let mut accounts = PackedAccounts::default();
//!
//! // Add pre-accounts (signers)
//! accounts.add_pre_accounts_signer(payer_pubkey);
//!
//! // Add Light system program accounts (v2)
//! #[cfg(feature = "v2")]
//! accounts.add_system_accounts_v2(system_account_meta_config)?;
//! #[cfg(not(feature = "v2"))]
//! accounts.add_system_accounts(system_account_meta_config)?;
//!
//! // Add Merkle tree accounts (automatically tracked and deduplicated)
//! let output_merkle_tree_index = accounts.insert_or_get(merkle_tree_pubkey);
//!
//! // Convert to final account metas with offsets
//! let (account_metas, system_accounts_offset, tree_accounts_offset) = accounts.to_account_metas();
//! # assert_eq!(output_merkle_tree_index, 0);
//! # Ok(())
//! # }
//! ```
//!
//! # Account Organization
//!
//! The final account layout is:
//! ```text
//! [pre_accounts] [system_accounts] [packed_accounts]
//!     ↑                ↑                  ↑
//!  Signers,       Light system      Merkle trees,
//!  fee payer      program accts     address trees
//! ```
//!
//! # Automatic Deduplication
//!
//! ```rust
//! # use light_sdk::instruction::PackedAccounts;
//! # use solana_pubkey::Pubkey;
//! let mut accounts = PackedAccounts::default();
//! let tree_pubkey = Pubkey::new_unique();
//! let other_tree = Pubkey::new_unique();
//!
//! // First insertion gets index 0
//! let index1 = accounts.insert_or_get(tree_pubkey);
//! assert_eq!(index1, 0);
//!
//! // Same tree inserted again returns same index (deduplicated)
//! let index2 = accounts.insert_or_get(tree_pubkey);
//! assert_eq!(index2, 0);
//!
//! // Different tree gets next index
//! let index3 = accounts.insert_or_get(other_tree);
//! assert_eq!(index3, 1);
//! ```
//!
//! # Building Instructions with Anchor Programs
//!
//! When building instructions for Anchor programs, concatenate your custom accounts with the packed accounts:
//!
//! ```rust,ignore
//! # use anchor_lang::InstructionData;
//! # use light_sdk::instruction::{PackedAccounts, SystemAccountMetaConfig};
//! # use solana_instruction::{AccountMeta, Instruction};
//!
//! // 1. Set up packed accounts
//! let config = SystemAccountMetaConfig::new(program_id);
//! let mut remaining_accounts = PackedAccounts::default();
//! remaining_accounts.add_system_accounts(config)?;
//!
//! // 2. Pack tree accounts from proof result
//! let packed_tree_info = proof_result.pack_tree_infos(&mut remaining_accounts);
//! let output_tree_index = state_tree_info.pack_output_tree_index(&mut remaining_accounts)?;
//!
//! // 3. Convert to account metas
//! let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();
//!
//! // 4. Build instruction: custom accounts first, then remaining_accounts
//! let instruction = Instruction {
//!     program_id: your_program::ID,
//!     accounts: [
//!         vec![AccountMeta::new(payer.pubkey(), true)],  // Your program's accounts
//!         // Add other custom accounts here if needed
//!         remaining_accounts,                             // Light system accounts + trees
//!     ]
//!     .concat(),
//!     data: your_program::instruction::YourInstruction {
//!         proof: proof_result.proof,
//!         address_tree_info: packed_tree_info.address_trees[0],
//!         output_tree_index,
//!         // ... your other fields
//!     }
//!     .data(),
//! };
//! ```

use std::collections::HashMap;

use crate::{
    error::LightSdkError,
    instruction::system_accounts::{get_light_system_account_metas, SystemAccountMetaConfig},
    AccountMeta, Pubkey,
};

/// Builder for organizing accounts into compressed account instructions.
///
/// Manages three categories of accounts:
/// - **Pre-accounts**: Signers and other custom accounts that come before system accounts.
/// - **System accounts**: Light system program accounts (authority, trees, queues).
/// - **Packed accounts**: Dynamically tracked deduplicted accounts.
///
/// # Example
///
/// ```rust
/// # use light_sdk::instruction::{PackedAccounts, SystemAccountMetaConfig};
/// # use solana_pubkey::Pubkey;
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let payer_pubkey = Pubkey::new_unique();
/// # let program_id = Pubkey::new_unique();
/// # let merkle_tree_pubkey = Pubkey::new_unique();
/// let mut accounts = PackedAccounts::default();
///
/// // Add signer
/// accounts.add_pre_accounts_signer(payer_pubkey);
///
/// // Add system accounts (use v2 if feature is enabled)
/// let config = SystemAccountMetaConfig::new(program_id);
/// #[cfg(feature = "v2")]
/// accounts.add_system_accounts_v2(config)?;
/// #[cfg(not(feature = "v2"))]
/// accounts.add_system_accounts(config)?;
///
/// // Add and track tree accounts
/// let tree_index = accounts.insert_or_get(merkle_tree_pubkey);
///
/// // Get final account metas
/// let (metas, system_offset, tree_offset) = accounts.to_account_metas();
/// # assert_eq!(tree_index, 0);
/// # Ok(())
/// # }
/// ```
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
    pub fn new_with_system_accounts(config: SystemAccountMetaConfig) -> crate::error::Result<Self> {
        let mut remaining_accounts = PackedAccounts::default();
        remaining_accounts.add_system_accounts(config)?;
        Ok(remaining_accounts)
    }

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

    /// Adds v1 Light system program accounts to the account list.
    ///
    /// **Use with [`cpi::v1::CpiAccounts`](crate::cpi::v1::CpiAccounts) on the program side.**
    ///
    /// This adds all the accounts required by the Light system program for v1 operations,
    /// including the CPI authority, registered programs, account compression program, and Noop program.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use light_sdk::instruction::{PackedAccounts, SystemAccountMetaConfig};
    /// # use solana_pubkey::Pubkey;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let program_id = Pubkey::new_unique();
    /// let mut accounts = PackedAccounts::default();
    /// let config = SystemAccountMetaConfig::new(program_id);
    /// accounts.add_system_accounts(config)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_system_accounts(
        &mut self,
        config: SystemAccountMetaConfig,
    ) -> crate::error::Result<()> {
        self.system_accounts
            .extend(get_light_system_account_metas(config));
        // note cpi context account is part of the system accounts
        /*  if let Some(pubkey) = config.cpi_context {
            if self.next_index != 0 {
                return Err(crate::error::LightSdkError::CpiContextOrderingViolation);
            }
            self.insert_or_get(pubkey);
        }*/
        Ok(())
    }

    /// Adds v2 Light system program accounts to the account list.
    ///
    /// **Use with [`cpi::v2::CpiAccounts`](crate::cpi::v2::CpiAccounts) on the program side.**
    ///
    /// This adds all the accounts required by the Light system program for v2 operations.
    /// V2 uses a different account layout optimized for batched state trees.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[cfg(feature = "v2")]
    /// # {
    /// # use light_sdk::instruction::{PackedAccounts, SystemAccountMetaConfig};
    /// # use solana_pubkey::Pubkey;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let program_id = Pubkey::new_unique();
    /// let mut accounts = PackedAccounts::default();
    /// let config = SystemAccountMetaConfig::new(program_id);
    /// accounts.add_system_accounts_v2(config)?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    #[cfg(feature = "v2")]
    pub fn add_system_accounts_v2(
        &mut self,
        config: SystemAccountMetaConfig,
    ) -> crate::error::Result<()> {
        self.system_accounts
            .extend(crate::instruction::get_light_system_account_metas_v2(
                config,
            ));
        // note cpi context account is part of the system accounts
        /*  if let Some(pubkey) = config.cpi_context {
            if self.next_index != 0 {
                return Err(crate::error::LightSdkError::CpiContextOrderingViolation);
            }
            self.insert_or_get(pubkey);
        }*/
        Ok(())
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
    ///
    /// The `system_accounts_offset` can be used to slice the accounts when creating [`CpiAccounts`](crate::cpi::v1::CpiAccounts):
    /// ```ignore
    /// let accounts_for_cpi = &ctx.remaining_accounts[system_accounts_offset..];
    /// let cpi_accounts = CpiAccounts::new(fee_payer, accounts_for_cpi, cpi_signer)?;
    /// ```
    ///
    /// The offset can be hardcoded if your program always has the same pre-accounts layout, or passed
    /// as a field in your instruction data.
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
    ) -> crate::error::Result<()> {
        accounts.get_account_metas_vec(self)
    }
}

pub trait AccountMetasVec {
    fn get_account_metas_vec(&self, accounts: &mut PackedAccounts) -> Result<(), LightSdkError>;
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

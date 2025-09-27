//! # Light Account
//!
//! LightAccount is a wrapper around a compressed account similar to anchor Account.
//! LightAccount abstracts hashing of compressed account data,
//! and wraps the compressed account data so that it is easy to use.
//!
//! Data structs used with LightAccount must implement the traits:
//! - LightDiscriminator
//! - BorshSerialize, BorshDeserialize
//! - Debug, Default, Clone
//!
//! ### Account Data Hashing
//!
//! Sha256 data hashing is the recommended for most use cases.
//! Account data is serialized into a vector with borsh and hashed with Sha256.
//!
//! Poseidon data hashing is recommended zk use cases.
//! The data struct needs to implement the DataHasher implementation.
//! The LightHasher derives, the DataHasher trait a hashing scheme from the compressed account layout.
//! Alternatively, DataHasher can be implemented manually.
//! Poseidon hashing is CU intensive and has limitations with regards to hash inputs see Poseidon module for details.
//!
//!
//! ### Compressed account with LightDiscriminator
//! ```
//! use light_sdk::LightDiscriminator;
//! use solana_pubkey::Pubkey;
//! use borsh::{BorshSerialize, BorshDeserialize};
//! #[derive(Clone, Debug, Default, LightDiscriminator, BorshSerialize, BorshDeserialize)]
//! pub struct CounterAccount {
//!     pub owner: Pubkey,
//!     pub counter: u64,
//! }
//! ```
//!
//!
//! ### Create compressed account
//! ```rust
//! use light_sdk::{LightAccount, LightDiscriminator};
//! use borsh::{BorshSerialize, BorshDeserialize};
//! use solana_pubkey::Pubkey;
//!
//! #[derive(Clone, Debug, Default, LightDiscriminator, BorshSerialize, BorshDeserialize)]
//! pub struct CounterAccount {
//!     pub owner: Pubkey,
//!     pub counter: u64,
//! };
//!
//! let program_id = Pubkey::new_unique();
//! let address = [0u8; 32];
//! let output_tree_index = 0u8;
//! let owner = Pubkey::new_unique();
//!
//! let mut my_compressed_account = LightAccount::<'_, CounterAccount>::new_init(
//!     &program_id,
//!     // Address
//!     Some(address),
//!     output_tree_index,
//! );
//! // Set data:
//! my_compressed_account.owner = owner;
//! ```
//! ### Update compressed account
//! ```rust
//! use light_sdk::{LightAccount, LightDiscriminator};
//! use light_sdk::instruction::account_meta::CompressedAccountMeta;
//! use borsh::{BorshSerialize, BorshDeserialize};
//! use solana_pubkey::Pubkey;
//!
//! #[derive(Clone, Debug, Default, LightDiscriminator, BorshSerialize, BorshDeserialize)]
//! pub struct CounterAccount {
//!     pub owner: Pubkey,
//!     pub counter: u64,
//! };
//!
//! let program_id = Pubkey::new_unique();
//! let account_meta = CompressedAccountMeta::default();
//! let compressed_account_data = CounterAccount::default();
//!
//! let mut my_compressed_account = LightAccount::<'_, CounterAccount>::new_mut(
//!     &program_id,
//!     &account_meta,
//!     compressed_account_data,
//! ).unwrap();
//! // Increment counter.
//! my_compressed_account.counter += 1;
//! ```
//! ### Close compressed account
//! ```rust
//! use light_sdk::{LightAccount, LightDiscriminator};
//! use light_sdk::instruction::account_meta::CompressedAccountMetaClose;
//! use borsh::{BorshSerialize, BorshDeserialize};
//! use solana_pubkey::Pubkey;
//!
//! #[derive(Clone, Debug, Default, LightDiscriminator, BorshSerialize, BorshDeserialize)]
//! pub struct CounterAccount {
//!     pub owner: Pubkey,
//!     pub counter: u64,
//! };
//!
//! let program_id = Pubkey::new_unique();
//! let account_meta_close = CompressedAccountMetaClose::default();
//! let compressed_account_data = CounterAccount::default();
//!
//! let _my_compressed_account = LightAccount::<'_, CounterAccount>::new_close(
//!     &program_id,
//!     &account_meta_close,
//!     compressed_account_data,
//! ).unwrap();
//! ```
// TODO: add example for manual hashing

use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use light_compressed_account::{
    compressed_account::PackedMerkleContext,
    instruction_data::with_account_info::{CompressedAccountInfo, InAccountInfo, OutAccountInfo},
};
use light_sdk_types::instruction::account_meta::CompressedAccountMetaTrait;
use solana_pubkey::Pubkey;

use crate::{
    error::LightSdkError,
    light_hasher::{DataHasher, Hasher, Poseidon, Sha256},
    AnchorDeserialize, AnchorSerialize, LightDiscriminator,
};

const DEFAULT_DATA_HASH: [u8; 32] = [0u8; 32];

pub trait Size {
    fn size(&self) -> usize;
}
pub use sha::LightAccount;
/// SHA256 borsh flat hashed Light Account.
/// This is the recommended account type for most use cases.
pub mod sha {
    use super::*;
    /// Light Account variant that uses SHA256 hashing with flat borsh serialization.
    /// This is the recommended account type for most use cases.
    pub type LightAccount<'a, A> = super::LightAccountInner<'a, Sha256, A, true>;
}

/// Poseidon hashed Light Account.
/// Poseidon hashing is zk friendly and enables you to do zk proofs over your compressed account data.
pub mod poseidon {
    use super::*;
    /// Light Account type using Poseidon hashing.
    /// Poseidon hashing is zk friendly and enables you to do zk proofs over your compressed account.
    /// ### Compressed account with LightHasher and LightDiscriminator
    /// ```rust
    /// use light_sdk::{LightHasher, LightDiscriminator};
    /// use solana_pubkey::Pubkey;
    /// #[derive(Clone, Debug, Default, LightHasher, LightDiscriminator)]
    /// pub struct CounterAccount {
    ///     #[hash]
    ///     pub owner: Pubkey,
    ///     pub counter: u64,
    /// }
    /// ```
    /// Constraints:
    /// - Poseidon hashes can only take up to 12 inputs
    ///   -> use nested structs for structs with more than 12 fields.
    /// - Poseidon hashes inputs must be less than bn254 field size (254 bits).
    ///   hash_to_field_size methods in light hasher can be used to hash data longer than 253 bits.
    ///   -> use the `#[hash]` attribute for fields with data types greater than 31 bytes eg Pubkeys.
    pub type LightAccount<'a, A> = super::LightAccountInner<'a, Poseidon, A, false>;
}

#[doc(hidden)]
pub use __internal::LightAccountInner;

/// INTERNAL IMPLEMENTATION - DO NOT USE DIRECTLY
/// **Use the type aliases instead:**
/// - `LightAccount` for Poseidon hashing
/// - `sha::LightAccount` for SHA256 hashing
#[doc(hidden)]
pub mod __internal {
    use light_sdk_types::instruction::account_meta::CompressedAccountMetaClose;
    use solana_program_error::ProgramError;

    use super::*;

    #[doc(hidden)]
    #[derive(Debug, PartialEq)]
    pub struct LightAccountInner<
        'a,
        H: Hasher,
        A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + Default,
        const HASH_FLAT: bool,
    > {
        owner: &'a Pubkey,
        pub account: A,
        account_info: CompressedAccountInfo,
        should_remove_data: bool,
        _hasher: PhantomData<H>,
    }

    impl<
            'a,
            H: Hasher,
            A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + Default,
            const HASH_FLAT: bool,
        > LightAccountInner<'a, H, A, HASH_FLAT>
    {
        pub fn new_init(
            owner: &'a Pubkey,
            address: Option<[u8; 32]>,
            output_state_tree_index: u8,
        ) -> Self {
            let output_account_info = OutAccountInfo {
                output_merkle_tree_index: output_state_tree_index,
                discriminator: A::LIGHT_DISCRIMINATOR,
                ..Default::default()
            };
            Self {
                owner,
                account: A::default(),
                account_info: CompressedAccountInfo {
                    address,
                    input: None,
                    output: Some(output_account_info),
                },
                should_remove_data: false,
                _hasher: PhantomData,
            }
        }

        pub fn discriminator(&self) -> &[u8; 8] {
            &A::LIGHT_DISCRIMINATOR
        }

        pub fn lamports(&self) -> u64 {
            if let Some(output) = self.account_info.output.as_ref() {
                output.lamports
            } else if let Some(input) = self.account_info.input.as_ref() {
                input.lamports
            } else {
                0
            }
        }

        pub fn lamports_mut(&mut self) -> &mut u64 {
            if let Some(output) = self.account_info.output.as_mut() {
                &mut output.lamports
            } else if let Some(input) = self.account_info.input.as_mut() {
                &mut input.lamports
            } else {
                panic!("No lamports field available in account_info")
            }
        }

        pub fn address(&self) -> &Option<[u8; 32]> {
            &self.account_info.address
        }

        pub fn owner(&self) -> &Pubkey {
            self.owner
        }

        pub fn in_account_info(&self) -> &Option<InAccountInfo> {
            &self.account_info.input
        }

        pub fn out_account_info(&mut self) -> &Option<OutAccountInfo> {
            &self.account_info.output
        }
    }

    // Specialized implementation for HASH_FLAT = false (structured hashing with DataHasher)
    impl<
            'a,
            H: Hasher,
            A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + DataHasher + Default,
        > LightAccountInner<'a, H, A, false>
    {
        pub fn new_mut(
            owner: &'a Pubkey,
            input_account_meta: &impl CompressedAccountMetaTrait,
            input_account: A,
        ) -> Result<Self, LightSdkError> {
            let input_account_info = {
                // For HASH_FLAT = false, always use DataHasher
                let input_data_hash = input_account.hash::<H>()?;
                let tree_info = input_account_meta.get_tree_info();
                InAccountInfo {
                    data_hash: input_data_hash,
                    lamports: input_account_meta.get_lamports().unwrap_or_default(),
                    merkle_context: PackedMerkleContext {
                        merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
                        queue_pubkey_index: tree_info.queue_pubkey_index,
                        leaf_index: tree_info.leaf_index,
                        prove_by_index: tree_info.prove_by_index,
                    },
                    root_index: input_account_meta.get_root_index().unwrap_or_default(),
                    discriminator: A::LIGHT_DISCRIMINATOR,
                }
            };
            let output_account_info = {
                let output_merkle_tree_index = input_account_meta
                    .get_output_state_tree_index()
                    .ok_or(LightSdkError::OutputStateTreeIndexIsNone)?;
                OutAccountInfo {
                    lamports: input_account_meta.get_lamports().unwrap_or_default(),
                    output_merkle_tree_index,
                    discriminator: A::LIGHT_DISCRIMINATOR,
                    ..Default::default()
                }
            };

            Ok(Self {
                owner,
                account: input_account,
                account_info: CompressedAccountInfo {
                    address: input_account_meta.get_address(),
                    input: Some(input_account_info),
                    output: Some(output_account_info),
                },
                should_remove_data: false,
                _hasher: PhantomData,
            })
        }

        pub fn new_empty(
            owner: &'a Pubkey,
            input_account_meta: &impl CompressedAccountMetaTrait,
            input_account: A,
        ) -> Result<Self, LightSdkError> {
            let input_account_info = {
                let input_data_hash = DEFAULT_DATA_HASH;
                let tree_info = input_account_meta.get_tree_info();
                InAccountInfo {
                    data_hash: input_data_hash,
                    lamports: input_account_meta.get_lamports().unwrap_or_default(),
                    merkle_context: PackedMerkleContext {
                        merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
                        queue_pubkey_index: tree_info.queue_pubkey_index,
                        leaf_index: tree_info.leaf_index,
                        prove_by_index: tree_info.prove_by_index,
                    },
                    root_index: input_account_meta.get_root_index().unwrap_or_default(),
                    discriminator: [0u8; 8],
                }
            };
            let output_account_info = {
                let output_merkle_tree_index = input_account_meta
                    .get_output_state_tree_index()
                    .ok_or(LightSdkError::OutputStateTreeIndexIsNone)?;
                OutAccountInfo {
                    lamports: input_account_meta.get_lamports().unwrap_or_default(),
                    output_merkle_tree_index,
                    discriminator: A::LIGHT_DISCRIMINATOR,
                    ..Default::default()
                }
            };

            Ok(Self {
                owner,
                account: input_account,
                account_info: CompressedAccountInfo {
                    address: input_account_meta.get_address(),
                    input: Some(input_account_info),
                    output: Some(output_account_info),
                },
                should_remove_data: false,
                _hasher: PhantomData,
            })
        }

        pub fn new_close(
            owner: &'a Pubkey,
            input_account_meta: &impl CompressedAccountMetaTrait,
            input_account: A,
        ) -> Result<Self, LightSdkError> {
            let mut account = Self::new_mut(owner, input_account_meta, input_account)?;
            account.should_remove_data = true;

            Ok(account)
        }

        /// Closes the compressed account.
        /// Define whether to close the account permanently or not.
        /// The address of an account that is closed permanently cannot be created again.
        /// For accounts that are not closed permanently the accounts address
        /// continues to exist in an account with discriminator and without data.
        pub fn new_close_permanent(
            owner: &'a Pubkey,
            input_account_meta: &CompressedAccountMetaClose,
            input_account: A,
        ) -> Result<Self, LightSdkError> {
            let input_account_info = {
                // For HASH_FLAT = false, always use DataHasher
                let input_data_hash = input_account.hash::<H>()?;
                let tree_info = input_account_meta.get_tree_info();
                InAccountInfo {
                    data_hash: input_data_hash,
                    lamports: input_account_meta.get_lamports().unwrap_or_default(),
                    merkle_context: PackedMerkleContext {
                        merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
                        queue_pubkey_index: tree_info.queue_pubkey_index,
                        leaf_index: tree_info.leaf_index,
                        prove_by_index: tree_info.prove_by_index,
                    },
                    root_index: input_account_meta.get_root_index().unwrap_or_default(),
                    discriminator: A::LIGHT_DISCRIMINATOR,
                }
            };

            Ok(Self {
                owner,
                account: input_account,
                account_info: CompressedAccountInfo {
                    address: input_account_meta.get_address(),
                    input: Some(input_account_info),
                    output: None,
                },
                should_remove_data: false,
                _hasher: PhantomData,
            })
        }

        pub fn to_account_info(mut self) -> Result<CompressedAccountInfo, ProgramError> {
            if let Some(output) = self.account_info.output.as_mut() {
                if self.should_remove_data {
                    // Data should be empty to close account.
                    if !output.data.is_empty() {
                        return Err(LightSdkError::ExpectedNoData.into());
                    }
                    output.data_hash = DEFAULT_DATA_HASH;
                    output.discriminator = [0u8; 8];
                } else {
                    output.data = self
                        .account
                        .try_to_vec()
                        .map_err(|_| LightSdkError::Borsh)?;
                    // For HASH_FLAT = false, always use DataHasher
                    output.data_hash = self.account.hash::<H>()?;
                }
            }
            Ok(self.account_info)
        }
        pub fn to_in_account(&self) -> Option<InAccount> {
            self.account_info
                .input
                .as_ref()
                .map(|input| input.into_in_account(self.account_info.address))
        }
    }

    // Specialized implementation for HASH_FLAT = true (flat serialization without DataHasher)
    impl<'a, H: Hasher, A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + Default>
        LightAccountInner<'a, H, A, true>
    {
        pub fn new_mut(
            owner: &'a Pubkey,
            input_account_meta: &impl CompressedAccountMetaTrait,
            input_account: A,
        ) -> Result<Self, ProgramError> {
            let input_account_info = {
                // For HASH_FLAT = true, use direct serialization
                let data = input_account
                    .try_to_vec()
                    .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;
                let mut input_data_hash = H::hash(data.as_slice())?;
                input_data_hash[0] = 0;
                let tree_info = input_account_meta.get_tree_info();
                InAccountInfo {
                    data_hash: input_data_hash,
                    lamports: input_account_meta.get_lamports().unwrap_or_default(),
                    merkle_context: PackedMerkleContext {
                        merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
                        queue_pubkey_index: tree_info.queue_pubkey_index,
                        leaf_index: tree_info.leaf_index,
                        prove_by_index: tree_info.prove_by_index,
                    },
                    root_index: input_account_meta.get_root_index().unwrap_or_default(),
                    discriminator: A::LIGHT_DISCRIMINATOR,
                }
            };
            let output_account_info = {
                let output_merkle_tree_index = input_account_meta
                    .get_output_state_tree_index()
                    .ok_or(LightSdkError::OutputStateTreeIndexIsNone)
                    .map_err(ProgramError::from)?;
                OutAccountInfo {
                    lamports: input_account_meta.get_lamports().unwrap_or_default(),
                    output_merkle_tree_index,
                    discriminator: A::LIGHT_DISCRIMINATOR,
                    ..Default::default()
                }
            };

            Ok(Self {
                owner,
                account: input_account,
                account_info: CompressedAccountInfo {
                    address: input_account_meta.get_address(),
                    input: Some(input_account_info),
                    output: Some(output_account_info),
                },
                should_remove_data: false,
                _hasher: PhantomData,
            })
        }

        pub fn new_empty(
            owner: &'a Pubkey,
            input_account_meta: &impl CompressedAccountMetaTrait,
            input_account: A,
        ) -> Result<Self, ProgramError> {
            let input_account_info = {
                let input_data_hash = DEFAULT_DATA_HASH;
                let tree_info = input_account_meta.get_tree_info();
                InAccountInfo {
                    data_hash: input_data_hash,
                    lamports: input_account_meta.get_lamports().unwrap_or_default(),
                    merkle_context: PackedMerkleContext {
                        merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
                        queue_pubkey_index: tree_info.queue_pubkey_index,
                        leaf_index: tree_info.leaf_index,
                        prove_by_index: tree_info.prove_by_index,
                    },
                    root_index: input_account_meta.get_root_index().unwrap_or_default(),
                    discriminator: [0u8; 8],
                }
            };
            let output_account_info = {
                let output_merkle_tree_index = input_account_meta
                    .get_output_state_tree_index()
                    .ok_or(LightSdkError::OutputStateTreeIndexIsNone)
                    .map_err(ProgramError::from)?;
                OutAccountInfo {
                    lamports: input_account_meta.get_lamports().unwrap_or_default(),
                    output_merkle_tree_index,
                    discriminator: A::LIGHT_DISCRIMINATOR,
                    ..Default::default()
                }
            };

            Ok(Self {
                owner,
                account: input_account,
                account_info: CompressedAccountInfo {
                    address: input_account_meta.get_address(),
                    input: Some(input_account_info),
                    output: Some(output_account_info),
                },
                should_remove_data: false,
                _hasher: PhantomData,
            })
        }

        pub fn new_close(
            owner: &'a Pubkey,
            input_account_meta: &impl CompressedAccountMetaTrait,
            input_account: A,
        ) -> Result<Self, ProgramError> {
            let mut account = Self::new_mut(owner, input_account_meta, input_account)?;
            account.should_remove_data = true;
            Ok(account)
        }

        /// Closes the compressed account.
        /// Define whether to close the account permanently or not.
        /// The address of an account that is closed permanently cannot be created again.
        /// For accounts that are not closed permanently the accounts address
        /// continues to exist in an account without discriminator and data.
        pub fn new_close_permanent(
            owner: &'a Pubkey,
            input_account_meta: &CompressedAccountMetaClose,
            input_account: A,
        ) -> Result<Self, ProgramError> {
            let input_account_info = {
                // For HASH_FLAT = true, use direct serialization
                let data = input_account
                    .try_to_vec()
                    .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;
                let mut input_data_hash = H::hash(data.as_slice())?;
                input_data_hash[0] = 0;
                let tree_info = input_account_meta.get_tree_info();
                InAccountInfo {
                    data_hash: input_data_hash,
                    lamports: input_account_meta.get_lamports().unwrap_or_default(),
                    merkle_context: PackedMerkleContext {
                        merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
                        queue_pubkey_index: tree_info.queue_pubkey_index,
                        leaf_index: tree_info.leaf_index,
                        prove_by_index: tree_info.prove_by_index,
                    },
                    root_index: input_account_meta.get_root_index().unwrap_or_default(),
                    discriminator: A::LIGHT_DISCRIMINATOR,
                }
            };

            Ok(Self {
                owner,
                account: input_account,
                account_info: CompressedAccountInfo {
                    address: input_account_meta.get_address(),
                    input: Some(input_account_info),
                    output: None,
                },
                should_remove_data: false,
                _hasher: PhantomData,
            })
        }

        pub fn to_account_info(mut self) -> Result<CompressedAccountInfo, ProgramError> {
            if let Some(output) = self.account_info.output.as_mut() {
                if self.should_remove_data {
                    // Data should be empty to close account.
                    if !output.data.is_empty() {
                        return Err(LightSdkError::ExpectedNoData.into());
                    }
                    output.data_hash = DEFAULT_DATA_HASH;
                    output.discriminator = [0u8; 8];
                } else {
                    output.data = self
                        .account
                        .try_to_vec()
                        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;
                    // For HASH_FLAT = true, use direct serialization
                    output.data_hash = H::hash(output.data.as_slice())?;
                    output.data_hash[0] = 0;
                }
            }
            Ok(self.account_info)
        }
        pub fn to_in_account(&self) -> Option<InAccount> {
            self.account_info
                .input
                .as_ref()
                .map(|input| input.into_in_account(self.account_info.address))
        }
    }

    impl<
            H: Hasher,
            A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + Default,
            const HASH_FLAT: bool,
        > Deref for LightAccountInner<'_, H, A, HASH_FLAT>
    {
        type Target = A;

        fn deref(&self) -> &Self::Target {
            &self.account
        }
    }

    impl<
            H: Hasher,
            A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + Default,
            const HASH_FLAT: bool,
        > DerefMut for LightAccountInner<'_, H, A, HASH_FLAT>
    {
        fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
            &mut self.account
        }
    }
}

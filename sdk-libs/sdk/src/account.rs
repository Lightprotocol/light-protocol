//! # Light Account
//!
//! LightAccount wraps the compressed account data so that it is easy to use similar to anchor Account.
//! LightAccount sets the discriminator and creates the compressed account data hash.
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
//! # use light_sdk::{LightAccount, LightDiscriminator};
//! # use borsh::{BorshSerialize, BorshDeserialize};
//! # use solana_pubkey::Pubkey;
//! #
//! # #[derive(Clone, Debug, Default, LightDiscriminator, BorshSerialize, BorshDeserialize)]
//! # pub struct CounterAccount {
//! #     pub owner: Pubkey,
//! #     pub counter: u64,
//! # }
//! #
//! # let program_id = Pubkey::new_unique();
//! # let address = [0u8; 32];
//! # let output_tree_index = 0u8;
//! # let owner = Pubkey::new_unique();
//! let mut my_compressed_account = LightAccount::<CounterAccount>::new_init(
//!     &program_id,
//!     Some(address),
//!     output_tree_index,
//! );
//! // Set data:
//! my_compressed_account.owner = owner;
//! ```
//! ### Update compressed account
//! ```rust
//! # use light_sdk::{LightAccount, LightDiscriminator};
//! # use light_sdk::instruction::account_meta::CompressedAccountMeta;
//! # use borsh::{BorshSerialize, BorshDeserialize};
//! # use solana_pubkey::Pubkey;
//! # use solana_program_error::ProgramError;
//! #
//! # #[derive(Clone, Debug, Default, LightDiscriminator, BorshSerialize, BorshDeserialize)]
//! # pub struct CounterAccount {
//! #     pub owner: Pubkey,
//! #     pub counter: u64,
//! # }
//! #
//! # fn example() -> Result<(), ProgramError> {
//! # let program_id = Pubkey::new_unique();
//! # let account_meta = CompressedAccountMeta {
//! #     output_state_tree_index: 0,
//! #     ..Default::default()
//! # };
//! # let compressed_account_data = CounterAccount::default();
//! let mut my_compressed_account = LightAccount::<CounterAccount>::new_mut(
//!     &program_id,
//!     &account_meta,
//!     compressed_account_data,
//! )?;
//! // Increment counter.
//! my_compressed_account.counter += 1;
//! # Ok(())
//! # }
//! ```
//! ### Close compressed account
//! ```rust
//! # use light_sdk::{LightAccount, LightDiscriminator};
//! # use light_sdk::instruction::account_meta::CompressedAccountMeta;
//! # use borsh::{BorshSerialize, BorshDeserialize};
//! # use solana_pubkey::Pubkey;
//! # use solana_program_error::ProgramError;
//! #
//! # #[derive(Clone, Debug, Default, LightDiscriminator, BorshSerialize, BorshDeserialize)]
//! # pub struct CounterAccount {
//! #     pub owner: Pubkey,
//! #     pub counter: u64,
//! # }
//! #
//! # fn example() -> Result<(), ProgramError> {
//! # let program_id = Pubkey::new_unique();
//! # let account_meta = CompressedAccountMeta {
//! #     output_state_tree_index: 0,
//! #     ..Default::default()
//! # };
//! # let compressed_account_data = CounterAccount::default();
//! let my_compressed_account = LightAccount::<CounterAccount>::new_close(
//!     &program_id,
//!     &account_meta,
//!     compressed_account_data,
//! )?;
//! # Ok(())
//! # }
//! ```
// TODO: add example for manual hashing

use std::marker::PhantomData;

use light_compressed_account::{
    compressed_account::PackedMerkleContext,
    instruction_data::with_account_info::{CompressedAccountInfo, InAccountInfo, OutAccountInfo},
};
use light_sdk_types::instruction::account_meta::CompressedAccountMetaTrait;
use solana_pubkey::Pubkey;

#[cfg(feature = "poseidon")]
use crate::light_hasher::Poseidon;
use crate::{
    error::LightSdkError,
    light_hasher::{DataHasher, Hasher, Sha256},
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
    pub type LightAccount<A> = super::LightAccountInner<Sha256, A, true>;
}

/// Poseidon hashed Light Account.
/// Poseidon hashing is zk friendly and enables you to do zk proofs over your compressed account data.
#[cfg(feature = "poseidon")]
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
    pub type LightAccount<A> = super::LightAccountInner<Poseidon, A, false>;
}

#[doc(hidden)]
pub use __internal::LightAccountInner;

/// INTERNAL IMPLEMENTATION - DO NOT USE DIRECTLY
/// **Use the type aliases instead:**
/// - `LightAccount` for Poseidon hashing
/// - `sha::LightAccount` for SHA256 hashing
#[doc(hidden)]
pub mod __internal {
    use light_compressed_account::instruction_data::{
        data::OutputCompressedAccountWithPackedContext, with_readonly::InAccount,
    };
    use light_sdk_types::instruction::account_meta::CompressedAccountMetaBurn;
    #[cfg(feature = "v2")]
    use light_sdk_types::instruction::account_meta::CompressedAccountMetaReadOnly;
    use solana_program_error::ProgramError;

    use super::*;

    #[doc(hidden)]
    #[derive(Debug, PartialEq)]
    pub struct LightAccountInner<
        H: Hasher,
        A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + Default,
        const HASH_FLAT: bool,
    > {
        owner: Pubkey,
        pub account: A,
        account_info: CompressedAccountInfo,
        should_remove_data: bool,
        /// If set, this account is read-only and this contains the precomputed account hash.
        pub read_only_account_hash: Option<[u8; 32]>,
        _hasher: PhantomData<H>,
    }

    impl<
            H: Hasher,
            A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + Default,
            const HASH_FLAT: bool,
        > core::ops::Deref for LightAccountInner<H, A, HASH_FLAT>
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
        > core::ops::DerefMut for LightAccountInner<H, A, HASH_FLAT>
    {
        fn deref_mut(&mut self) -> &mut Self::Target {
            assert!(
                self.read_only_account_hash.is_none(),
                "Cannot mutate read-only account"
            );
            &mut self.account
        }
    }

    impl<
            H: Hasher,
            A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + Default,
            const HASH_FLAT: bool,
        > LightAccountInner<H, A, HASH_FLAT>
    {
        pub fn new_init(
            owner: &impl crate::PubkeyTrait,
            address: Option<[u8; 32]>,
            output_state_tree_index: u8,
        ) -> Self {
            let output_account_info = OutAccountInfo {
                output_merkle_tree_index: output_state_tree_index,
                discriminator: A::LIGHT_DISCRIMINATOR,
                ..Default::default()
            };
            Self {
                owner: owner.to_solana_pubkey(),
                account: A::default(),
                account_info: CompressedAccountInfo {
                    address,
                    input: None,
                    output: Some(output_account_info),
                },
                should_remove_data: false,
                read_only_account_hash: None,
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
            &self.owner
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
            H: Hasher,
            A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + DataHasher + Default,
        > LightAccountInner<H, A, false>
    {
        pub fn new_mut(
            owner: &impl crate::PubkeyTrait,
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
                owner: owner.to_solana_pubkey(),
                account: input_account,
                account_info: CompressedAccountInfo {
                    address: input_account_meta.get_address(),
                    input: Some(input_account_info),
                    output: Some(output_account_info),
                },
                should_remove_data: false,
                read_only_account_hash: None,
                _hasher: PhantomData,
            })
        }

        pub fn new_empty(
            owner: &impl crate::PubkeyTrait,
            input_account_meta: &impl CompressedAccountMetaTrait,
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
                owner: owner.to_solana_pubkey(),
                account: A::default(),
                account_info: CompressedAccountInfo {
                    address: input_account_meta.get_address(),
                    input: Some(input_account_info),
                    output: Some(output_account_info),
                },
                should_remove_data: false,
                read_only_account_hash: None,
                _hasher: PhantomData,
            })
        }

        pub fn new_close(
            owner: &impl crate::PubkeyTrait,
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
        pub fn new_burn(
            owner: &impl crate::PubkeyTrait,
            input_account_meta: &CompressedAccountMetaBurn,
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
                owner: owner.to_solana_pubkey(),
                account: input_account,
                account_info: CompressedAccountInfo {
                    address: input_account_meta.get_address(),
                    input: Some(input_account_info),
                    output: None,
                },
                should_remove_data: false,
                read_only_account_hash: None,
                _hasher: PhantomData,
            })
        }

        /// Creates a read-only compressed account for validation without state updates.
        /// Read-only accounts are used to prove that an account exists in a specific state
        /// without modifying it (v2 only).
        ///
        /// # Arguments
        /// * `owner` - The program that owns this compressed account
        /// * `input_account_meta` - Metadata about the existing compressed account
        /// * `input_account` - The account data to validate
        /// * `packed_account_pubkeys` - Slice of packed pubkeys from CPI accounts (packed accounts after system accounts)
        ///
        /// # Note
        /// Data hashing is consistent with the hasher type (H): SHA256 for `sha::LightAccount`,
        /// Poseidon for `LightAccount`. The same hasher is used for both the data hash and account hash.
        #[cfg(feature = "v2")]
        pub fn new_read_only(
            owner: &impl crate::PubkeyTrait,
            input_account_meta: &CompressedAccountMetaReadOnly,
            input_account: A,
            packed_account_pubkeys: &[Pubkey],
        ) -> Result<Self, ProgramError> {
            // Hash account data once and reuse
            let input_data_hash = input_account
                .hash::<H>()
                .map_err(LightSdkError::from)
                .map_err(ProgramError::from)?;
            let tree_info = input_account_meta.get_tree_info();

            let input_account_info = InAccountInfo {
                data_hash: input_data_hash,
                lamports: 0, // read-only accounts don't track lamports
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
                    queue_pubkey_index: tree_info.queue_pubkey_index,
                    leaf_index: tree_info.leaf_index,
                    prove_by_index: tree_info.prove_by_index,
                },
                root_index: input_account_meta.get_root_index().unwrap_or_default(),
                discriminator: A::LIGHT_DISCRIMINATOR,
            };

            // Compute account hash for read-only account
            let account_hash = {
                use light_compressed_account::compressed_account::{
                    CompressedAccount, CompressedAccountData,
                };

                let compressed_account = CompressedAccount {
                    address: Some(input_account_meta.address),
                    owner: owner.to_array().into(),
                    data: Some(CompressedAccountData {
                        data: vec![],               // not used for hash computation
                        data_hash: input_data_hash, // Reuse already computed hash
                        discriminator: A::LIGHT_DISCRIMINATOR,
                    }),
                    lamports: 0,
                };

                // Get merkle tree pubkey from packed pubkeys slice
                let merkle_tree_pubkey = packed_account_pubkeys
                    .get(tree_info.merkle_tree_pubkey_index as usize)
                    .ok_or(LightSdkError::InvalidMerkleTreeIndex)
                    .map_err(ProgramError::from)?
                    .to_bytes()
                    .into();

                compressed_account
                    .hash(&merkle_tree_pubkey, &tree_info.leaf_index, true)
                    .map_err(LightSdkError::from)
                    .map_err(ProgramError::from)?
            };

            Ok(Self {
                owner: owner.to_solana_pubkey(),
                account: input_account,
                account_info: CompressedAccountInfo {
                    address: Some(input_account_meta.address),
                    input: Some(input_account_info),
                    output: None,
                },
                should_remove_data: false,
                read_only_account_hash: Some(account_hash),
                _hasher: PhantomData,
            })
        }

        pub fn to_account_info(mut self) -> Result<CompressedAccountInfo, ProgramError> {
            if self.read_only_account_hash.is_some() {
                return Err(LightSdkError::ReadOnlyAccountCannotUseToAccountInfo.into());
            }

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
                    output.data_hash = self
                        .account
                        .hash::<H>()
                        .map_err(LightSdkError::from)
                        .map_err(ProgramError::from)?;
                }
            }
            Ok(self.account_info)
        }

        #[cfg(feature = "v2")]
        pub fn to_packed_read_only_account(
            self,
        ) -> Result<
            light_compressed_account::compressed_account::PackedReadOnlyCompressedAccount,
            ProgramError,
        > {
            let account_hash = self
                .read_only_account_hash
                .ok_or(LightSdkError::NotReadOnlyAccount)?;

            let input_account = self
                .account_info
                .input
                .ok_or(ProgramError::InvalidAccountData)?;

            use light_compressed_account::compressed_account::PackedReadOnlyCompressedAccount;
            Ok(PackedReadOnlyCompressedAccount {
                root_index: input_account.root_index,
                merkle_context: input_account.merkle_context,
                account_hash,
            })
        }
        pub fn to_in_account(&self) -> Option<InAccount> {
            self.account_info
                .input
                .as_ref()
                .map(|input| input.into_in_account(self.account_info.address))
        }

        pub fn to_output_compressed_account_with_packed_context(
            &self,
            owner: Option<solana_pubkey::Pubkey>,
        ) -> Result<Option<OutputCompressedAccountWithPackedContext>, ProgramError> {
            let owner = if let Some(owner) = owner {
                owner.to_bytes().into()
            } else {
                self.owner.to_bytes().into()
            };

            if let Some(mut output) = self.account_info.output.clone() {
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
                    // For HASH_FLAT = false, always use DataHasher
                    output.data_hash = self
                        .account
                        .hash::<H>()
                        .map_err(LightSdkError::from)
                        .map_err(ProgramError::from)?;
                    output.data = self
                        .account
                        .try_to_vec()
                        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;
                }
                let result = OutputCompressedAccountWithPackedContext::from_with_owner(
                    &output,
                    owner,
                    self.account_info.address,
                );
                Ok(Some(result))
            } else {
                Ok(None)
            }
        }
    }

    // Specialized implementation for HASH_FLAT = true (flat serialization without DataHasher)
    impl<H: Hasher, A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + Default>
        LightAccountInner<H, A, true>
    {
        pub fn new_mut(
            owner: &impl crate::PubkeyTrait,
            input_account_meta: &impl CompressedAccountMetaTrait,
            input_account: A,
        ) -> Result<Self, ProgramError> {
            let input_account_info = {
                // For HASH_FLAT = true, use direct serialization
                let data = input_account
                    .try_to_vec()
                    .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;
                let mut input_data_hash = H::hash(data.as_slice())
                    .map_err(LightSdkError::from)
                    .map_err(ProgramError::from)?;
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
                owner: owner.to_solana_pubkey(),
                account: input_account,
                account_info: CompressedAccountInfo {
                    address: input_account_meta.get_address(),
                    input: Some(input_account_info),
                    output: Some(output_account_info),
                },
                should_remove_data: false,
                read_only_account_hash: None,
                _hasher: PhantomData,
            })
        }

        // TODO: add in a different pr and release
        // pub fn init_if_needed(
        //     owner: &'a Pubkey,
        //     input_account_meta: CompressedAccountMetaInitIfNeeded,
        //     input_account: A,
        // ) -> Result<Self, ProgramError> {
        //     if input_account_meta.init && input_account_meta.with_new_adress {
        //         Ok(Self::new_init(
        //             owner,
        //             Some(input_account_meta.address),
        //             input_account_meta.output_state_tree_index,
        //         ))
        //     } else if input_account_meta.init {
        //         // For new_empty, we need a CompressedAccountMetaTrait implementor
        //         let tree_info = input_account_meta
        //             .tree_info
        //             .ok_or(LightSdkError::ExpectedTreeInfo)
        //             .map_err(ProgramError::from)?;

        //         let meta = CompressedAccountMeta {
        //             tree_info,
        //             address: input_account_meta.address,
        //             output_state_tree_index: input_account_meta.output_state_tree_index,
        //         };
        //         Self::new_empty(owner, &meta)
        //     } else {
        //         // For new_mut, we need a CompressedAccountMetaTrait implementor
        //         let tree_info = input_account_meta
        //             .tree_info
        //             .ok_or(LightSdkError::ExpectedTreeInfo)
        //             .map_err(ProgramError::from)?;
        //         let meta = CompressedAccountMeta {
        //             tree_info,
        //             address: input_account_meta.address,
        //             output_state_tree_index: input_account_meta.output_state_tree_index,
        //         };
        //         Self::new_mut(owner, &meta, input_account)
        //     }
        // }

        pub fn new_empty(
            owner: &impl crate::PubkeyTrait,
            input_account_meta: &impl CompressedAccountMetaTrait,
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
                owner: owner.to_solana_pubkey(),
                account: A::default(),
                account_info: CompressedAccountInfo {
                    address: input_account_meta.get_address(),
                    input: Some(input_account_info),
                    output: Some(output_account_info),
                },
                should_remove_data: false,
                read_only_account_hash: None,
                _hasher: PhantomData,
            })
        }

        /// Closes the compressed account.
        /// Closed accounts can be reopened again (use LightAccount::new_empty to reopen a compressed account.).
        /// If you want to ensure an account cannot be opened again use burn.
        /// Closed accounts preserve the accounts address
        /// in a compressed account without discriminator and data.
        pub fn new_close(
            owner: &impl crate::PubkeyTrait,
            input_account_meta: &impl CompressedAccountMetaTrait,
            input_account: A,
        ) -> Result<Self, ProgramError> {
            let mut account = Self::new_mut(owner, input_account_meta, input_account)?;
            account.should_remove_data = true;
            Ok(account)
        }

        /// Burns the compressed account.
        /// The address of an account that is burned cannot be created again.
        pub fn new_burn(
            owner: &impl crate::PubkeyTrait,
            input_account_meta: &CompressedAccountMetaBurn,
            input_account: A,
        ) -> Result<Self, ProgramError> {
            let input_account_info = {
                // For HASH_FLAT = true, use direct serialization
                let data = input_account
                    .try_to_vec()
                    .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;
                let mut input_data_hash = H::hash(data.as_slice())
                    .map_err(LightSdkError::from)
                    .map_err(ProgramError::from)?;
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
                owner: owner.to_solana_pubkey(),
                account: input_account,
                account_info: CompressedAccountInfo {
                    address: input_account_meta.get_address(),
                    input: Some(input_account_info),
                    output: None,
                },
                should_remove_data: false,
                read_only_account_hash: None,
                _hasher: PhantomData,
            })
        }

        /// Creates a read-only compressed account for validation without state updates.
        /// Read-only accounts are used to prove that an account exists in a specific state
        /// without modifying it (v2 only).
        ///
        /// # Arguments
        /// * `owner` - The program that owns this compressed account
        /// * `input_account_meta` - Metadata about the existing compressed account
        /// * `input_account` - The account data to validate
        /// * `packed_account_pubkeys` - Slice of packed pubkeys from CPI accounts (packed accounts after system accounts)
        ///
        /// # Note
        /// Uses SHA256 flat hashing with borsh serialization (HASH_FLAT = true).
        #[cfg(feature = "v2")]
        pub fn new_read_only(
            owner: &impl crate::PubkeyTrait,
            input_account_meta: &CompressedAccountMetaReadOnly,
            input_account: A,
            packed_account_pubkeys: &[Pubkey],
        ) -> Result<Self, ProgramError> {
            // Hash account data once and reuse (SHA256 flat: borsh serialize then hash)
            let data = input_account
                .try_to_vec()
                .map_err(|_| LightSdkError::Borsh)
                .map_err(ProgramError::from)?;
            let mut input_data_hash = H::hash(data.as_slice())
                .map_err(LightSdkError::from)
                .map_err(ProgramError::from)?;
            input_data_hash[0] = 0;

            let tree_info = input_account_meta.get_tree_info();

            let input_account_info = InAccountInfo {
                data_hash: input_data_hash,
                lamports: 0, // read-only accounts don't track lamports
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
                    queue_pubkey_index: tree_info.queue_pubkey_index,
                    leaf_index: tree_info.leaf_index,
                    prove_by_index: tree_info.prove_by_index,
                },
                root_index: input_account_meta.get_root_index().unwrap_or_default(),
                discriminator: A::LIGHT_DISCRIMINATOR,
            };

            // Compute account hash for read-only account
            let account_hash = {
                use light_compressed_account::compressed_account::{
                    CompressedAccount, CompressedAccountData,
                };

                let compressed_account = CompressedAccount {
                    address: Some(input_account_meta.address),
                    owner: owner.to_array().into(),
                    data: Some(CompressedAccountData {
                        data: vec![],               // not used for hash computation
                        data_hash: input_data_hash, // Reuse already computed hash
                        discriminator: A::LIGHT_DISCRIMINATOR,
                    }),
                    lamports: 0,
                };

                // Get merkle tree pubkey from packed pubkeys slice
                let merkle_tree_pubkey = packed_account_pubkeys
                    .get(tree_info.merkle_tree_pubkey_index as usize)
                    .ok_or(LightSdkError::InvalidMerkleTreeIndex)
                    .map_err(ProgramError::from)?
                    .to_bytes()
                    .into();

                compressed_account
                    .hash(&merkle_tree_pubkey, &tree_info.leaf_index, true)
                    .map_err(LightSdkError::from)
                    .map_err(ProgramError::from)?
            };

            Ok(Self {
                owner: owner.to_solana_pubkey(),
                account: input_account,
                account_info: CompressedAccountInfo {
                    address: Some(input_account_meta.address),
                    input: Some(input_account_info),
                    output: None,
                },
                should_remove_data: false,
                read_only_account_hash: Some(account_hash),
                _hasher: PhantomData,
            })
        }

        pub fn to_account_info(mut self) -> Result<CompressedAccountInfo, ProgramError> {
            if self.read_only_account_hash.is_some() {
                return Err(LightSdkError::ReadOnlyAccountCannotUseToAccountInfo.into());
            }

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
                    output.data_hash = H::hash(output.data.as_slice())
                        .map_err(LightSdkError::from)
                        .map_err(ProgramError::from)?;
                    output.data_hash[0] = 0;
                }
            }
            Ok(self.account_info)
        }

        #[cfg(feature = "v2")]
        pub fn to_packed_read_only_account(
            self,
        ) -> Result<
            light_compressed_account::compressed_account::PackedReadOnlyCompressedAccount,
            ProgramError,
        > {
            let account_hash = self
                .read_only_account_hash
                .ok_or(LightSdkError::NotReadOnlyAccount)?;

            let input_account = self
                .account_info
                .input
                .ok_or(ProgramError::InvalidAccountData)?;

            use light_compressed_account::compressed_account::PackedReadOnlyCompressedAccount;
            Ok(PackedReadOnlyCompressedAccount {
                root_index: input_account.root_index,
                merkle_context: input_account.merkle_context,
                account_hash,
            })
        }

        pub fn to_in_account(&self) -> Option<InAccount> {
            self.account_info
                .input
                .as_ref()
                .map(|input| input.into_in_account(self.account_info.address))
        }

        pub fn to_output_compressed_account_with_packed_context(
            &self,
            owner: Option<solana_pubkey::Pubkey>,
        ) -> Result<Option<OutputCompressedAccountWithPackedContext>, ProgramError> {
            let owner = if let Some(owner) = owner {
                owner.to_bytes().into()
            } else {
                self.owner.to_bytes().into()
            };

            if let Some(mut output) = self.account_info.output.clone() {
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
                    output.data_hash = H::hash(output.data.as_slice())
                        .map_err(LightSdkError::from)
                        .map_err(ProgramError::from)?;
                    output.data_hash[0] = 0;
                    output.data = self
                        .account
                        .try_to_vec()
                        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;
                }

                let result = OutputCompressedAccountWithPackedContext::from_with_owner(
                    &output,
                    owner,
                    self.account_info.address,
                );
                Ok(Some(result))
            } else {
                Ok(None)
            }
        }
    }
}

//! # Light Account
//!
//! LightAccount is a wrapper around a compressed account similar to anchor Account.
//! LightAccount abstracts hashing of compressed account data,
//! and wraps the compressed account data so that it is easy to use.
//!
//! Data structs used with LightAccount must implement the traits:
//! - DataHasher
//! - LightDiscriminator
//! - BorshSerialize, BorshDeserialize
//! - Debug, Default, Clone
//!
//! ### Account Data Hashing
//! The LightHasher derives a hashing scheme from the compressed account layout.
//! Alternatively, DataHasher can be implemented manually.
//!
//! Constraints:
//! - Poseidon hashes can only take up to 12 inputs
//!     -> use nested structs for structs with more than 12 fields.
//! - Poseidon hashes inputs must be less than bn254 field size (254 bits).
//! hash_to_field_size methods in light hasher can be used to hash data longer than 253 bits.
//!     -> use the `#[hash]` attribute for fields with data types greater than 31 bytes eg Pubkeys.
//!
//! ### Compressed account with LightHasher and LightDiscriminator
//! ```
//! use light_sdk::{LightHasher, LightDiscriminator};
//! use solana_pubkey::Pubkey;
//! #[derive(Clone, Debug, Default, LightHasher, LightDiscriminator)]
//! pub struct CounterAccount {
//!     #[hash]
//!     pub owner: Pubkey,
//!     pub counter: u64,
//! }
//! ```
//!
//!
//! ### Create compressed account
//! ```ignore
//! let mut my_compressed_account = LightAccount::<'_, CounterAccount>::new_init(
//!     &crate::ID,
//!     // Address
//!     Some(address),
//!     output_tree_index,
//! );
//! // Set data:
//! my_compressed_account.owner = ctx.accounts.signer.key();
//! ```
//! ### Update compressed account
//! ```ignore
//! let mut my_compressed_account = LightAccount::<'_, CounterAccount>::new_mut(
//!     &crate::ID,
//!     &account_meta,
//!     my_compressed_account,
//! );
//! // Increment counter.
//! my_compressed_account.counter += 1;
//! ```
//! ### Close compressed account
//! ```ignore
//! let mut my_compressed_account = LightAccount::<'_, CounterAccount>::new_close(
//!     &crate::ID,
//!     &account_meta_close,
//!     my_compressed_account,
//! );
//! ```
// TODO: add example for manual hashing

use std::ops::{Deref, DerefMut};

use light_compressed_account::{
    compressed_account::PackedMerkleContext,
    instruction_data::with_account_info::{CompressedAccountInfo, InAccountInfo, OutAccountInfo},
};
use light_sdk_types::instruction::account_meta::CompressedAccountMetaTrait;
use solana_pubkey::Pubkey;

use crate::{
    error::LightSdkError,
    light_hasher::{DataHasher, Poseidon},
    AnchorDeserialize, AnchorSerialize, LightDiscriminator,
};

#[derive(Debug, PartialEq)]
pub struct LightAccount<
    'a,
    A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + DataHasher + Default,
> {
    owner: &'a Pubkey,
    pub account: A,
    account_info: CompressedAccountInfo,
}

impl<'a, A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + DataHasher + Default>
    LightAccount<'a, A>
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
        }
    }

    pub fn new_mut(
        owner: &'a Pubkey,
        input_account_meta: &impl CompressedAccountMetaTrait,
        input_account: A,
    ) -> Result<Self, LightSdkError> {
        let input_account_info = {
            let input_data_hash = input_account.hash::<Poseidon>()?;
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
        })
    }

    pub fn new_close(
        owner: &'a Pubkey,
        input_account_meta: &impl CompressedAccountMetaTrait,
        input_account: A,
    ) -> Result<Self, LightSdkError> {
        let input_account_info = {
            let input_data_hash = input_account.hash::<Poseidon>()?;
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
        })
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

    /// 1. Serializes the account data and sets the output data hash.
    /// 2. Returns CompressedAccountInfo.
    ///
    /// Note this is an expensive operation
    /// that should only be called once per instruction.
    pub fn to_account_info(mut self) -> Result<CompressedAccountInfo, LightSdkError> {
        if let Some(output) = self.account_info.output.as_mut() {
            output.data_hash = self.account.hash::<Poseidon>()?;
            output.data = self
                .account
                .try_to_vec()
                .map_err(|_| LightSdkError::Borsh)?;
        }
        Ok(self.account_info)
    }
}

impl<A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + DataHasher + Default> Deref
    for LightAccount<'_, A>
{
    type Target = A;

    fn deref(&self) -> &Self::Target {
        &self.account
    }
}

impl<A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + DataHasher + Default> DerefMut
    for LightAccount<'_, A>
{
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.account
    }
}

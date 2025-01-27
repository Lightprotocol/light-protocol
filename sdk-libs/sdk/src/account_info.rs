use std::{cell::RefCell, rc::Rc};

use anchor_lang::prelude::Result;
use solana_program::pubkey::Pubkey;

use crate::{
    account_meta::LightAccountMeta,
    address::PackedNewAddressParams,
    compressed_account::{
        CompressedAccount, CompressedAccountData, OutputCompressedAccountWithPackedContext,
        PackedCompressedAccountWithMerkleContext,
    },
    error::LightSdkError,
    merkle_context::PackedMerkleContext,
};

/// Information about compressed account which is being initialized.
#[derive(Debug)]
pub struct LightInputAccountInfo<'a> {
    /// Lamports.
    pub lamports: Option<u64>,
    /// Address.
    pub address: Option<[u8; 32]>,
    /// Account data.
    pub data: Option<&'a [u8]>,
    /// Data hash.
    pub data_hash: Option<[u8; 32]>,
    /// Merkle tree context.
    pub merkle_context: PackedMerkleContext,
    /// Root index.
    pub root_index: u16,
}

/// Information about compressed account which is being mutated.
#[derive(Debug)]
pub struct LightAccountInfo<'a> {
    /// Input account.
    pub(crate) input: Option<LightInputAccountInfo<'a>>,
    /// Owner of the account.
    ///
    /// Defaults to the program ID.
    pub owner: &'a Pubkey,
    /// Lamports.
    pub lamports: Option<u64>,
    /// Discriminator.
    pub discriminator: Option<[u8; 8]>,
    /// Account data.
    pub data: Option<Rc<RefCell<Vec<u8>>>>,
    /// Data hash.
    pub data_hash: Option<[u8; 32]>,
    /// Address.
    pub address: Option<[u8; 32]>,
    /// New Merkle tree index. Set `None` for `close` account infos.
    pub output_merkle_tree_index: Option<u8>,
    /// New address parameters.
    pub new_address_params: Option<PackedNewAddressParams>,
}

impl<'a> LightAccountInfo<'a> {
    pub fn from_meta_init(
        meta: &'a LightAccountMeta,
        discriminator: [u8; 8],
        new_address: [u8; 32],
        new_address_seed: [u8; 32],
        space: Option<usize>,
        owner: &'a Pubkey,
    ) -> Result<Self> {
        let address_merkle_context = meta
            .address_merkle_context
            .as_ref()
            .ok_or(LightSdkError::ExpectedAddressMerkleContext)?;

        let new_address_params = PackedNewAddressParams {
            seed: new_address_seed,
            address_queue_account_index: address_merkle_context.address_queue_pubkey_index,
            address_merkle_tree_account_index: address_merkle_context
                .address_merkle_tree_pubkey_index,
            address_merkle_tree_root_index: meta
                .address_merkle_tree_root_index
                .ok_or(LightSdkError::ExpectedAddressRootIndex)?,
        };

        let data = match space {
            Some(space) => Vec::with_capacity(space),
            None => Vec::new(),
        };
        let data = Some(Rc::new(RefCell::new(data)));

        let account_info = LightAccountInfo {
            input: None,
            owner,
            // Needs to be assigned by the program.
            lamports: None,
            // Needs to be assigned by the program.
            discriminator: Some(discriminator),
            data,
            // Needs to be assigned by the program.
            data_hash: None,
            address: Some(new_address),
            output_merkle_tree_index: meta.output_merkle_tree_index,
            new_address_params: Some(new_address_params),
        };
        Ok(account_info)
    }

    pub fn from_meta_mut(
        meta: &'a LightAccountMeta,
        discriminator: [u8; 8],
        owner: &'a Pubkey,
    ) -> Result<Self> {
        let input = LightInputAccountInfo {
            lamports: meta.lamports,
            address: meta.address,
            data: meta.data.as_deref(),
            // Needs to be assigned by the program.
            data_hash: None,
            merkle_context: meta
                .merkle_context
                .ok_or(LightSdkError::ExpectedMerkleContext)?,
            root_index: meta
                .merkle_tree_root_index
                .ok_or(LightSdkError::ExpectedRootIndex)?,
        };

        let account_info = LightAccountInfo {
            input: Some(input),
            owner,
            // Needs to be assigned by the program.
            lamports: None,
            // Needs to be assigned by the program.
            discriminator: Some(discriminator),
            // NOTE(vadorovsky): A `clone()` here is unavoidable.
            // What we have here is an immutable reference to `LightAccountMeta`,
            // from which we can take an immutable reference to `data`.
            //
            // - That immutable reference can be used in the input account,
            //   since we don't make modifications there.
            // - In the most cases, we intend to make modifications for the
            //   output account. We make a copy, which then we try not to
            //   copy again until the moment of creating a CPI call.
            //
            // The reason why `solana_account_info::AccountInfo` stores data as
            // `Rc<RefCell<&'a mut [u8]>>` is that the reference points to
            // runtime's memory region which provides the accout and is mutable
            // by design.
            //
            // In our case, compressed accounts are part of instruction data.
            // Instruction data is immutable (`&[u8]`). There is no way to
            // mutate instruction data without copy.
            data: meta
                .data
                .as_ref()
                .map(|data| Rc::new(RefCell::new(data.clone()))),
            // Needs to be assigned by the program.
            data_hash: None,
            address: meta.address,
            output_merkle_tree_index: meta.output_merkle_tree_index,
            new_address_params: None,
        };
        Ok(account_info)
    }

    pub fn from_meta_close(
        meta: &'a LightAccountMeta,
        discriminator: [u8; 8],
        owner: &'a Pubkey,
    ) -> Result<Self> {
        let input = LightInputAccountInfo {
            lamports: meta.lamports,
            address: meta.address,
            data: meta.data.as_deref(),
            // Needs to be assigned by the program.
            data_hash: None,
            merkle_context: meta
                .merkle_context
                .ok_or(LightSdkError::ExpectedMerkleContext)?,
            root_index: meta
                .merkle_tree_root_index
                .ok_or(LightSdkError::ExpectedRootIndex)?,
        };

        let account_info = LightAccountInfo {
            input: Some(input),
            owner,
            // Needs to be assigned by the program.
            lamports: None,
            // Needs to be assigned by the program.
            discriminator: Some(discriminator),
            data: None,
            // Needs to be assigned by the program.
            data_hash: None,
            address: meta.address,
            output_merkle_tree_index: None,
            new_address_params: None,
        };
        Ok(account_info)
    }

    pub(crate) fn from_meta_init_without_output_data(
        meta: &'a LightAccountMeta,
        discriminator: [u8; 8],
        new_address: [u8; 32],
        new_address_seed: [u8; 32],
        owner: &'a Pubkey,
    ) -> Result<Self> {
        let address_merkle_context = meta
            .address_merkle_context
            .as_ref()
            .ok_or(LightSdkError::ExpectedAddressMerkleContext)?;

        let new_address_params = PackedNewAddressParams {
            seed: new_address_seed,
            address_queue_account_index: address_merkle_context.address_queue_pubkey_index,
            address_merkle_tree_account_index: address_merkle_context
                .address_merkle_tree_pubkey_index,
            address_merkle_tree_root_index: meta
                .address_merkle_tree_root_index
                .ok_or(LightSdkError::ExpectedAddressRootIndex)?,
        };

        let account_info = LightAccountInfo {
            input: None,
            owner,
            // Needs to be assigned by the program.
            lamports: None,
            // Needs to be assigned by the program.
            discriminator: Some(discriminator),
            data: None,
            data_hash: None,
            address: Some(new_address),
            output_merkle_tree_index: meta.output_merkle_tree_index,
            new_address_params: Some(new_address_params),
        };
        Ok(account_info)
    }

    /// Converts [`LightAcccountMeta`], representing either a `mut` or `close`
    /// account, to a `LightAccountInfo` without output data set.
    ///
    /// Not intended for external use, intended for building upper abstraction
    /// layers which handle data serialization on their own.
    pub(crate) fn from_meta_without_output_data(
        meta: &'a LightAccountMeta,
        discriminator: [u8; 8],
        owner: &'a Pubkey,
    ) -> Result<Self> {
        let input = LightInputAccountInfo {
            lamports: meta.lamports,
            address: meta.address,
            data: meta.data.as_deref(),
            // Needs to be assigned by the program.
            data_hash: None,
            merkle_context: meta
                .merkle_context
                .ok_or(LightSdkError::ExpectedMerkleContext)?,
            root_index: meta
                .merkle_tree_root_index
                .ok_or(LightSdkError::ExpectedRootIndex)?,
        };

        let account_info = LightAccountInfo {
            input: Some(input),
            owner,
            // Needs to be assigned by the program.
            lamports: None,
            discriminator: Some(discriminator),
            // Needs to be assigned by the program.
            data: None,
            data_hash: None,
            address: meta.address,
            output_merkle_tree_index: meta.output_merkle_tree_index,
            new_address_params: None,
        };
        Ok(account_info)
    }

    pub fn compress_and_add_sol(&mut self, lamports: u64) {
        self.lamports = Some(lamports);
    }

    /// Returns the original data sent by the client, before any potential
    /// modifications made by the program.
    pub fn initial_data(&self) -> Option<&[u8]> {
        self.input.as_ref().and_then(|input| input.data)
    }

    /// Converts the given [LightAccountInfo] into a
    /// [PackedCompressedAccountWithMerkleContext] which can be sent to the
    /// light-system program.
    pub fn input_compressed_account(
        &self,
    ) -> Result<Option<PackedCompressedAccountWithMerkleContext>> {
        match self.input.as_ref() {
            Some(input) => {
                let data = match input.data {
                    Some(_) => {
                        let discriminator = self
                            .discriminator
                            .ok_or(LightSdkError::ExpectedDiscriminator)?;
                        let data_hash = input.data_hash.ok_or(LightSdkError::ExpectedHash)?;
                        Some(CompressedAccountData {
                            discriminator,
                            data: Vec::new(),
                            data_hash,
                        })
                    }
                    None => None,
                };
                Ok(Some(PackedCompressedAccountWithMerkleContext {
                    compressed_account: CompressedAccount {
                        owner: *self.owner,
                        lamports: input.lamports.unwrap_or(0),
                        address: input.address,
                        hash: None,
                        data,
                    },
                    merkle_context: input.merkle_context,
                    root_index: input.root_index,
                    read_only: false,
                }))
            }
            None => Ok(None),
        }
    }

    pub fn output_compressed_account(
        &self,
    ) -> Result<Option<OutputCompressedAccountWithPackedContext>> {
        match self.output_merkle_tree_index {
            Some(merkle_tree_index) => {
                let data = match self.data {
                    Some(_) => {
                        let discriminator = self
                            .discriminator
                            .ok_or(LightSdkError::ExpectedDiscriminator)?;
                        let data_hash = self.data_hash.ok_or(LightSdkError::ExpectedHash)?;
                        Some(CompressedAccountData {
                            discriminator,
                            data: Vec::new(),
                            data_hash,
                        })
                    }
                    None => None,
                };
                Ok(Some(OutputCompressedAccountWithPackedContext {
                    compressed_account: CompressedAccount {
                        owner: *self.owner,
                        lamports: self.lamports.unwrap_or(0),
                        address: self.address,
                        hash: None,
                        data,
                    },
                    merkle_tree_index,
                }))
            }
            None => Ok(None),
        }
    }
}

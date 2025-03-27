use std::{cell::RefCell, rc::Rc};

use light_compressed_account::{
    compressed_account::{
        CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
        PackedMerkleContext,
    },
    instruction_data::data::OutputCompressedAccountWithPackedContext,
};
use solana_program::pubkey::Pubkey;

use crate::{
    account_meta::{
        LightAccountMeta, ZInputAccountMeta, ZInputAccountMetaNoLamports,
        ZInputAccountMetaWithAddress, ZInputAccountMetaWithAddressNoLamports,
    },
    error::{LightSdkError, Result},
};

/// TODO: consider to create InputMerkleContext that includes root_index that is not optional or it is optional and we remove proof by index -> for zero copy its better to not use options
/// TODO: rename to input metadata
/// Input compressed account state that is being invalidated.
/// Account Meta should give us all info
/// except for data hash we need to do that onchain.
#[derive(Debug)]
pub struct LightInputAccountInfo {
    /// Data hash
    pub data_hash: [u8; 32],
    /// Merkle tree context.
    pub merkle_context: PackedMerkleContext,
    /// Lamports.
    pub lamports: Option<u64>,
    /// Address.
    pub address: Option<[u8; 32]>,
    /// Root index.
    pub root_index: Option<u16>,
}

impl LightInputAccountInfo {
    pub fn from_input_account_meta(meta: &ZInputAccountMeta, data_hash: [u8; 32]) -> Result<Self> {
        Ok(Self {
            lamports: Some((*meta.lamports).into()),
            address: None,
            data_hash,
            merkle_context: (&meta.merkle_context).into(),
            root_index: meta.root_index.map(|x| (*x).into()),
        })
    }

    pub fn from_input_account_meta_no_lamports(
        meta: &ZInputAccountMetaNoLamports,
        data_hash: [u8; 32],
    ) -> Result<Self> {
        Ok(Self {
            lamports: None,
            address: None,
            data_hash,
            merkle_context: (&meta.merkle_context).into(),
            root_index: meta.root_index.map(|x| (*x).into()),
        })
    }

    pub fn from_input_account_meta_with_address(
        meta: &ZInputAccountMetaWithAddress,
        data_hash: [u8; 32],
    ) -> Result<Self> {
        Ok(Self {
            lamports: Some((*meta.lamports).into()),
            address: Some(*meta.address),
            data_hash,
            root_index: meta.root_index.map(|x| (*x).into()),
            merkle_context: (&meta.merkle_context).into(),
        })
    }

    pub fn from_input_account_meta_with_address_no_lamports(
        meta: &ZInputAccountMetaWithAddressNoLamports,
        data_hash: [u8; 32],
    ) -> Result<Self> {
        Ok(Self {
            lamports: None,
            address: Some(*meta.address),
            data_hash,
            root_index: meta.root_index.map(|x| (*x).into()),
            merkle_context: (&meta.merkle_context).into(),
        })
    }
}

// TODO: consider to create LightOutputAccountInfo and wrap it in LightAccountInfo

/// Information about compressed account which is being mutated.
#[derive(Debug)]
pub struct LightAccountInfo<'a> {
    /// Input account.
    pub(crate) input: Option<LightInputAccountInfo>,
    /// Owner of the account.
    ///
    /// Defaults to the program ID.
    pub owner: &'a Pubkey,
    /// Lamports.
    pub lamports: Option<u64>,
    /// Discriminator. TODO: why option?
    pub discriminator: Option<[u8; 8]>,
    /// Account data.
    pub data: Option<Rc<RefCell<Vec<u8>>>>,
    /// Data hash.
    pub data_hash: Option<[u8; 32]>,
    /// Address.
    pub address: Option<[u8; 32]>,
    /// New Merkle tree index. Set `None` for `close` account infos.
    pub output_merkle_tree_index: Option<u8>,
}

impl<'a> LightAccountInfo<'a> {
    pub fn init_with_address(
        owner: &'a Pubkey,
        discriminator: [u8; 8],
        data: Vec<u8>,
        data_hash: [u8; 32],
        address: [u8; 32],
        output_merkle_tree_index: u8,
    ) -> Self {
        Self {
            input: None,
            owner,
            lamports: None,
            discriminator: Some(discriminator),
            data: Some(Rc::new(RefCell::new(data))),
            data_hash: Some(data_hash),
            address: Some(address),
            output_merkle_tree_index: Some(output_merkle_tree_index),
        }
    }

    pub fn from_meta_mut(
        input: LightInputAccountInfo,
        owner: &'a Pubkey,
        data: Vec<u8>,
        discriminator: [u8; 8],
        output_merkle_tree_index: u8,
    ) -> Result<Self> {
        let account_info = LightAccountInfo {
            owner,
            // Needs to be assigned by the program.
            lamports: input.lamports,
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
            data: Some(Rc::new(RefCell::new(data))),
            // Needs to be assigned by the program.
            data_hash: None,
            address: input.address,
            output_merkle_tree_index: Some(output_merkle_tree_index),
            input: Some(input),
        };
        Ok(account_info)
    }

    pub fn from_meta_close(
        meta: &'a LightAccountMeta,
        discriminator: [u8; 8],
        owner: &'a Pubkey,
        data_hash: [u8; 32],
    ) -> Result<Self> {
        let input = LightInputAccountInfo {
            lamports: meta.lamports,
            address: meta.address,
            // data: meta.data.as_deref(),
            // Needs to be assigned by the program.
            data_hash,
            merkle_context: meta.merkle_context.unwrap(),
            root_index: meta.merkle_tree_root_index,
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
        };
        Ok(account_info)
    }

    pub fn compress_and_add_sol(&mut self, lamports: u64) {
        self.lamports = Some(lamports);
    }

    // /// Returns the original data sent by the client, before any potential
    // /// modifications made by the program.
    // pub fn initial_data(&self) -> Option<&[u8]> {
    //     self.input.as_ref().and_then(|input| input.data)
    // }

    /// Converts the given [LightAccountInfo] into a
    /// [PackedCompressedAccountWithMerkleContext] which can be sent to the
    /// light-system program.
    pub fn input_compressed_account(
        &self,
    ) -> Result<Option<PackedCompressedAccountWithMerkleContext>> {
        match self.input.as_ref() {
            Some(input) => {
                let data = {
                    let discriminator = self
                        .discriminator
                        .ok_or(LightSdkError::ExpectedDiscriminator)?;
                    Some(CompressedAccountData {
                        discriminator,
                        data: Vec::new(),
                        data_hash: input.data_hash,
                    })
                };
                Ok(Some(PackedCompressedAccountWithMerkleContext {
                    compressed_account: CompressedAccount {
                        owner: *self.owner,
                        lamports: input.lamports.unwrap_or(0),
                        address: input.address,
                        data,
                    },
                    merkle_context: input.merkle_context,
                    root_index: input.root_index.unwrap_or_default(),
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
                        data,
                    },
                    merkle_tree_index,
                }))
            }
            None => Ok(None),
        }
    }
}

use std::{cell::RefCell, rc::Rc};

use anchor_lang::prelude::Result;
use solana_program::{account_info::AccountInfo, pubkey::Pubkey};

use crate::{
    account_meta::LightAccountMeta,
    address::{
        derive_address_from_params, derive_address_seed, unpack_new_address_params,
        PackedNewAddressParams,
    },
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
    pub owner: Option<Pubkey>,
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
    /// New Merkle tree index. Set only if you want to change the tree.
    pub output_merkle_tree_index: Option<u8>,
    /// New address parameters.
    pub new_address: Option<PackedNewAddressParams>,
}

impl<'a> LightAccountInfo<'a> {
    pub fn from_meta(meta: &'a LightAccountMeta, program_id: &Pubkey) -> Result<Self> {
        let input = match meta.merkle_context {
            Some(merkle_context) => Some(LightInputAccountInfo {
                lamports: meta.lamports,
                address: meta.address,
                data: meta.data.as_deref(),
                merkle_context,
                root_index: meta
                    .merkle_tree_root_index
                    .ok_or(LightSdkError::ExpectedRootIndex)?,
            }),
            None => None,
        };
        let account_info = LightAccountInfo {
            input,
            owner: Some(*program_id),
            // Needs to be assigned by the program.
            lamports: None,
            // Needs to be assigned by the program.
            discriminator: None,
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
            new_address: match meta.address_merkle_context {
                Some(address_merkle_tree_meta) => {
                    Some(PackedNewAddressParams {
                        // Seed has to be overwritten later.
                        seed: [0u8; 32],
                        address_merkle_tree_account_index: address_merkle_tree_meta
                            .address_merkle_tree_pubkey_index,
                        address_queue_account_index: address_merkle_tree_meta
                            .address_queue_pubkey_index,
                        address_merkle_tree_root_index: meta
                            .address_merkle_tree_root_index
                            .ok_or(LightSdkError::ExpectedAddressRootIndex)?,
                    })
                }
                None => None,
            },
        };
        Ok(account_info)
    }

    pub fn compress_and_add_sol(&mut self, lamports: u64) {
        self.lamports = Some(lamports);
    }

    pub fn derive_address(
        &mut self,
        seeds: &[&[u8]],
        program_id: &Pubkey,
        remaining_accounts: &[AccountInfo],
    ) -> Result<()> {
        match self.new_address {
            Some(ref mut params) => {
                anchor_lang::prelude::msg!("program_id: {:?}", program_id);
                params.seed = derive_address_seed(seeds, program_id);
                let unpacked_params = unpack_new_address_params(params, remaining_accounts);

                anchor_lang::prelude::msg!("params: {:?}", unpacked_params);
                self.address = Some(derive_address_from_params(unpacked_params));
                Ok(())
            }
            None => Err(LightSdkError::ExpectedAddressParams.into()),
        }
    }

    pub fn set_discriminator(&mut self, discriminator: [u8; 8]) {
        self.discriminator = Some(discriminator);
    }

    /// Converts the given [LightAccountInfo] into a
    /// [PackedCompressedAccountWithMerkleContext] which can be sent to the
    /// light-system program.
    pub fn input_compressed_account(
        &self,
        program_id: &Pubkey,
    ) -> Result<Option<PackedCompressedAccountWithMerkleContext>> {
        match self.input.as_ref() {
            Some(input) => {
                let data = match input.data {
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
                Ok(Some(PackedCompressedAccountWithMerkleContext {
                    compressed_account: CompressedAccount {
                        owner: *program_id,
                        lamports: input.lamports.unwrap_or(0),
                        address: input.address,
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
        program_id: &Pubkey,
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
                        owner: self.owner.unwrap_or(*program_id),
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

pub fn convert_metas_to_infos<'a, 'b>(
    metas: &'a Option<Vec<LightAccountMeta>>,
    program_id: &'b Pubkey,
) -> Result<Vec<LightAccountInfo<'a>>>
where
    'a: 'b,
{
    match metas {
        Some(metas) => {
            let mut infos = Vec::with_capacity(metas.len());
            for meta in metas {
                let info = LightAccountInfo::from_meta(meta, program_id)?;
                infos.push(info);
            }
            Ok(infos)
        }
        None => Ok(Vec::new()),
    }
}

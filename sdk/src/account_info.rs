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
    pub new_address_params: Option<PackedNewAddressParams>,
}

impl<'a> LightAccountInfo<'a> {
    pub fn from_meta(
        meta: &'a LightAccountMeta,
        discriminator: Option<[u8; 8]>,
        new_address: Option<[u8; 32]>,
        new_address_seed: Option<[u8; 32]>,
        program_id: &Pubkey,
    ) -> Result<Self> {
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

        // `new_address` and `new_address_seeds` are co-dependent. And when
        // they're defined, they need `meta.address_merkle_context` to be
        // defined.
        // We can't create an address without knowing in which Merkle tree.
        //
        // When all of them are defined, we request a creation of a new
        // address.
        // When none of them is defined, we don't do that.
        // When only a subset of them is defined, we raise an error.
        let (address, new_address_params) =
            match (new_address, new_address_seed, meta.address_merkle_context) {
                (Some(address), Some(seed), Some(address_merkle_context)) => {
                    let new_address = PackedNewAddressParams {
                        seed,
                        address_merkle_tree_account_index: address_merkle_context
                            .address_merkle_tree_pubkey_index,
                        address_queue_account_index: address_merkle_context
                            .address_queue_pubkey_index,
                        address_merkle_tree_root_index: meta
                            .address_merkle_tree_root_index
                            .ok_or(LightSdkError::ExpectedAddressRootIndex)?,
                    };
                    (Some(address), Some(new_address))
                }
                (None, None, None) => (None, None),
                // If no seeds are provided and there is no address Merkle context,
                // don't do anything.
                (None, None, Some(_)) => (None, None),
                // Otherwise, throw an error.
                _ => return Err(LightSdkError::ExpectedAddressParams.into()),
            };

        let address = match address {
            Some(address) => Some(address),
            // If we didn't derive a new address, just take the one which was
            // submitted by the client.
            None => meta.address,
        };

        let account_info = LightAccountInfo {
            input,
            owner: Some(*program_id),
            // Needs to be assigned by the program.
            lamports: None,
            // Needs to be assigned by the program.
            discriminator,
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
            address,
            output_merkle_tree_index: meta.output_merkle_tree_index,
            new_address_params,
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
        match self.new_address_params {
            Some(ref mut params) => {
                params.seed = derive_address_seed(seeds, program_id);
                let unpacked_params = unpack_new_address_params(params, remaining_accounts);

                self.address = Some(derive_address_from_params(unpacked_params));
                Ok(())
            }
            None => Err(LightSdkError::ExpectedAddressParams.into()),
        }
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

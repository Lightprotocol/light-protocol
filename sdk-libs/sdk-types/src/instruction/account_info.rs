#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

#[cfg(feature = "alloc")]
use light_compressed_account::compressed_account::PackedCompressedAccountWithMerkleContext;
use light_compressed_account::{
    compressed_account::PackedMerkleContext,
    instruction_data::with_account_info::{CompressedAccountInfo, InAccountInfo},
};
#[cfg(feature = "alloc")]
use light_compressed_account::{
    compressed_account::{CompressedAccount, CompressedAccountData},
    instruction_data::data::OutputCompressedAccountWithPackedContext,
    Pubkey,
};

use super::account_meta::CompressedAccountMetaTrait;
use crate::error::LightSdkTypesError;

pub trait InAccountInfoTrait {
    fn input_meta<T: CompressedAccountMetaTrait>(
        &mut self,
        meta: &T,
        data_hash: [u8; 32],
        discriminator: [u8; 8],
    );
}

impl InAccountInfoTrait for InAccountInfo {
    fn input_meta<T: CompressedAccountMetaTrait>(
        &mut self,
        meta: &T,
        data_hash: [u8; 32],
        discriminator: [u8; 8],
    ) {
        if let Some(input_lamports) = meta.get_lamports() {
            self.lamports = input_lamports;
        }
        self.data_hash = data_hash;
        self.discriminator = discriminator;
        if let Some(root_index) = meta.get_root_index().as_ref() {
            self.root_index = *root_index;
        }
        let tree_info = meta.get_tree_info();
        self.merkle_context = PackedMerkleContext {
            merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
            queue_pubkey_index: tree_info.queue_pubkey_index,
            leaf_index: tree_info.leaf_index,
            prove_by_index: tree_info.prove_by_index,
        };
    }
}

pub trait CompressedAccountInfoTrait {
    fn init(
        &mut self,
        discriminator: [u8; 8],
        address: Option<[u8; 32]>,
        output_state_tree_index: u8,
    ) -> Result<(), LightSdkTypesError>;

    fn meta_mut<M: CompressedAccountMetaTrait>(
        &mut self,
        input_account_meta: &M,
        input_data_hash: [u8; 32],
        discriminator: [u8; 8],
        output_state_tree_index: u8,
    ) -> Result<(), LightSdkTypesError>;

    fn meta_close<M: CompressedAccountMetaTrait>(
        &mut self,
        input_account_meta: &M,
        input_data_hash: [u8; 32],
        discriminator: [u8; 8],
    ) -> Result<(), LightSdkTypesError>;
    #[cfg(feature = "alloc")]
    fn input_compressed_account(
        &self,
        owner: Pubkey,
    ) -> Result<Option<PackedCompressedAccountWithMerkleContext>, LightSdkTypesError>;
    #[cfg(feature = "alloc")]
    fn output_compressed_account(
        &self,
        owner: Pubkey,
    ) -> Result<Option<OutputCompressedAccountWithPackedContext>, LightSdkTypesError>;
}

impl CompressedAccountInfoTrait for CompressedAccountInfo {
    /// Initializes a compressed account info with address.
    /// 1. The account is zeroed, data has to be added in a separate step.
    /// 2. Once data is added the data hash has to be added.
    fn init(
        &mut self,
        discriminator: [u8; 8],
        address: Option<[u8; 32]>,
        output_state_tree_index: u8,
    ) -> Result<(), LightSdkTypesError> {
        if let Some(self_address) = self.address.as_mut() {
            if let Some(address) = address {
                self_address.copy_from_slice(&address);
            } else {
                return Err(LightSdkTypesError::InitAddressIsNone);
            }
        } else {
            return Err(LightSdkTypesError::InitWithAddressIsNone);
        }
        if let Some(output) = self.output.as_mut() {
            output.output_merkle_tree_index = output_state_tree_index;
            output.discriminator = discriminator;
        } else {
            return Err(LightSdkTypesError::InitWithAddressOutputIsNone);
        }
        Ok(())
    }

    fn meta_mut<M: CompressedAccountMetaTrait>(
        &mut self,
        input_account_meta: &M,
        input_data_hash: [u8; 32],
        discriminator: [u8; 8],
        output_state_tree_index: u8,
    ) -> Result<(), LightSdkTypesError> {
        if let Some(self_address) = self.address.as_mut() {
            if let Some(address) = input_account_meta.get_address().as_ref() {
                *self_address = *address;
            } else {
                return Err(LightSdkTypesError::MetaMutAddressIsNone);
            }
        } else {
            return Err(LightSdkTypesError::MetaMutAddressIsNone);
        }

        if let Some(input) = self.input.as_mut() {
            input.input_meta(input_account_meta, input_data_hash, discriminator);
        } else {
            return Err(LightSdkTypesError::MetaMutInputIsNone);
        }

        if let Some(output) = self.output.as_mut() {
            output.output_merkle_tree_index = output_state_tree_index;
            output.discriminator = discriminator;

            if let Some(input_lamports) = input_account_meta.get_lamports() {
                output.lamports = input_lamports;
            } else {
                return Err(LightSdkTypesError::MetaMutOutputLamportsIsNone);
            }
        } else {
            return Err(LightSdkTypesError::MetaMutOutputIsNone);
        }
        Ok(())
    }

    fn meta_close<M: CompressedAccountMetaTrait>(
        &mut self,
        input_account_meta: &M,
        input_data_hash: [u8; 32],
        discriminator: [u8; 8],
    ) -> Result<(), LightSdkTypesError> {
        if let Some(self_address) = self.address.as_mut() {
            if let Some(address) = input_account_meta.get_address() {
                self_address.copy_from_slice(&address);
            } else {
                return Err(LightSdkTypesError::MetaCloseAddressIsNone);
            }
        } else {
            return Err(LightSdkTypesError::MetaCloseAddressIsNone);
        }

        if let Some(input) = self.input.as_mut() {
            input.input_meta(input_account_meta, input_data_hash, discriminator);
        } else {
            return Err(LightSdkTypesError::MetaCloseInputIsNone);
        }

        Ok(())
    }

    #[cfg(feature = "alloc")]
    fn input_compressed_account(
        &self,
        owner: Pubkey,
    ) -> Result<Option<PackedCompressedAccountWithMerkleContext>, LightSdkTypesError> {
        match self.input.as_ref() {
            Some(input) => {
                let data = Some(CompressedAccountData {
                    discriminator: input.discriminator,
                    data: Vec::new(),
                    data_hash: input.data_hash,
                });
                Ok(Some(PackedCompressedAccountWithMerkleContext {
                    compressed_account: CompressedAccount {
                        owner: owner.to_bytes().into(),
                        lamports: input.lamports,
                        address: self.address,
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

    #[cfg(feature = "alloc")]
    fn output_compressed_account(
        &self,
        owner: Pubkey,
    ) -> Result<Option<OutputCompressedAccountWithPackedContext>, LightSdkTypesError> {
        match self.output.as_ref() {
            Some(output) => {
                let data = Some(CompressedAccountData {
                    discriminator: output.discriminator,
                    data: output.data.clone(),
                    data_hash: output.data_hash,
                });
                Ok(Some(OutputCompressedAccountWithPackedContext {
                    compressed_account: CompressedAccount {
                        owner: owner.to_bytes().into(),
                        lamports: output.lamports,
                        address: self.address,
                        data,
                    },
                    merkle_tree_index: output.output_merkle_tree_index,
                }))
            }
            None => Ok(None),
        }
    }
}

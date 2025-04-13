use light_compressed_account::{
    compressed_account::{
        CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
    },
    instruction_data::{
        data::OutputCompressedAccountWithPackedContext,
        with_account_info::{CompressedAccountInfo, InAccountInfo},
    },
    CompressedAccountError,
};

use crate::{error::LightSdkError, instruction::account_meta::CompressedAccountMetaTrait, msg};

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
        self.merkle_context = *meta.get_merkle_context();
    }
}

pub trait AccountInfoTrait {
    fn init(
        &mut self,
        discriminator: [u8; 8],
        address: Option<[u8; 32]>,
        output_merkle_tree_index: u8,
    ) -> Result<(), CompressedAccountError>;

    fn meta_mut<M: CompressedAccountMetaTrait>(
        &mut self,
        // Input
        input_account_meta: &M,
        input_data_hash: [u8; 32],
        discriminator: [u8; 8],
        output_merkle_tree_index: u8,
    ) -> Result<(), CompressedAccountError>;

    fn meta_close<M: CompressedAccountMetaTrait>(
        &mut self,
        input_account_meta: &M,
        input_data_hash: [u8; 32],
        discriminator: [u8; 8],
    ) -> Result<(), CompressedAccountError>;
    fn input_compressed_account(
        &self,
        owner: crate::Pubkey,
    ) -> Result<Option<PackedCompressedAccountWithMerkleContext>, LightSdkError>;
    fn output_compressed_account(
        &self,
        owner: crate::Pubkey,
    ) -> Result<Option<OutputCompressedAccountWithPackedContext>, LightSdkError>;
}

impl AccountInfoTrait for CompressedAccountInfo {
    /// Initializes a compressed account info with address.
    /// 1. The account is zeroed, data has to be added in a separate step.
    /// 2. Once data is added the data hash has to be added.
    fn init(
        &mut self,
        discriminator: [u8; 8],
        address: Option<[u8; 32]>,
        output_merkle_tree_index: u8,
    ) -> Result<(), CompressedAccountError> {
        if let Some(self_address) = self.address.as_mut() {
            if let Some(address) = address {
                self_address.copy_from_slice(&address);
            } else {
                msg!("init: address is none");
                return Err(CompressedAccountError::InvalidAccountSize);
            }
        } else {
            msg!("init_with_address: address is none");
            return Err(CompressedAccountError::InvalidAccountSize);
        }
        if let Some(output) = self.output.as_mut() {
            output.output_merkle_tree_index = output_merkle_tree_index;
            output.discriminator = discriminator;
        } else {
            msg!("init_with_address: output is none");
            return Err(CompressedAccountError::InvalidAccountSize);
        }
        Ok(())
    }

    /// Initializes a compressed account info with address.
    /// 1. The account is zeroed, data has to be added in a separate step.
    /// 2. Once data is added the data hash has to be added.
    fn meta_mut<M: CompressedAccountMetaTrait>(
        &mut self,
        // Input
        input_account_meta: &M,
        input_data_hash: [u8; 32],
        discriminator: [u8; 8],
        output_merkle_tree_index: u8,
    ) -> Result<(), CompressedAccountError> {
        if let Some(self_address) = self.address.as_mut() {
            if let Some(address) = input_account_meta.get_address().as_ref() {
                *self_address = *address;
            } else {
                msg!("from_z_meta_mut: address is none");
                return Err(CompressedAccountError::InvalidAccountSize);
            }
        } else {
            msg!("from_z_meta_mut: address is none");
            return Err(CompressedAccountError::InvalidAccountSize);
        }

        if let Some(input) = self.input.as_mut() {
            input.input_meta(input_account_meta, input_data_hash, discriminator);
        } else {
            msg!("from_z_meta_mut: input is none");
            return Err(CompressedAccountError::InvalidAccountSize);
        }

        if let Some(output) = self.output.as_mut() {
            output.output_merkle_tree_index = output_merkle_tree_index;
            output.discriminator = discriminator;

            if let Some(input_lamports) = input_account_meta.get_lamports() {
                output.lamports = input_lamports;
            } else {
                msg!("from_z_meta_mut: output lamports is none");
                return Err(CompressedAccountError::InvalidAccountSize);
            }
        } else {
            msg!("from_z_meta_mut: output is none");
            return Err(CompressedAccountError::InvalidAccountSize);
        }
        Ok(())
    }

    /// Initializes a compressed account info with address.
    /// 1. The account is zeroed, data has to be added in a separate step.
    /// 2. Once data is added the data hash has to be added.
    fn meta_close<M: CompressedAccountMetaTrait>(
        &mut self,
        input_account_meta: &M,
        input_data_hash: [u8; 32],
        discriminator: [u8; 8],
    ) -> Result<(), CompressedAccountError> {
        if let Some(self_address) = self.address.as_mut() {
            if let Some(address) = input_account_meta.get_address() {
                self_address.copy_from_slice(&address);
            } else {
                msg!("from_z_meta_mut: address is none");
                return Err(CompressedAccountError::InvalidAccountSize);
            }
        } else {
            msg!("from_z_meta_mut: address is none");
            return Err(CompressedAccountError::InvalidAccountSize);
        }

        if let Some(input) = self.input.as_mut() {
            input.input_meta(input_account_meta, input_data_hash, discriminator);
        } else {
            msg!("from_z_meta_mut: input is none");
            return Err(CompressedAccountError::InvalidAccountSize);
        }

        Ok(())
    }

    fn input_compressed_account(
        &self,
        owner: crate::Pubkey,
    ) -> Result<Option<PackedCompressedAccountWithMerkleContext>, LightSdkError> {
        match self.input.as_ref() {
            Some(input) => {
                let data = Some(CompressedAccountData {
                    discriminator: input.discriminator,
                    data: Vec::new(),
                    data_hash: input.data_hash,
                });
                Ok(Some(PackedCompressedAccountWithMerkleContext {
                    compressed_account: CompressedAccount {
                        owner,
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

    fn output_compressed_account(
        &self,
        owner: crate::Pubkey,
    ) -> Result<Option<OutputCompressedAccountWithPackedContext>, LightSdkError> {
        match self.output.as_ref() {
            Some(output) => {
                let data = Some(CompressedAccountData {
                    discriminator: output.discriminator,
                    data: output.data.clone(),
                    data_hash: output.data_hash,
                });
                Ok(Some(OutputCompressedAccountWithPackedContext {
                    compressed_account: CompressedAccount {
                        owner,
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

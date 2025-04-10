use light_compressed_account::{
    compressed_account::{
        CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
        PackedMerkleContext, PackedReadOnlyCompressedAccount,
    },
    instruction_data::{
        compressed_proof::CompressedProof,
        cpi_context::CompressedCpiContext,
        data::{
            NewAddressParamsPacked, OutputCompressedAccountWithPackedContext, PackedReadOnlyAddress,
        },
    },
    pubkey::Pubkey,
    CompressedAccountError,
};

use crate::{
    error::LightSdkError, instruction::account_meta::CompressedAccountMetaTrait, msg,
    AnchorDeserialize, AnchorSerialize,
};

#[derive(Debug, AnchorSerialize, AnchorDeserialize)]
pub struct SystemInfoInstructionData {
    pub bump: u8,
    pub invoking_program_id: Pubkey,
    pub is_compress: bool,
    pub compress_or_decompress_lamports: u64,
    pub cpi_context: CompressedCpiContext,
    pub proof: Option<CompressedProof>,
    pub new_addresses: Vec<NewAddressParamsPacked>,
    pub read_only_accounts: Vec<PackedReadOnlyCompressedAccount>,
    pub read_only_addresses: Vec<PackedReadOnlyAddress>,
    pub light_account_infos: Vec<CompressedAccountInfo>,
}

#[derive(Debug, Default, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct InAccountInfo {
    /// Data hash
    pub data_hash: [u8; 32],
    /// Merkle tree context.
    pub merkle_context: PackedMerkleContext,
    /// Root index.
    pub root_index: u16,
    /// Lamports.
    pub lamports: u64,
}

#[derive(Debug, Default, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct OutAccountInfo {
    /// Data hash
    pub data_hash: [u8; 32],
    pub output_merkle_tree_index: u8,
    /// Lamports.
    pub lamports: u64,
    /// Account data.
    pub data: Vec<u8>,
}

#[derive(Debug, Default, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedAccountInfo {
    // TODO: optimize parsing by manually implementing ZeroCopy and using the bitmask.
    // bitmask: u8,
    pub discriminator: [u8; 8], // 1
    /// Address.
    pub address: Option<[u8; 32]>, // 2
    /// Input account.
    pub input: Option<InAccountInfo>, // 3
    /// Output account.
    pub output: Option<OutAccountInfo>, // 5
}

impl InAccountInfo {
    pub fn from_input_meta<T: CompressedAccountMetaTrait>(
        &mut self,
        meta: &T,
        data_hash: [u8; 32],
    ) {
        if let Some(input_lamports) = meta.get_lamports() {
            self.lamports = input_lamports;
        }
        self.data_hash = data_hash;
        if let Some(root_index) = meta.get_root_index().as_ref() {
            self.root_index = *root_index;
        }
        self.merkle_context = *meta.get_merkle_context();
    }
}

impl CompressedAccountInfo {
    /// Initializes a compressed account info with address.
    /// 1. The account is zeroed, data has to be added in a separate step.
    /// 2. Once data is added the data hash has to be added.
    pub fn init(
        &mut self,
        discriminator: [u8; 8],
        address: Option<[u8; 32]>,
        output_merkle_tree_index: u8,
    ) -> Result<(), CompressedAccountError> {
        self.discriminator = discriminator;
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
        } else {
            msg!("init_with_address: output is none");
            return Err(CompressedAccountError::InvalidAccountSize);
        }
        Ok(())
    }

    /// Initializes a compressed account info with address.
    /// 1. The account is zeroed, data has to be added in a separate step.
    /// 2. Once data is added the data hash has to be added.
    pub fn from_meta_mut<M: CompressedAccountMetaTrait>(
        &mut self,
        // Input
        input_account_meta: &M,
        input_data_hash: [u8; 32],
        discriminator: [u8; 8],
        output_merkle_tree_index: u8,
    ) -> Result<(), CompressedAccountError> {
        if self.discriminator != [0; 8] {
            msg!("from_z_meta_mut: discriminator is not zeroed. Account already loaded.");
            return Err(CompressedAccountError::InvalidAccountSize);
        }
        self.discriminator = discriminator;

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
            input.from_input_meta(input_account_meta, input_data_hash);
        } else {
            msg!("from_z_meta_mut: input is none");
            return Err(CompressedAccountError::InvalidAccountSize);
        }

        if let Some(output) = self.output.as_mut() {
            output.output_merkle_tree_index = output_merkle_tree_index;

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
    pub fn from_meta_close<M: CompressedAccountMetaTrait>(
        &mut self,
        input_account_meta: &M,
        input_data_hash: [u8; 32],
        discriminator: [u8; 8],
    ) -> Result<(), CompressedAccountError> {
        self.discriminator = discriminator;

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
            input.from_input_meta(input_account_meta, input_data_hash);
        } else {
            msg!("from_z_meta_mut: input is none");
            return Err(CompressedAccountError::InvalidAccountSize);
        }

        Ok(())
    }

    pub(crate) fn input_compressed_account(
        &self,
        owner: crate::Pubkey,
    ) -> Result<Option<PackedCompressedAccountWithMerkleContext>, LightSdkError> {
        match self.input.as_ref() {
            Some(input) => {
                let data = Some(CompressedAccountData {
                    discriminator: self.discriminator,
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

    pub fn output_compressed_account(
        &self,
        owner: crate::Pubkey,
    ) -> Result<Option<OutputCompressedAccountWithPackedContext>, LightSdkError> {
        match self.output.as_ref() {
            Some(output) => {
                let data = Some(CompressedAccountData {
                    discriminator: self.discriminator,
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

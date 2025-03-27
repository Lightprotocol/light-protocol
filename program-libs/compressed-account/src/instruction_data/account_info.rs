use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::ZeroCopy;
use zerocopy::{IntoBytes, U16};

use crate::{
    compressed_account::{PackedMerkleContext, PackedReadOnlyCompressedAccount},
    pubkey::Pubkey,
    CompressedAccountError,
};
use std::mem::size_of;

use super::{
    compressed_proof::CompressedProof,
    data::{NewAddressParamsPacked, PackedReadOnlyAddress},
    meta::{InputAccountMetaTrait, ZInputAccountMetaTrait},
};

/// New method must include instruction discriminator.
#[derive(Debug, ZeroCopy, BorshSerialize, BorshDeserialize)]
pub struct SystemInfoInstructionData {
    pub bump: u8,
    pub invoking_program_id: Pubkey,
    pub proof: Option<CompressedProof>,
    pub new_addresses: Vec<NewAddressParamsPacked>,
    pub read_only_accounts: Vec<PackedReadOnlyCompressedAccount>,
    pub read_only_addresses: Vec<PackedReadOnlyAddress>,
    pub light_account_infos: Vec<CAccountInfo>,
}

pub struct Config {
    pub space: usize,
    pub has_address: bool,
}

pub enum CAccountConfig {
    Init(Config),
    Mut(Config),
    Close(Config),
}

impl SystemInfoInstructionData {
    // TODO: unit test
    fn bytes_required_for_capacity(
        proof: bool,
        new_addresses: usize,
        read_only_accounts: usize,
        read_only_addresses: usize,
        light_account_infos: usize,
    ) -> usize {
        let proof_bytes = if proof { 256 } else { 0 };
        1 // bump
        + 32 // invoking_program_id
            + 1 // option proof
            + proof_bytes
            + size_of::<NewAddressParamsPacked>() * new_addresses
            + size_of::<PackedReadOnlyCompressedAccount>() * read_only_accounts
            + size_of::<PackedReadOnlyAddress>() * read_only_addresses
            + light_account_infos
    }

    // pub fn new<'a>(
    //     proof: bool,
    //     new_addresses: usize,
    //     read_only_accounts: usize,
    //     read_only_addresses: usize,
    //     light_account_infos: Vec<CAccountConfig>,
    //     bytes: &mut [u8],
    // ) -> ZSystemInfoInstructionDataMut<'a> {
    //     let min_size = Self::bytes_required_for_capacity(
    //         proof,
    //         new_addresses,
    //         read_only_accounts,
    //         read_only_addresses,
    //         light_account_infos
    //             .iter()
    //             .map(|x| match x {
    //                 CAccountConfig::Init(config) => config.space,
    //                 CAccountConfig::Mut(config) => config.space,
    //                 CAccountConfig::Close(config) => config.space,
    //             })
    //             .sum(),
    //     );
    // }
}

/// Zero copy casts:
/// 1 + 2 option -> 3
///
/// With Bitmask:
/// - 1 cast completely with lamports or without
#[derive(Debug, ZeroCopy, Default, BorshSerialize, BorshDeserialize)]
pub struct CInAccountInfo {
    /// Data hash
    pub data_hash: [u8; 32],
    /// Merkle tree context.
    pub merkle_context: PackedMerkleContext,
    /// Root index.
    pub root_index: u16,
    /// Lamports.
    pub lamports: u64,
}

/// Zero copy casts:
/// 1 + 2 + 2 -> 5
///
/// With Bitmask:
/// 1 + 2 -> 3
#[derive(Debug, ZeroCopy, Default, BorshSerialize, BorshDeserialize)]
pub struct COutAccountInfo {
    /// Data hash
    pub data_hash: [u8; 32],
    pub output_merkle_tree_index: u8,
    /// Lamports.
    pub lamports: u64,
    /// Account data.
    pub data: Vec<u8>,
}

/// Zero copy casts: 11
/// With Option bitmask: 1 (bits) + 1(meta) + 1(input) + 3 (output) = 6
/// Max bytes: 1 (bit) + 40 + 59 (inputs) + 41 + data (output) = 141 + output data
/// Bytes without lamports: 125 + output data
/// No input not lamports: 83 + output data
#[derive(Debug, ZeroCopy, Default, BorshSerialize, BorshDeserialize)]
pub struct CAccountInfo {
    // TODO: optimize parsing by manually implementing ZeroCopy and using the bitmask.
    // bitmask: u8,
    pub discriminator: [u8; 8], // 1
    /// Address.
    pub address: Option<[u8; 32]>, // 2
    /// Input account.
    pub input: Option<CInAccountInfo>, // 3
    /// Output account.
    pub output: Option<COutAccountInfo>, // 5
}

// Total legacy Zerocopy casts (1 in 1 out): 18
//
// Comparison ZPackedCompressedAccountWithMerkleContext zero copy casts: 9
// 1 (meta) + 8 (ZCompressedAccount)
//
// ZCompressedAccount Zero copy casts: 8
// 1 (meta) + 2 (address option) + (Option::<ZCompressedAccountData>) 5
//
// ZCompressedAccountData Zero copy casts: 4
//
// ZOutputCompressedAccountWithPackedContext Zero copy casts: 9

impl<'a, 'b> ZCInAccountInfoMut<'a> {
    pub fn from_input_meta<T: InputAccountMetaTrait>(&mut self, meta: &T, data_hash: [u8; 32]) {
        if let Some(input_lamports) = meta.get_lamports() {
            if let Some(lamports) = self.lamports.as_mut() {
                **lamports = input_lamports;
            }
        }
        self.data_hash = data_hash;
        if let Some(root_index) = meta.get_root_index().as_ref() {
            *self.root_index = U16::from(*root_index)
        }
        {
            let merkle_context = meta.get_merkle_context();
            self.merkle_context.leaf_index = merkle_context.leaf_index.into();
            self.merkle_context.merkle_tree_pubkey_index = merkle_context.merkle_tree_pubkey_index;
            self.merkle_context.nullifier_queue_pubkey_index =
                merkle_context.nullifier_queue_pubkey_index;
            self.merkle_context.prove_by_index = merkle_context.prove_by_index.into();
        }
    }

    pub fn from_z_input_meta<T: ZInputAccountMetaTrait<'b>>(
        &mut self,
        meta: &T,
        data_hash: [u8; 32],
    ) {
        if let Some(input_lamports) = meta.get_lamports() {
            if let Some(lamports) = self.lamports.as_mut() {
                **lamports = input_lamports.into();
            }
        }
        self.data_hash = data_hash;
        if let Some(root_index) = meta.get_root_index().as_ref() {
            *self.root_index = *root_index
        }
        {
            let merkle_context = meta.get_merkle_context();
            self.merkle_context
                .as_mut_bytes()
                .copy_from_slice(merkle_context.as_bytes());
        }
    }
}

/// Notes:
/// - we don't allow accounts without addresses
impl<'a, 'b> ZCAccountInfoMut<'a> {
    /// Initializes a compressed account info with address.
    /// 1. The account is zeroed, data has to be added in a separate step.
    /// 2. Once data is added the data hash has to be added.
    pub fn init_with_address(
        &mut self,
        discriminator: [u8; 8],
        address: [u8; 32],
        output_merkle_tree_index: u8,
    ) -> Result<(), CompressedAccountError> {
        self.discriminator = discriminator;
        if let Some(self_address) = self.address.as_mut() {
            self_address.copy_from_slice(&address);
        } else {
            solana_program::msg!("init_with_address: address is none");
            return Err(CompressedAccountError::InvalidAccountSize);
        }
        if let Some(output) = self.output.as_mut() {
            output.output_merkle_tree_index = output_merkle_tree_index;
            output.lamports = None;
        } else {
            solana_program::msg!("init_with_address: output is none");
            return Err(CompressedAccountError::InvalidAccountSize);
        }
        Ok(())
    }

    /// Initializes a compressed account info with address.
    /// 1. The account is zeroed, data has to be added in a separate step.
    /// 2. Once data is added the data hash has to be added.
    pub fn from_z_meta_mut<M: ZInputAccountMetaTrait<'b>>(
        &mut self,
        // Input
        input_account_meta: &M,
        input_data_hash: [u8; 32],
        discriminator: [u8; 8],
        output_merkle_tree_index: u8,
    ) -> Result<(), CompressedAccountError> {
        if self.discriminator != [0; 8] {
            solana_program::msg!(
                "from_z_meta_mut: discriminator is not zeroed. Account already loaded."
            );
            return Err(CompressedAccountError::InvalidAccountSize);
        }
        self.discriminator = discriminator;

        if let Some(self_address) = self.address.as_mut() {
            if let Some(address) = input_account_meta.get_address().as_ref() {
                **self_address = *address;
            } else {
                solana_program::msg!("from_z_meta_mut: address is none");
                return Err(CompressedAccountError::InvalidAccountSize);
            }
        } else {
            solana_program::msg!("from_z_meta_mut: address is none");
            return Err(CompressedAccountError::InvalidAccountSize);
        }

        if let Some(input) = self.input.as_mut() {
            input.from_z_input_meta(input_account_meta, input_data_hash);
        } else {
            solana_program::msg!("from_z_meta_mut: input is none");
            return Err(CompressedAccountError::InvalidAccountSize);
        }

        if let Some(output) = self.output.as_mut() {
            output.output_merkle_tree_index = output_merkle_tree_index;
            if let Some(lamports) = output.lamports.as_mut() {
                if let Some(input_lamports) = input_account_meta.get_lamports() {
                    **lamports = input_lamports.into();
                } else {
                    solana_program::msg!("from_z_meta_mut: output lamports is none");
                    return Err(CompressedAccountError::InvalidAccountSize);
                }
            }
        } else {
            solana_program::msg!("from_z_meta_mut: output is none");
            return Err(CompressedAccountError::InvalidAccountSize);
        }
        Ok(())
    }

    /// Initializes a compressed account info with address.
    /// 1. The account is zeroed, data has to be added in a separate step.
    /// 2. Once data is added the data hash has to be added.
    pub fn from_z_meta_close<M: ZInputAccountMetaTrait<'b>>(
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
                solana_program::msg!("from_z_meta_mut: address is none");
                return Err(CompressedAccountError::InvalidAccountSize);
            }
        } else {
            solana_program::msg!("from_z_meta_mut: address is none");
            return Err(CompressedAccountError::InvalidAccountSize);
        }

        if let Some(input) = self.input.as_mut() {
            input.from_z_input_meta(input_account_meta, input_data_hash);
        } else {
            solana_program::msg!("from_z_meta_mut: input is none");
            return Err(CompressedAccountError::InvalidAccountSize);
        }

        Ok(())
    }
}

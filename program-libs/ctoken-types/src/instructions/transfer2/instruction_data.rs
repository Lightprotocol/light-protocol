use light_compressed_account::{
    compressed_account::PackedMerkleContext, instruction_data::compressed_proof::CompressedProof,
};
use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use super::compression::Compression;
use crate::{instructions::transfer2::CompressedCpiContext, AnchorDeserialize, AnchorSerialize};

#[repr(C)]
#[derive(
    Debug,
    Copy,
    Clone,
    Default,
    PartialEq,
    AnchorSerialize,
    AnchorDeserialize,
    ZeroCopy,
    ZeroCopyMut,
)]
pub struct MultiInputTokenDataWithContext {
    pub owner: u8,
    pub amount: u64,
    pub has_delegate: bool, // Optional delegate is set
    pub delegate: u8,
    pub mint: u8,
    pub version: u8,
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
}

#[repr(C)]
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    AnchorSerialize,
    AnchorDeserialize,
    ZeroCopy,
    ZeroCopyMut,
)]
pub struct MultiTokenTransferOutputData {
    pub owner: u8,
    pub amount: u64,
    pub has_delegate: bool, // Optional delegate is set
    pub delegate: u8,
    pub mint: u8,
    pub version: u8,
    pub merkle_tree: u8,
}

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut)]
pub struct CompressedTokenInstructionDataTransfer2 {
    pub with_transaction_hash: bool,
    pub with_lamports_change_account_merkle_tree_index: bool,
    // Set zero if unused
    pub lamports_change_account_merkle_tree_index: u8,
    pub lamports_change_account_owner_index: u8,
    pub proof: Option<CompressedProof>,
    pub in_token_data: Vec<MultiInputTokenDataWithContext>,
    pub out_token_data: Vec<MultiTokenTransferOutputData>,
    pub in_lamports: Option<Vec<u64>>,
    pub out_lamports: Option<Vec<u64>>,
    pub in_tlv: Option<Vec<Vec<u8>>>,
    pub out_tlv: Option<Vec<Vec<u8>>>,
    pub compressions: Option<Vec<Compression>>,
    pub cpi_context: Option<CompressedCpiContext>,
}

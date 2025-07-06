use std::fmt::Debug;

use anchor_compressed_token::process_transfer::Amount;
use anchor_lang::{prelude::ProgramError, AnchorDeserialize, AnchorSerialize};
use light_compressed_account::instruction_data::{
    compressed_proof::CompressedProof, cpi_context::CompressedCpiContext,
};
use light_sdk::instruction::PackedMerkleContext;
use light_zero_copy::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut)]
pub struct MultiInputTokenDataWithContext {
    pub amount: u64,
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
    // From remaining accounts.
    pub mint: u8,
    pub owner: u8,
    pub with_delegate: bool,
    // Only used if with_delegate is true
    pub delegate: u8,
    // // Only used if with_delegate is true
    // pub delegate_change_account: u8,
    // pub lamports: Option<u64>, move into separate vector to opt zero copy
    // pub tlv: Option<Vec<u8>>, move into separate vector to opt zero copy
}

impl Amount for ZMultiInputTokenDataWithContext<'_> {
    fn amount(&self) -> u64 {
        self.amount.into()
    }
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
pub struct MultiTokenTransferOutputData {
    pub owner: u8,
    pub amount: u64,
    pub merkle_tree: u8,
    pub delegate: u8,
}

impl Amount for ZMultiTokenTransferOutputData<'_> {
    fn amount(&self) -> u64 {
        self.amount.into()
    }
}

// #[derive(
//     Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
// )]
// pub struct MultiTokenTransferDelegateOutputData {
//     pub delegate: u8,
//     pub owner: u8,
//     pub amount: u64,
//     pub merkle_tree: u8,
// }

// impl Amount for MultiTokenTransferDelegateOutputData {
//     fn amount(&self) -> u64 {
//         self.amount
//     }
// }

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut)]
pub struct CompressedTokenInstructionDataMultiTransfer {
    pub is_compress: bool,
    pub with_transaction_hash: bool,
    pub with_lamports_change_account_merkle_tree_index: bool,
    // Set zero if unused
    pub lamports_change_account_merkle_tree_index: u8,
    pub lamports_change_account_owner_index: u8,
    pub proof: Option<CompressedProof>,
    pub in_token_data: Vec<MultiInputTokenDataWithContext>,
    pub out_token_data: Vec<MultiTokenTransferOutputData>,
    // pub delegate_out_token_data: Option<Vec<MultiTokenTransferDelegateOutputData>>,
    // put accounts with lamports first, stop adding values after TODO: only access by get to prevent oob errors
    // TODO: add len check that < input_token_data_with_context.len()
    pub in_lamports: Option<Vec<u64>>,
    // put accounts with lamports first, stop adding values after TODO: only access by get to prevent oob errors
    // TODO: add len check that < output_token_data_with_context.len()
    pub out_lamports: Option<Vec<u64>>,
    // put accounts with tlv first, stop adding values after TODO: only access by get to prevent oob errors
    // TODO: add len check that < input_token_data_with_context.len()
    pub in_tlv: Option<Vec<Vec<u8>>>,
    pub out_tlv: Option<Vec<Vec<u8>>>,
    pub compress_or_decompress_amount: Option<u64>,
    pub cpi_context: Option<CompressedCpiContext>,
}

/// Validate instruction data consistency (lamports and TLV checks)
pub fn validate_instruction_data(
    inputs: &ZCompressedTokenInstructionDataMultiTransfer,
) -> Result<(), ProgramError> {
    if let Some(ref in_lamports) = inputs.in_lamports {
        if in_lamports.len() > inputs.in_token_data.len() {
            unimplemented!("Tlv is unimplemented");
        }
    }
    if let Some(ref out_lamports) = inputs.out_lamports {
        if out_lamports.len() > inputs.out_token_data.len() {
            unimplemented!("Tlv is unimplemented");
        }
    }
    if inputs.in_tlv.is_some() {
        unimplemented!("Tlv is unimplemented");
    }
    if inputs.out_tlv.is_some() {
        unimplemented!("Tlv is unimplemented");
    }
    Ok(())
}

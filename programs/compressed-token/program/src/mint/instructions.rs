use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::{instruction_data::compressed_proof::CompressedProof, Pubkey};
use light_zero_copy::ZeroCopy;

use crate::extensions::ExtensionInstructionData;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct CreateCompressedMintInstructionData {
    pub decimals: u8,
    pub mint_authority: Pubkey,
    pub proof: CompressedProof,
    pub mint_bump: u8,
    pub address_merkle_tree_root_index: u16,
    // compressed address TODO: make a type CompressedAddress
    pub mint_address: [u8; 32],
    pub freeze_authority: Option<Pubkey>,
    pub version: u8,
    pub extensions: Option<Vec<ExtensionInstructionData>>,
}

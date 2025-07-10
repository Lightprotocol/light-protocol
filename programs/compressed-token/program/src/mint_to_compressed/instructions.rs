use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::{
    compressed_account::PackedMerkleContext, instruction_data::compressed_proof::CompressedProof,
    Pubkey,
};
use light_zero_copy::ZeroCopy;

use crate::mint::{instructions::UpdateCompressedMintInstructionData, state::CompressedMint};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct CompressedMintInputs {
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
    pub address: [u8; 32],
    pub compressed_mint_input: CompressedMint,
    pub output_merkle_tree_index: u8,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct Recipient {
    pub recipient: Pubkey,
    pub amount: u64,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct MintToCompressedInstructionData {
    pub compressed_mint_inputs: UpdateCompressedMintInstructionData,
    pub lamports: Option<u64>,
    pub recipients: Vec<Recipient>,
    pub proof: Option<CompressedProof>,
}

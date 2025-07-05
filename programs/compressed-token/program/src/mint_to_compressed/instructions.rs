use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::{
    compressed_account::PackedMerkleContext, instruction_data::compressed_proof::CompressedProof,
    Pubkey,
};
use light_zero_copy::ZeroCopy;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct CompressedMintInputs {
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
    pub address: [u8; 32],
    pub compressed_mint_input: CompressedMintInput,
    pub output_merkle_tree_index: u8,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct CompressedMintInput {
    pub spl_mint: Pubkey,
    pub supply: u64,
    pub decimals: u8,
    pub is_decompressed: bool,
    pub freeze_authority_is_set: bool,
    pub freeze_authority: Pubkey,
    pub num_extensions: u8,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct Recipient {
    pub recipient: Pubkey,
    pub amount: u64,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct MintToCompressedInstructionData {
    pub lamports: u64,
    pub compressed_mint_inputs: CompressedMintInputs,
    pub recipients: Vec<Recipient>,
    pub proof: Option<CompressedProof>,
}

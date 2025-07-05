use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::{instruction_data::compressed_proof::CompressedProof, Pubkey};
use light_zero_copy::ZeroCopy;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct CreateCompressedMintInstructionData {
    pub decimals: u8,
    pub mint_authority: Pubkey,
    pub freeze_authority: Option<Pubkey>,
    pub proof: CompressedProof,
    pub mint_bump: u8,
    pub address_merkle_tree_root_index: u16,
}

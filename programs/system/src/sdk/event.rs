use anchor_lang::{
    prelude::borsh, solana_program::pubkey::Pubkey, AnchorDeserialize, AnchorSerialize,
};

use crate::OutputCompressedAccountWithPackedContext;

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, Default, PartialEq)]
pub struct MerkleTreeSequenceNumber {
    pub pubkey: Pubkey,
    pub seq: u64,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, Default, PartialEq)]
pub struct PublicTransactionEvent {
    pub input_compressed_account_hashes: Vec<[u8; 32]>,
    pub output_compressed_account_hashes: Vec<[u8; 32]>,
    pub output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    pub output_leaf_indices: Vec<u32>,
    pub sequence_numbers: Vec<MerkleTreeSequenceNumber>,
    pub relay_fee: Option<u64>,
    pub is_compress: bool,
    pub compress_or_decompress_lamports: Option<u64>,
    pub pubkey_array: Vec<Pubkey>,
    pub message: Option<Vec<u8>>,
}

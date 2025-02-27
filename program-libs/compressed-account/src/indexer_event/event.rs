use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

use crate::instruction_data::{
    data::OutputCompressedAccountWithPackedContext,
    insert_into_queues::MerkleTreeSequenceNumber as InstructionDataSequenceNumber,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BatchPublicTransactionEvent {
    pub event: PublicTransactionEvent,
    pub new_addresses: Vec<NewAddress>,
    pub input_sequence_numbers: Vec<MerkleTreeSequenceNumber>,
    pub address_sequence_numbers: Vec<MerkleTreeSequenceNumber>,
    pub tx_hash: [u8; 32],
    pub batch_input_accounts: Vec<BatchNullifyContext>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Default, PartialEq)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NewAddress {
    pub address: [u8; 32],
    pub mt_pubkey: Pubkey,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct BatchNullifyContext {
    pub tx_hash: [u8; 32],
    pub account_hash: [u8; 32],
    pub nullifier: [u8; 32],
    pub nullifier_queue_index: u64,
}

// Separate type because zerocopy::U64 doesn't implement BorshSerialize.
#[derive(Debug, Clone, Copy, BorshSerialize, BorshDeserialize, Default, PartialEq)]
pub struct MerkleTreeSequenceNumber {
    pub pubkey: Pubkey,
    pub seq: u64,
}

impl MerkleTreeSequenceNumber {
    pub fn is_empty(&self) -> bool {
        self.pubkey == Pubkey::default()
    }
}

impl From<&InstructionDataSequenceNumber> for MerkleTreeSequenceNumber {
    fn from(seq: &InstructionDataSequenceNumber) -> Self {
        Self {
            pubkey: seq.pubkey.into(),
            seq: seq.seq.into(),
        }
    }
}

use anchor_lang::{AnchorDeserialize, AnchorSerialize};
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_indexed_merkle_tree::array::IndexedElement;
use num_bigint::BigUint;
use solana_program::pubkey::Pubkey;

#[derive(Debug, Clone)]
pub struct MerkleProof {
    pub hash: [u8; 32],
    pub leaf_index: u64,
    pub merkle_tree: Pubkey,
    pub proof: Vec<[u8; 32]>,
    pub root_seq: u64,
}

// For consistency with the Photon API.
#[derive(Clone, Default, Debug, PartialEq)]
pub struct NewAddressProofWithContext {
    pub merkle_tree: Pubkey,
    pub root: [u8; 32],
    pub root_seq: u64,
    pub low_address_index: u64,
    pub low_address_value: [u8; 32],
    pub low_address_next_index: u64,
    pub low_address_next_value: [u8; 32],
    pub low_address_proof: [[u8; 32]; 16],
    pub new_low_element: Option<IndexedElement<usize>>,
    pub new_element: Option<IndexedElement<usize>>,
    pub new_element_next_value: Option<BigUint>,
}

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct ProofRpcResult {
    pub proof: CompressedProof,
    pub root_indices: Vec<Option<u16>>,
    pub address_root_indices: Vec<u16>,
}

#[derive(Debug, Default)]
pub struct BatchedTreeProofRpcResult {
    pub proof: Option<CompressedProof>,
    // If none -> proof by index  and not included in zkp, else included in zkp
    pub root_indices: Vec<Option<u16>>,
    pub address_root_indices: Vec<u16>,
}

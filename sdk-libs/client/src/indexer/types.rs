use solana_program::pubkey::Pubkey;

pub struct ProofOfLeaf {
    pub leaf: [u8; 32],
    pub proof: Vec<[u8; 32]>,
}

pub type Address = [u8; 32];
pub type Hash = [u8; 32];

pub struct AddressWithTree {
    pub address: Address,
    pub tree: Pubkey,
}

#[derive(Debug, Clone)]
pub struct MerkleProofWithContext {
    pub proof: Vec<[u8; 32]>,
    pub root: [u8; 32],
    pub leaf_index: u64,
    pub leaf: [u8; 32],
    pub merkle_tree: [u8; 32],
    pub root_seq: u64,
    pub tx_hash: Option<[u8; 32]>,
    pub account_hash: [u8; 32],
}

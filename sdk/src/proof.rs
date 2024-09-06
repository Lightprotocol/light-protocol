use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct CompressedProof {
    pub a: [u8; 32],
    pub b: [u8; 64],
    pub c: [u8; 32],
}

impl anchor_lang::IdlBuild for CompressedProof {}

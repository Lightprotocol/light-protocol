use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, Default, Clone, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct BatchCompressInstructionData {
    pub pubkeys: Vec<[u8; 32]>,
    // Some if one amount per pubkey.
    pub amounts: Option<Vec<u64>>,
    pub lamports: Option<u64>,
    // Some if one amount across all pubkeys.
    pub amount: Option<u64>,
    pub index: u8,
    pub bump: u8,
}

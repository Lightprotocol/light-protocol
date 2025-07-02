use borsh::{BorshDeserialize, BorshSerialize};

// Note: MintToInstruction is an Anchor account struct, not an instruction data struct
// This file is for completeness but there's no specific MintToInstructionData type
// The mint_to instruction uses pubkeys and amounts directly as parameters

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct MintToParams {
    pub public_keys: Vec<[u8; 32]>,
    pub amounts: Vec<u64>,
    pub lamports: Option<u64>,
}

use borsh::{BorshDeserialize, BorshSerialize};

// Generic instruction data wrapper that can hold any instruction data as bytes
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct GenericInstructionData {
    pub instruction_data: Vec<u8>,
}

// Type alias for the main generic instruction data type
pub type CompressedTokenInstructionData = GenericInstructionData;

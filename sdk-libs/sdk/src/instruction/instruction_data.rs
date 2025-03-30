use light_compressed_account::instruction_data::compressed_proof::CompressedProof;

use crate::{
    instruction::merkle_context::PackedAddressMerkleContext, BorshDeserialize, BorshSerialize,
};

#[derive(Debug, Default, Clone, BorshSerialize, PartialEq, BorshDeserialize)]
pub struct LightInstructionData {
    pub proof: Option<CompressedProof>,
    pub new_addresses: Option<Vec<PackedAddressMerkleContext>>,
}

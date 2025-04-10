use light_compressed_account::instruction_data::compressed_proof::CompressedProof;

use crate::{
    instruction::merkle_context::PackedAddressMerkleContext, AnchorDeserialize, AnchorSerialize,
};

#[derive(Debug, Default, Clone, AnchorSerialize, PartialEq, AnchorDeserialize)]
pub struct LightInstructionData {
    pub proof: Option<CompressedProof>,
    pub new_addresses: Option<Vec<PackedAddressMerkleContext>>,
}

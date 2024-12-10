use crate::{
    helpers::bigint_to_u8_32,
    non_inclusion::merkle_non_inclusion_proof_inputs::NonInclusionMerkleProofInputs,
};

impl NonInclusionMerkleProofInputs {
    pub fn public_inputs_legacy(&self) -> [[u8; 32]; 2] {
        let root = bigint_to_u8_32(&self.root).unwrap();
        let value = bigint_to_u8_32(&self.value).unwrap();
        [root, value]
    }
}

#[derive(Clone, Debug)]
pub struct NonInclusionProofInputs<'a>(pub &'a [NonInclusionMerkleProofInputs]);

impl<'a> NonInclusionProofInputs<'a> {
    pub fn new(non_inclusion_merkle_proof_inputs: &'a [NonInclusionMerkleProofInputs]) -> Self {
        NonInclusionProofInputs(non_inclusion_merkle_proof_inputs)
    }
}

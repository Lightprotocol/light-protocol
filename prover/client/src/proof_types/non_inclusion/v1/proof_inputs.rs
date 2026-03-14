use crate::{
    errors::ProverClientError, helpers::bigint_to_u8_32,
    proof_types::non_inclusion::v2::NonInclusionMerkleProofInputs,
};

impl NonInclusionMerkleProofInputs {
    pub fn public_inputs_legacy(&self) -> Result<[[u8; 32]; 2], ProverClientError> {
        let root = bigint_to_u8_32(&self.root)?;
        let value = bigint_to_u8_32(&self.value)?;
        Ok([root, value])
    }
}

#[derive(Clone, Debug)]
pub struct NonInclusionProofInputs<'a>(pub &'a [NonInclusionMerkleProofInputs]);

impl<'a> NonInclusionProofInputs<'a> {
    pub fn new(non_inclusion_merkle_proof_inputs: &'a [NonInclusionMerkleProofInputs]) -> Self {
        NonInclusionProofInputs(non_inclusion_merkle_proof_inputs)
    }
}

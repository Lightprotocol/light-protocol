use crate::{
    inclusion::merkle_inclusion_proof_inputs::InclusionProofInputs,
    non_inclusion::merkle_non_inclusion_proof_inputs::NonInclusionProofInputs,
};

#[derive(Clone, Debug)]
pub struct CombinedProofInputs<'a> {
    pub inclusion_parameters: InclusionProofInputs<'a>,
    pub non_inclusion_parameters: NonInclusionProofInputs<'a>,
}

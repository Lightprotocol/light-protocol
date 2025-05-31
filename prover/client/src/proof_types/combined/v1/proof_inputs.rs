use crate::proof_types::{
    inclusion::v2::InclusionProofInputs, non_inclusion::v1::NonInclusionProofInputs,
};

#[derive(Clone, Debug)]
pub struct CombinedProofInputs<'a> {
    pub inclusion_parameters: InclusionProofInputs<'a>,
    pub non_inclusion_parameters: NonInclusionProofInputs<'a>,
}

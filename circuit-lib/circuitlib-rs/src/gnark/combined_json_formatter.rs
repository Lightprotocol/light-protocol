use serde::Serialize;

use crate::combined::merkle_combined_proof_inputs::CombinedProofInputs;

use super::{
    helpers::create_json_from_struct, inclusion_json_formatter::InclusionJsonStruct,
    non_inclusion_json_formatter::NonInclusionJsonStruct,
};

#[allow(non_snake_case)]
#[derive(Serialize, Debug)]
pub struct CombinedJsonStruct {
    pub inclusion: InclusionJsonStruct,
    pub nonInclusion: NonInclusionJsonStruct,
}

impl CombinedJsonStruct {
    pub fn from_combined_inputs(inputs: &CombinedProofInputs) -> Self {
        let inclusion_parameters =
            InclusionJsonStruct::from_inclusion_proof_inputs(&inputs.inclusion_parameters);
        let non_inclusion_parameters = NonInclusionJsonStruct::from_non_inclusion_proof_inputs(
            &inputs.non_inclusion_parameters,
        );
        Self {
            inclusion: inclusion_parameters,
            nonInclusion: non_inclusion_parameters,
        }
    }
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        create_json_from_struct(&self)
    }
}

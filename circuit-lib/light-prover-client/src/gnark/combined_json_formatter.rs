use serde::Serialize;

use crate::combined::merkle_combined_proof_inputs::CombinedProofInputs;
use crate::gnark::inclusion_json_formatter::BatchInclusionJsonStruct;
use crate::gnark::non_inclusion_json_formatter::BatchNonInclusionJsonStruct;

use super::{
    helpers::create_json_from_struct, inclusion_json_formatter::InclusionJsonStruct,
    non_inclusion_json_formatter::NonInclusionJsonStruct,
};

#[derive(Serialize, Debug)]
pub struct CombinedJsonStruct {
    #[serde(rename(serialize = "input-compressed-accounts"))]
    pub inclusion: Vec<InclusionJsonStruct>,

    #[serde(rename(serialize = "new-addresses"))]
    pub non_inclusion: Vec<NonInclusionJsonStruct>,
}

impl CombinedJsonStruct {
    pub fn from_combined_inputs(inputs: &CombinedProofInputs) -> Self {
        let inclusion_parameters =
            BatchInclusionJsonStruct::from_inclusion_proof_inputs(&inputs.inclusion_parameters);
        let non_inclusion_parameters = BatchNonInclusionJsonStruct::from_non_inclusion_proof_inputs(
            &inputs.non_inclusion_parameters,
        );
        Self {
            inclusion: inclusion_parameters.inputs,
            non_inclusion: non_inclusion_parameters.inputs,
        }
    }
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        create_json_from_struct(&self)
    }
}

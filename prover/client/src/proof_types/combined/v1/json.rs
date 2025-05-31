use serde::Serialize;

use crate::{
    helpers::create_json_from_struct,
    proof_types::{
        circuit_type::CircuitType,
        combined::v1::CombinedProofInputs,
        inclusion::v2::{BatchInclusionJsonStruct, InclusionJsonStruct},
        non_inclusion::v1::{BatchNonInclusionJsonStruct, LegacyNonInclusionJsonStruct},
    },
};
#[derive(Serialize, Debug)]
pub struct CombinedJsonStruct {
    #[serde(rename = "circuitType")]
    pub circuit_type: String,
    #[serde(rename = "stateTreeHeight")]
    pub state_tree_height: u32,
    #[serde(rename = "addressTreeHeight")]
    pub address_tree_height: u32,
    #[serde(rename(serialize = "inputCompressedAccounts"))]
    pub inclusion: Vec<InclusionJsonStruct>,
    #[serde(rename(serialize = "newAddresses"))]
    pub non_inclusion: Vec<LegacyNonInclusionJsonStruct>,
}

impl CombinedJsonStruct {
    pub fn from_combined_inputs(inputs: &CombinedProofInputs) -> Self {
        let inclusion_parameters =
            BatchInclusionJsonStruct::from_inclusion_proof_inputs(&inputs.inclusion_parameters);
        let non_inclusion_parameters = BatchNonInclusionJsonStruct::from_non_inclusion_proof_inputs(
            &inputs.non_inclusion_parameters,
        );
        Self {
            circuit_type: CircuitType::Combined.to_string(),
            state_tree_height: inclusion_parameters.state_tree_height,
            address_tree_height: non_inclusion_parameters.address_tree_height,
            inclusion: inclusion_parameters.inputs,
            non_inclusion: non_inclusion_parameters.inputs,
        }
    }
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        create_json_from_struct(&self)
    }
}

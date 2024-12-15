use serde::Serialize;

use super::{
    helpers::create_json_from_struct, inclusion_json_formatter::InclusionJsonStruct,
    non_inclusion_json_formatter::NonInclusionJsonStruct,
};
use crate::{
    combined_legacy::merkle_combined_proof_inputs::CombinedProofInputs,
    gnark::{
        inclusion_json_formatter_legacy::BatchInclusionJsonStruct,
        non_inclusion_json_formatter_legacy::BatchNonInclusionJsonStruct,
    },
    prove_utils::CircuitType,
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
    pub non_inclusion: Vec<NonInclusionJsonStruct>,
}

impl CombinedJsonStruct {
    fn new_with_public_inputs(num_inclusion: usize, num_non_inclusion: usize) -> Self {
        let inclusion = BatchInclusionJsonStruct::new_with_public_inputs(num_inclusion);
        let (non_inclusion, _) =
            BatchNonInclusionJsonStruct::new_with_public_inputs(num_non_inclusion);

        Self {
            circuit_type: CircuitType::Combined.to_string(),
            state_tree_height: inclusion.state_tree_height,
            address_tree_height: non_inclusion.address_tree_height,

            inclusion: inclusion.inputs,
            non_inclusion: non_inclusion.inputs,
        }
    }

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

pub fn combined_inputs_string(num_inclusion: usize, num_non_inclusion: usize) -> String {
    let json_struct = CombinedJsonStruct::new_with_public_inputs(num_inclusion, num_non_inclusion);
    json_struct.to_string()
}

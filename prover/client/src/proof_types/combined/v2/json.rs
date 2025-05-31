use serde::Serialize;

use crate::{
    constants::{DEFAULT_BATCH_ADDRESS_TREE_HEIGHT, DEFAULT_BATCH_STATE_TREE_HEIGHT},
    helpers::{big_int_to_string, create_json_from_struct},
    proof_types::{
        circuit_type::CircuitType,
        combined::v2::CombinedProofInputs,
        inclusion::v2::{BatchInclusionJsonStruct, InclusionJsonStruct},
        non_inclusion::v2::{BatchNonInclusionJsonStruct, NonInclusionJsonStruct},
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
    #[serde(rename = "publicInputHash")]
    pub public_input_hash: String,
    #[serde(rename(serialize = "inputCompressedAccounts"))]
    pub inclusion: Vec<InclusionJsonStruct>,

    #[serde(rename(serialize = "newAddresses"))]
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
            circuit_type: CircuitType::Combined.to_string(),
            state_tree_height: DEFAULT_BATCH_STATE_TREE_HEIGHT,
            address_tree_height: DEFAULT_BATCH_ADDRESS_TREE_HEIGHT,
            public_input_hash: big_int_to_string(&inputs.public_input_hash),
            inclusion: inclusion_parameters.inputs,
            non_inclusion: non_inclusion_parameters.inputs,
        }
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        create_json_from_struct(&self)
    }
}

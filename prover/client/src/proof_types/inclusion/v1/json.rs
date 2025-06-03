use num_traits::ToPrimitive;
use serde::Serialize;

use crate::{
    helpers::{big_int_to_string, create_json_from_struct},
    proof_types::{
        circuit_type::CircuitType,
        inclusion::{v1::InclusionProofInputs, v2::InclusionJsonStruct},
    },
};
// TODO: why is this called Batch?
#[derive(Serialize, Debug)]
pub struct BatchInclusionJsonStruct {
    #[serde(rename = "circuitType")]
    pub circuit_type: String,
    #[serde(rename = "stateTreeHeight")]
    pub state_tree_height: u32,
    #[serde(rename(serialize = "inputCompressedAccounts"))]
    pub inputs: Vec<InclusionJsonStruct>,
}

impl BatchInclusionJsonStruct {
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        create_json_from_struct(&self)
    }

    pub fn from_inclusion_proof_inputs(inputs: &InclusionProofInputs) -> Self {
        let mut proof_inputs: Vec<InclusionJsonStruct> = Vec::new();
        for input in inputs.0.iter() {
            let proof_input = InclusionJsonStruct {
                root: big_int_to_string(&input.root),
                leaf: big_int_to_string(&input.leaf),
                pathIndex: input.path_index.to_u32().unwrap(),
                pathElements: input.path_elements.iter().map(big_int_to_string).collect(),
            };
            proof_inputs.push(proof_input);
        }
        Self {
            circuit_type: CircuitType::Inclusion.to_string(),
            state_tree_height: 26,
            inputs: proof_inputs,
        }
    }
}

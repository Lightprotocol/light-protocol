use num_traits::ToPrimitive;
use serde::Serialize;

use crate::{
    constants::DEFAULT_BATCH_STATE_TREE_HEIGHT,
    helpers::{big_int_to_string, create_json_from_struct},
    proof_types::{circuit_type::CircuitType, inclusion::v2::InclusionProofInputs},
};

#[derive(Serialize, Debug)]
pub struct BatchInclusionJsonStruct {
    #[serde(rename = "circuitType")]
    pub circuit_type: String,
    #[serde(rename = "stateTreeHeight")]
    pub state_tree_height: u32,
    #[serde(rename = "publicInputHash")]
    pub public_input_hash: String,
    #[serde(rename(serialize = "inputCompressedAccounts"))]
    pub inputs: Vec<InclusionJsonStruct>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Clone, Debug)]
pub struct InclusionJsonStruct {
    #[serde(rename = "root")]
    pub root: String,
    #[serde(rename = "leaf")]
    pub leaf: String,
    #[serde(rename = "pathIndex")]
    pub pathIndex: u32,
    #[serde(rename = "pathElements")]
    pub pathElements: Vec<String>,
}

impl BatchInclusionJsonStruct {
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        create_json_from_struct(&self)
    }

    pub fn from_inclusion_proof_inputs(inputs: &InclusionProofInputs) -> Self {
        let mut proof_inputs: Vec<InclusionJsonStruct> = Vec::new();
        for input in inputs.inputs.iter() {
            let prof_input = InclusionJsonStruct {
                root: big_int_to_string(&input.root),
                leaf: big_int_to_string(&input.leaf),
                pathIndex: input.path_index.to_u32().unwrap(),
                pathElements: input.path_elements.iter().map(big_int_to_string).collect(),
            };
            proof_inputs.push(prof_input);
        }
        Self {
            circuit_type: CircuitType::Inclusion.to_string(),
            state_tree_height: DEFAULT_BATCH_STATE_TREE_HEIGHT,
            public_input_hash: big_int_to_string(&inputs.public_input_hash),
            inputs: proof_inputs,
        }
    }
}

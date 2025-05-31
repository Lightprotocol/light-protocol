use num_traits::ToPrimitive;
use serde::Serialize;

use crate::{
    helpers::{big_int_to_string, create_json_from_struct},
    proof_types::{circuit_type::CircuitType, non_inclusion::v2::NonInclusionProofInputs},
};

#[derive(Serialize, Debug)]
pub struct BatchNonInclusionJsonStruct {
    #[serde(rename = "circuitType")]
    pub circuit_type: String,
    #[serde(rename = "addressTreeHeight")]
    pub address_tree_height: u32,
    #[serde(rename = "publicInputHash")]
    pub public_input_hash: String,
    #[serde(rename(serialize = "newAddresses"))]
    pub inputs: Vec<NonInclusionJsonStruct>,
}

#[derive(Serialize, Clone, Debug)]
pub struct NonInclusionJsonStruct {
    pub root: String,
    pub value: String,

    #[serde(rename(serialize = "pathIndex"))]
    pub path_index: u32,

    #[serde(rename(serialize = "pathElements"))]
    pub path_elements: Vec<String>,

    #[serde(rename(serialize = "leafLowerRangeValue"))]
    pub leaf_lower_range_value: String,

    #[serde(rename(serialize = "leafHigherRangeValue"))]
    pub leaf_higher_range_value: String,
}

impl BatchNonInclusionJsonStruct {
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        create_json_from_struct(&self)
    }

    pub fn from_non_inclusion_proof_inputs(inputs: &NonInclusionProofInputs) -> Self {
        let mut proof_inputs: Vec<NonInclusionJsonStruct> = Vec::new();
        for input in inputs.inputs.iter() {
            let prof_input = NonInclusionJsonStruct {
                root: big_int_to_string(&input.root),
                value: big_int_to_string(&input.value),
                path_index: input.index_hashed_indexed_element_leaf.to_u32().unwrap(),
                path_elements: input
                    .merkle_proof_hashed_indexed_element_leaf
                    .iter()
                    .map(big_int_to_string)
                    .collect(),
                leaf_lower_range_value: big_int_to_string(&input.leaf_lower_range_value),
                leaf_higher_range_value: big_int_to_string(&input.leaf_higher_range_value),
            };
            proof_inputs.push(prof_input);
        }

        Self {
            circuit_type: CircuitType::NonInclusion.to_string(),
            address_tree_height: 40,
            public_input_hash: big_int_to_string(&inputs.public_input_hash),
            inputs: proof_inputs,
        }
    }
}

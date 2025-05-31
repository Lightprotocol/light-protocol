use num_traits::ToPrimitive;
use serde::Serialize;

use crate::{
    helpers::{big_int_to_string, create_json_from_struct},
    proof_types::{circuit_type::CircuitType, non_inclusion::v1::NonInclusionProofInputs},
};

#[derive(Serialize, Debug)]
pub struct BatchNonInclusionJsonStruct {
    #[serde(rename = "circuitType")]
    pub circuit_type: String,
    #[serde(rename = "addressTreeHeight")]
    pub address_tree_height: u32,
    #[serde(rename(serialize = "newAddresses"))]
    pub inputs: Vec<LegacyNonInclusionJsonStruct>,
}

#[derive(Serialize, Clone, Debug)]
pub struct LegacyNonInclusionJsonStruct {
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

    #[serde(rename(serialize = "nextIndex"))]
    pub next_index: u32,
}

impl BatchNonInclusionJsonStruct {
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        create_json_from_struct(&self)
    }

    pub fn from_non_inclusion_proof_inputs(inputs: &NonInclusionProofInputs) -> Self {
        let mut proof_inputs: Vec<LegacyNonInclusionJsonStruct> = Vec::new();
        for input in inputs.0 {
            let prof_input = LegacyNonInclusionJsonStruct {
                root: big_int_to_string(&input.root),
                value: big_int_to_string(&input.value),
                path_index: input.index_hashed_indexed_element_leaf.to_u32().unwrap(),
                path_elements: input
                    .merkle_proof_hashed_indexed_element_leaf
                    .iter()
                    .map(big_int_to_string)
                    .collect(),
                next_index: input.next_index.to_u32().unwrap(),
                leaf_lower_range_value: big_int_to_string(&input.leaf_lower_range_value),
                leaf_higher_range_value: big_int_to_string(&input.leaf_higher_range_value),
            };
            proof_inputs.push(prof_input);
        }

        Self {
            circuit_type: CircuitType::NonInclusion.to_string(),
            address_tree_height: 26,
            inputs: proof_inputs,
        }
    }
}

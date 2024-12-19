use crate::gnark::helpers::{big_int_to_string, create_json_from_struct};
use crate::inclusion_legacy::merkle_inclusion_proof_inputs::InclusionProofInputs;
use crate::prove_utils::CircuitType;
use crate::{
    inclusion::merkle_tree_info::MerkleTreeInfo, init_merkle_tree::inclusion_merkle_tree_inputs,
};
use num_traits::ToPrimitive;
use serde::Serialize;

use super::inclusion_json_formatter::InclusionJsonStruct;

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
    pub fn new_with_public_inputs(number_of_utxos: usize) -> Self {
        let merkle_inputs = inclusion_merkle_tree_inputs(MerkleTreeInfo::H26);

        let input = InclusionJsonStruct {
            root: big_int_to_string(&merkle_inputs.root),
            leaf: big_int_to_string(&merkle_inputs.leaf),
            pathElements: merkle_inputs
                .path_elements
                .iter()
                .map(big_int_to_string)
                .collect(),
            pathIndex: merkle_inputs.path_index.to_u32().unwrap(),
        };

        let inputs = vec![input; number_of_utxos];
        Self {
            circuit_type: CircuitType::Inclusion.to_string(),
            state_tree_height: 26,
            inputs,
        }
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        create_json_from_struct(&self)
    }

    pub fn from_inclusion_proof_inputs(inputs: &InclusionProofInputs) -> Self {
        let mut proof_inputs: Vec<InclusionJsonStruct> = Vec::new();
        for input in inputs.0.iter() {
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
            state_tree_height: 26,
            inputs: proof_inputs,
        }
    }
}

pub fn inclusion_inputs_string(number_of_utxos: usize) -> String {
    let json_struct = BatchInclusionJsonStruct::new_with_public_inputs(number_of_utxos);
    json_struct.to_string()
}

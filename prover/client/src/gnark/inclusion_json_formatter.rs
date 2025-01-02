use light_batched_merkle_tree::constants::DEFAULT_BATCH_STATE_TREE_HEIGHT;
use light_utils::hashchain::create_two_inputs_hash_chain;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use serde::Serialize;

use crate::{
    gnark::helpers::{big_int_to_string, create_json_from_struct},
    helpers::bigint_to_u8_32,
    inclusion::{
        merkle_inclusion_proof_inputs::InclusionProofInputs, merkle_tree_info::MerkleTreeInfo,
    },
    init_merkle_tree::inclusion_merkle_tree_inputs,
    prove_utils::CircuitType,
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
    pub fn new_with_public_inputs(number_of_utxos: usize) -> (Self, [u8; 32]) {
        let merkle_inputs = inclusion_merkle_tree_inputs(MerkleTreeInfo::H32);

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
        let public_input_hash = create_two_inputs_hash_chain(
            vec![bigint_to_u8_32(&merkle_inputs.root).unwrap(); number_of_utxos].as_slice(),
            vec![bigint_to_u8_32(&merkle_inputs.leaf).unwrap(); number_of_utxos].as_slice(),
        )
        .unwrap();
        let public_input_hash_string = big_int_to_string(&BigInt::from_bytes_be(
            num_bigint::Sign::Plus,
            &public_input_hash,
        ));
        (
            Self {
                circuit_type: CircuitType::Inclusion.to_string(),
                state_tree_height: DEFAULT_BATCH_STATE_TREE_HEIGHT,
                public_input_hash: public_input_hash_string,
                inputs,
            },
            public_input_hash,
        )
    }

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

pub fn inclusion_inputs_string(number_of_utxos: usize) -> String {
    let (json_struct, _) = BatchInclusionJsonStruct::new_with_public_inputs(number_of_utxos);
    json_struct.to_string()
}

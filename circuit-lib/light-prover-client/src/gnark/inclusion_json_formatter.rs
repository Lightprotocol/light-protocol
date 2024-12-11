use crate::batch_append_with_subtrees::calculate_two_inputs_hash_chain;
use crate::gnark::helpers::{big_int_to_string, create_json_from_struct};
use crate::helpers::bigint_to_u8_32;
use crate::{
    inclusion::{
        merkle_inclusion_proof_inputs::{InclusionMerkleProofInputs, InclusionProofInputs},
        merkle_tree_info::MerkleTreeInfo,
    },
    init_merkle_tree::inclusion_merkle_tree_inputs,
};
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct BatchInclusionJsonStruct {
    #[serde(rename = "circuitType")]
    pub circuit_type: String,
    #[serde(rename = "publicInputHash")]
    pub public_input_hash: String,
    #[serde(rename(serialize = "input-compressed-accounts"))]
    pub inputs: Vec<InclusionJsonStruct>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Clone, Debug)]
pub struct InclusionJsonStruct {
    pub root: String,
    pub leaf: String,
    pub pathIndex: u32,
    pub pathElements: Vec<String>,
}

impl BatchInclusionJsonStruct {
    fn new_with_public_inputs(number_of_utxos: usize) -> (Self, InclusionMerkleProofInputs) {
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
        let public_input_hash = calculate_two_inputs_hash_chain(
            vec![bigint_to_u8_32(&merkle_inputs.root).unwrap(); number_of_utxos].as_slice(),
            vec![bigint_to_u8_32(&merkle_inputs.leaf).unwrap(); number_of_utxos].as_slice(),
        );
        let public_input_hash = big_int_to_string(&BigInt::from_bytes_be(
            num_bigint::Sign::Plus,
            &public_input_hash,
        ));
        (
            Self {
                circuit_type: "inclusion".to_string(),
                public_input_hash,
                inputs,
            },
            merkle_inputs,
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
            circuit_type: "inclusion".to_string(),
            public_input_hash: big_int_to_string(&inputs.public_input_hash),
            inputs: proof_inputs,
        }
    }
}

pub fn inclusion_inputs_string(number_of_utxos: usize) -> (String, InclusionMerkleProofInputs) {
    let (json_struct, public_inputs) =
        BatchInclusionJsonStruct::new_with_public_inputs(number_of_utxos);
    (json_struct.to_string(), public_inputs)
}

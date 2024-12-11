use crate::batch_append_with_subtrees::calculate_two_inputs_hash_chain;
use crate::gnark::helpers::big_int_to_string;
use crate::helpers::bigint_to_u8_32;
use crate::{
    gnark::helpers::create_json_from_struct,
    init_merkle_tree::non_inclusion_merkle_tree_inputs_26,
    non_inclusion::merkle_non_inclusion_proof_inputs::{
        NonInclusionMerkleProofInputs, NonInclusionProofInputs,
    },
};
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct BatchNonInclusionJsonStruct {
    #[serde(rename = "circuitType")]
    pub circuit_type: String,
    #[serde(rename = "publicInputHash")]
    pub public_input_hash: String,
    #[serde(rename(serialize = "new-addresses"))]
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

    #[serde(rename(serialize = "nextIndex"))]
    pub next_index: u32,
}

impl BatchNonInclusionJsonStruct {
    fn new_with_public_inputs(number_of_utxos: usize) -> (Self, NonInclusionMerkleProofInputs) {
        let merkle_inputs = non_inclusion_merkle_tree_inputs_26();

        let input = NonInclusionJsonStruct {
            root: big_int_to_string(&merkle_inputs.root),
            value: big_int_to_string(&merkle_inputs.value),
            path_elements: merkle_inputs
                .merkle_proof_hashed_indexed_element_leaf
                .iter()
                .map(big_int_to_string)
                .collect(),
            path_index: merkle_inputs
                .index_hashed_indexed_element_leaf
                .to_u32()
                .unwrap(),
            next_index: merkle_inputs.next_index.to_u32().unwrap(),
            leaf_lower_range_value: big_int_to_string(&merkle_inputs.leaf_lower_range_value),
            leaf_higher_range_value: big_int_to_string(&merkle_inputs.leaf_higher_range_value),
        };
        let inputs = vec![input; number_of_utxos];
        let public_input_hash = calculate_two_inputs_hash_chain(
            vec![bigint_to_u8_32(&merkle_inputs.root).unwrap(); number_of_utxos].as_slice(),
            vec![bigint_to_u8_32(&merkle_inputs.value).unwrap(); number_of_utxos].as_slice(),
        );
        let public_input_hash = big_int_to_string(&BigInt::from_bytes_be(
            num_bigint::Sign::Plus,
            &public_input_hash,
        ));
        (
            Self {
                circuit_type: "non-inclusion".to_string(),
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
                next_index: input.next_index.to_u32().unwrap(),
                leaf_lower_range_value: big_int_to_string(&input.leaf_lower_range_value),
                leaf_higher_range_value: big_int_to_string(&input.leaf_higher_range_value),
            };
            proof_inputs.push(prof_input);
        }

        Self {
            circuit_type: "non-inclusion".to_string(),
            public_input_hash: big_int_to_string(&inputs.public_input_hash),
            inputs: proof_inputs,
        }
    }
}

pub fn inclusion_inputs_string(number_of_utxos: usize) -> (String, NonInclusionMerkleProofInputs) {
    let (json_struct, public_inputs) =
        BatchNonInclusionJsonStruct::new_with_public_inputs(number_of_utxos);
    (json_struct.to_string(), public_inputs)
}

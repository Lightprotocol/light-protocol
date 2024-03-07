use serde::Serialize;
use circuitlib_rs::init_merkle_tree::{inclusion_merkle_tree_inputs, non_inclusion_merkle_tree_inputs_26};
use circuitlib_rs::merkle_tree_info::MerkleTreeInfo;
use crate::helpers::{create_json_from_struct, create_vec_of_string, create_vec_of_u32, create_vec_of_vec_of_string};

#[allow(non_snake_case)]
#[derive(Serialize)]
pub struct NonInclusionJsonStruct {
    root: Vec<String>,
    value: Vec<String>,
    leafLowerRangeValue: Vec<String>,
    leafHigherRangeValue: Vec<String>,
    leafIndex: Vec<u32>,
    inPathIndices: Vec<u32>,
    inPathElements: Vec<Vec<String>>,
}

impl NonInclusionJsonStruct {
    fn new(number_of_utxos: usize) -> Self {
        let merkle_inputs = non_inclusion_merkle_tree_inputs_26();
        let roots = create_vec_of_string(number_of_utxos, &merkle_inputs.root);
        let values = create_vec_of_string(number_of_utxos, &merkle_inputs.value);

        let lower_range_values = create_vec_of_string(number_of_utxos, &merkle_inputs.leaf_lower_range_value);
        let higher_range_values = create_vec_of_string(number_of_utxos, &merkle_inputs.leaf_higher_range_value);
        let leaf_indices = create_vec_of_u32(number_of_utxos, &merkle_inputs.leaf_index);

        let in_path_indices = create_vec_of_u32(number_of_utxos, &merkle_inputs.index_hashed_indexed_element_leaf);
        let in_path_elements =
            create_vec_of_vec_of_string(number_of_utxos, &merkle_inputs.merkle_proof_hashed_indexed_element_leaf);
        Self {
            root: roots,
            value: values,

            leafLowerRangeValue: lower_range_values,
            leafHigherRangeValue: higher_range_values,
            leafIndex: leaf_indices,

            inPathIndices: in_path_indices,
            inPathElements: in_path_elements,
        }
    }

    fn to_string(&self) -> String {
        create_json_from_struct(&self)
    }
}
pub fn non_inclusion_inputs_string(number_of_utxos: usize) -> String {
    NonInclusionJsonStruct::new(number_of_utxos).to_string()
}

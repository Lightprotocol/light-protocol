use serde::Serialize;

use crate::{
    gnark::helpers::{
        create_json_from_struct, create_vec_of_string, create_vec_of_u32,
        create_vec_of_vec_of_string,
    },
    init_merkle_tree::non_inclusion_merkle_tree_inputs_26,
    non_inclusion::merkle_non_inclusion_proof_inputs::{
        NonInclusionMerkleProofInputs, NonInclusionProofInputs,
    },
};

#[allow(non_snake_case)]
#[derive(Serialize)]
pub struct NonInclusionJsonStruct {
    roots: Vec<String>,
    values: Vec<String>,
    leafLowerRangeValues: Vec<String>,
    leafHigherRangeValues: Vec<String>,
    leafIndices: Vec<u32>,
    inPathIndices: Vec<u32>,
    inPathElements: Vec<Vec<String>>,
}

impl NonInclusionJsonStruct {
    fn new_with_public_inputs(number_of_utxos: usize) -> (Self, NonInclusionMerkleProofInputs) {
        let merkle_inputs = non_inclusion_merkle_tree_inputs_26();
        let roots = create_vec_of_string(number_of_utxos, &merkle_inputs.root);
        let values = create_vec_of_string(number_of_utxos, &merkle_inputs.value);
        let leaf_lower_range_values =
            create_vec_of_string(number_of_utxos, &merkle_inputs.leaf_lower_range_value);
        let leaf_higher_range_values =
            create_vec_of_string(number_of_utxos, &merkle_inputs.leaf_higher_range_value);
        let leaf_indices = create_vec_of_u32(number_of_utxos, &merkle_inputs.leaf_index);
        assert_eq!(
            merkle_inputs.leaf_index,
            merkle_inputs.index_hashed_indexed_element_leaf
        );
        let in_path_indices = create_vec_of_u32(
            number_of_utxos,
            &merkle_inputs.index_hashed_indexed_element_leaf,
        );
        let in_path_elements = create_vec_of_vec_of_string(
            number_of_utxos,
            &merkle_inputs.merkle_proof_hashed_indexed_element_leaf,
        );
        (
            Self {
                roots,
                values,
                leafLowerRangeValues: leaf_lower_range_values,
                leafHigherRangeValues: leaf_higher_range_values,
                leafIndices: leaf_indices,
                inPathIndices: in_path_indices,
                inPathElements: in_path_elements,
            },
            merkle_inputs,
        )
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        create_json_from_struct(&self)
    }

    pub fn from_non_inclusion_proof_inputs(inputs: &NonInclusionProofInputs) -> Self {
        let mut roots = Vec::new();
        let mut values = Vec::new();
        let mut leaf_lower_range_values = Vec::new();
        let mut leaf_higher_range_values = Vec::new();
        let mut leaf_indices = Vec::new();
        let mut in_path_indices = Vec::new();
        let mut in_path_elements = Vec::new();
        for input in inputs.0 {
            roots.push(format!("0x{}", input.root.to_str_radix(16)));
            values.push(format!("0x{}", input.value.to_str_radix(16)));
            leaf_lower_range_values.push(format!(
                "0x{}",
                input.leaf_lower_range_value.to_str_radix(16)
            ));
            leaf_higher_range_values.push(format!(
                "0x{}",
                input.leaf_higher_range_value.to_str_radix(16)
            ));
            leaf_indices.push(input.leaf_index.clone().try_into().unwrap());
            in_path_indices.push(
                input
                    .index_hashed_indexed_element_leaf
                    .clone()
                    .try_into()
                    .unwrap(),
            );
            in_path_elements.push(
                input
                    .merkle_proof_hashed_indexed_element_leaf
                    .iter()
                    .map(|x| format!("0x{}", x.to_str_radix(16)))
                    .collect(),
            );
        }

        Self {
            roots,
            values,
            leafLowerRangeValues: leaf_lower_range_values,
            leafHigherRangeValues: leaf_higher_range_values,
            leafIndices: leaf_indices,
            inPathIndices: in_path_indices,
            inPathElements: in_path_elements,
        }
    }
}

pub fn inclusion_inputs_string(number_of_utxos: usize) -> (String, NonInclusionMerkleProofInputs) {
    let (json_struct, public_inputs) =
        NonInclusionJsonStruct::new_with_public_inputs(number_of_utxos);
    (json_struct.to_string(), public_inputs)
}

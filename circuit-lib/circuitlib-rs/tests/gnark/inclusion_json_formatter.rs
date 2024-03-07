use serde::Serialize;
use circuitlib_rs::init_merkle_tree::inclusion_merkle_tree_inputs;
use circuitlib_rs::merkle_tree_info::MerkleTreeInfo;
use crate::helpers::{create_json_from_struct, create_vec_of_string, create_vec_of_u32, create_vec_of_vec_of_string};

#[allow(non_snake_case)]
#[derive(Serialize)]
pub struct InclusionJsonStruct {
    root: Vec<String>,
    leaf: Vec<String>,
    inPathIndices: Vec<u32>,
    inPathElements: Vec<Vec<String>>,
}

impl InclusionJsonStruct {
    fn new(number_of_utxos: usize) -> Self {
        let merkle_inputs = inclusion_merkle_tree_inputs(MerkleTreeInfo::H26);
        let roots = create_vec_of_string(number_of_utxos, &merkle_inputs.root);
        let leafs = create_vec_of_string(number_of_utxos, &merkle_inputs.leaf);
        let in_path_indices = create_vec_of_u32(number_of_utxos, &merkle_inputs.in_path_indices);
        let in_path_elements =
            create_vec_of_vec_of_string(number_of_utxos, &merkle_inputs.in_path_elements);
        Self {
            root: roots,
            leaf: leafs,
            inPathIndices: in_path_indices,
            inPathElements: in_path_elements,
        }
    }

    fn to_string(&self) -> String {
        create_json_from_struct(&self)
    }
}
pub fn inclusion_inputs_string(number_of_utxos: usize) -> String {
    InclusionJsonStruct::new(number_of_utxos).to_string()
}

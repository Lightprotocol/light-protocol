use serde::Serialize;

use crate::{
    gnark::helpers::{
        create_json_from_struct, create_vec_of_string, create_vec_of_u32,
        create_vec_of_vec_of_string,
    },
    inclusion::{
        merkle_inclusion_proof_inputs::{InclusionMerkleProofInputs, InclusionProofInputs},
        merkle_tree_info::MerkleTreeInfo,
    },
    init_merkle_tree::inclusion_merkle_tree_inputs,
};
#[allow(non_snake_case)]
#[derive(Serialize)]
pub struct InclusionJsonStruct {
    root: Vec<String>,
    leaf: Vec<String>,
    inPathIndices: Vec<u32>,
    inPathElements: Vec<Vec<String>>,
}

impl InclusionJsonStruct {
    fn new_with_public_inputs(number_of_utxos: usize) -> (Self, InclusionMerkleProofInputs) {
        let merkle_inputs = inclusion_merkle_tree_inputs(MerkleTreeInfo::H26);
        let roots = create_vec_of_string(number_of_utxos, &merkle_inputs.root);
        let leafs = create_vec_of_string(number_of_utxos, &merkle_inputs.leaf);
        let in_path_indices = create_vec_of_u32(number_of_utxos, &merkle_inputs.in_path_indices);
        let in_path_elements =
            create_vec_of_vec_of_string(number_of_utxos, &merkle_inputs.in_path_elements);
        (
            Self {
                root: roots,
                leaf: leafs,
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

    pub fn from_inclusion_proof_inputs(inputs: &InclusionProofInputs) -> Self {
        println!("from_inclusion_proof_inputs {:?}", inputs.0);
        let mut roots = Vec::new();
        let mut leaves = Vec::new();
        let mut in_path_indices = Vec::new();
        let mut in_path_elements = Vec::new();
        for input in inputs.0 {
            roots.push(format!("0x{}", input.root.to_str_radix(16)));
            leaves.push(format!("0x{}", input.leaf.to_str_radix(16)));
            in_path_indices.push(input.in_path_indices.clone().try_into().unwrap());
            in_path_elements.push(
                input
                    .in_path_elements
                    .iter()
                    .map(|x| format!("0x{}", x.to_str_radix(16)))
                    .collect(),
            );
        }

        Self {
            root: roots,
            leaf: leaves,
            inPathIndices: in_path_indices,
            inPathElements: in_path_elements,
        }
    }
}

pub fn inclusion_inputs_string(number_of_utxos: usize) -> (String, InclusionMerkleProofInputs) {
    let (json_struct, public_inputs) = InclusionJsonStruct::new_with_public_inputs(number_of_utxos);
    (json_struct.to_string(), public_inputs)
}

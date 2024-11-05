use crate::batch_append_with_subtrees::BatchAppendWithSubtreesCircuitInputs;
use crate::gnark::helpers::{big_int_to_string, create_json_from_struct};
use num_traits::ToPrimitive;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct BatchAppendWithSubtreesJsonStruct {
    #[serde(rename(serialize = "publicInputHash"))]
    pub public_input_hash: String,
    #[serde(rename(serialize = "oldSubTreeHashChain"))]
    pub old_sub_tree_hash_chain: String,
    #[serde(rename(serialize = "newSubTreeHashChain"))]
    pub new_sub_tree_hash_chain: String,
    #[serde(rename(serialize = "newRoot"))]
    pub new_root: String,
    #[serde(rename(serialize = "hashchainHash"))]
    pub hashchain_hash: String,
    #[serde(rename(serialize = "startIndex"))]
    pub start_index: u32,
    #[serde(rename(serialize = "treeHeight"))]
    pub tree_height: u32,
    #[serde(rename(serialize = "leaves"))]
    pub leaves: Vec<String>,
    #[serde(rename(serialize = "subtrees"))]
    pub subtrees: Vec<String>,
}

impl BatchAppendWithSubtreesJsonStruct {
    pub fn from_append_inputs(inputs: &BatchAppendWithSubtreesCircuitInputs) -> Self {
        let public_input_hash = big_int_to_string(&inputs.public_input_hash);
        let old_sub_tree_hash_chain = big_int_to_string(&inputs.old_sub_tree_hash_chain);
        let new_sub_tree_hash_chain = big_int_to_string(&inputs.new_sub_tree_hash_chain);
        let new_root = big_int_to_string(&inputs.new_root);
        let hashchain_hash = big_int_to_string(&inputs.hashchain_hash);
        let start_index = inputs.start_index.to_u32().unwrap();
        let tree_height = inputs.tree_height.to_u32().unwrap();

        let leaves = inputs
            .leaves
            .iter()
            .map(big_int_to_string)
            .collect::<Vec<String>>();

        let subtrees = inputs
            .subtrees
            .iter()
            .map(big_int_to_string)
            .collect::<Vec<String>>();

        Self {
            public_input_hash,
            old_sub_tree_hash_chain,
            new_sub_tree_hash_chain,
            new_root,
            hashchain_hash,
            start_index,
            tree_height,
            leaves,
            subtrees,
        }
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        create_json_from_struct(&self)
    }
}

pub fn append_inputs_string(inputs: &BatchAppendWithSubtreesCircuitInputs) -> String {
    let json_struct = BatchAppendWithSubtreesJsonStruct::from_append_inputs(inputs);
    json_struct.to_string()
}

pub fn new_with_append_inputs() -> (
    BatchAppendWithSubtreesJsonStruct,
    BatchAppendWithSubtreesCircuitInputs,
) {
    let append_inputs = BatchAppendWithSubtreesCircuitInputs::default();

    let json_struct = BatchAppendWithSubtreesJsonStruct {
        public_input_hash: big_int_to_string(&append_inputs.public_input_hash),
        old_sub_tree_hash_chain: big_int_to_string(&append_inputs.old_sub_tree_hash_chain),
        new_sub_tree_hash_chain: big_int_to_string(&append_inputs.new_sub_tree_hash_chain),
        new_root: big_int_to_string(&append_inputs.new_root),
        hashchain_hash: big_int_to_string(&append_inputs.hashchain_hash),
        start_index: append_inputs.start_index.to_u32().unwrap(),
        tree_height: append_inputs.tree_height.to_u32().unwrap(),
        leaves: append_inputs
            .leaves
            .iter()
            .map(big_int_to_string)
            .collect::<Vec<String>>(),
        subtrees: append_inputs
            .subtrees
            .iter()
            .map(big_int_to_string)
            .collect::<Vec<String>>(),
    };

    (json_struct, append_inputs)
}

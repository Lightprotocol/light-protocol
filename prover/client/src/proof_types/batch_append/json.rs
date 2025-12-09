use serde::Serialize;

use crate::{
    helpers::{big_int_to_string, create_json_from_struct},
    proof_types::{batch_append::BatchAppendsCircuitInputs, circuit_type::CircuitType},
};

#[derive(Debug, Clone, Serialize)]
pub struct BatchAppendInputsJson {
    #[serde(rename = "circuitType")]
    circuit_type: String,
    #[serde(rename = "publicInputHash")]
    public_input_hash: String,
    #[serde(rename = "oldRoot")]
    old_root: String,
    #[serde(rename = "newRoot")]
    new_root: String,
    #[serde(rename = "leavesHashchainHash")]
    leaves_hashchain_hash: String,
    #[serde(rename = "startIndex")]
    start_index: u32,
    #[serde(rename = "oldLeaves")]
    old_leaves: Vec<String>,
    #[serde(rename = "leaves")]
    leaves: Vec<String>,
    #[serde(rename = "merkleProofs")]
    merkle_proofs: Vec<Vec<String>>,
    #[serde(rename = "height")]
    height: u32,
    #[serde(rename = "batchSize")]
    batch_size: u32,
    /// Tree pubkey for fair queuing - used to prevent starvation when multiple trees have proofs pending
    #[serde(rename = "treeId", skip_serializing_if = "Option::is_none")]
    tree_id: Option<String>,
}

impl BatchAppendInputsJson {
    pub fn from_inputs(inputs: &BatchAppendsCircuitInputs) -> Self {
        Self {
            circuit_type: CircuitType::BatchAppend.to_string(),
            public_input_hash: big_int_to_string(&inputs.public_input_hash),
            old_root: big_int_to_string(&inputs.old_root),
            new_root: big_int_to_string(&inputs.new_root),
            leaves_hashchain_hash: big_int_to_string(&inputs.leaves_hashchain_hash),
            start_index: inputs.start_index,
            old_leaves: inputs.old_leaves.iter().map(big_int_to_string).collect(),
            leaves: inputs.leaves.iter().map(big_int_to_string).collect(),
            merkle_proofs: inputs
                .merkle_proofs
                .iter()
                .map(|proof| proof.iter().map(big_int_to_string).collect())
                .collect(),
            height: inputs.height,
            batch_size: inputs.batch_size,
            tree_id: None,
        }
    }

    /// Set the tree ID for fair queuing across multiple trees
    pub fn with_tree_id(mut self, tree_id: String) -> Self {
        self.tree_id = Some(tree_id);
        self
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        create_json_from_struct(&self)
    }
}

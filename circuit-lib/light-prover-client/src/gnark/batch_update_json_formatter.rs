use crate::{
    batch_update::BatchUpdateCircuitInputs,
    gnark::helpers::{big_int_to_string, create_json_from_struct},
};
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct BatchUpdateProofInputsJson {
    #[serde(rename(serialize = "publicInputHash"))]
    pub public_input_hash: String,
    #[serde(rename(serialize = "oldRoot"))]
    pub old_root: String,
    #[serde(rename(serialize = "newRoot"))]
    pub new_root: String,
    #[serde(rename(serialize = "leavesHashchainHash"))]
    pub leaves_hashchain_hash: String,
    #[serde(rename(serialize = "leaves"))]
    pub leaves: Vec<String>,
    #[serde(rename(serialize = "newMerkleProofs"))]
    pub merkle_proofs: Vec<Vec<String>>,
    #[serde(rename(serialize = "pathIndices"))]
    pub path_indices: Vec<u32>,
    #[serde(rename(serialize = "height"))]
    pub height: u32,
    #[serde(rename(serialize = "batchSize"))]
    pub batch_size: u32,
}

#[derive(Serialize, Debug)]
pub struct BatchUpdateParametersJson {
    #[serde(rename(serialize = "batch-update-inputs"))]
    pub inputs: BatchUpdateProofInputsJson,
}

impl BatchUpdateProofInputsJson {
    pub fn from_update_inputs(inputs: &BatchUpdateCircuitInputs) -> Self {
        let public_input_hash = big_int_to_string(&inputs.public_input_hash);
        let old_root = big_int_to_string(&inputs.old_root);
        let new_root = big_int_to_string(&inputs.new_root);
        let leaves_hashchain_hash = big_int_to_string(&inputs.leaves_hashchain_hash);

        let leaves = inputs
            .leaves
            .iter()
            .map(big_int_to_string)
            .collect::<Vec<String>>();

        let merkle_proofs = inputs
            .merkle_proofs
            .iter()
            .map(|proof| proof.iter().map(big_int_to_string).collect())
            .collect();

        let path_indices = inputs.path_indices.clone();
        let height = inputs.height;
        let batch_size = inputs.batch_size;

        Self {
            public_input_hash,
            old_root,
            new_root,
            leaves_hashchain_hash,
            leaves,
            merkle_proofs,
            path_indices,
            height,
            batch_size,
        }
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        create_json_from_struct(&self)
    }
}

pub fn update_inputs_string(inputs: &BatchUpdateCircuitInputs) -> String {
    let json_struct = BatchUpdateProofInputsJson::from_update_inputs(inputs);
    json_struct.to_string()
}

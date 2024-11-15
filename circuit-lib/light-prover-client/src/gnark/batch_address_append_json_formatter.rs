use serde::Serialize;
use crate::gnark::helpers::{big_uint_to_string, create_json_from_struct};
use crate::batch_address_append::BatchAddressAppendInputs;

#[derive(Debug, Clone, Serialize)]
pub struct BatchAddressAppendInputsJson {
    #[serde(rename = "BatchSize")]
    pub batch_size: usize,
    #[serde(rename = "HashchainHash")]
    pub hashchain_hash: String,
    #[serde(rename = "LowElementValues")]
    pub low_element_values: Vec<String>,
    #[serde(rename = "LowElementIndices")]
    pub low_element_indices: Vec<String>,
    #[serde(rename = "LowElementNextIndices")]
    pub low_element_next_indices: Vec<String>,
    #[serde(rename = "LowElementNextValues")]
    pub low_element_next_values: Vec<String>,
    #[serde(rename = "LowElementProofs")]
    pub low_element_proofs: Vec<Vec<String>>,
    #[serde(rename = "NewElementValues")]
    pub new_element_values: Vec<String>,
    #[serde(rename = "NewElementProofs")]
    pub new_element_proofs: Vec<Vec<String>>,
    #[serde(rename = "NewRoot")]
    pub new_root: String,
    #[serde(rename = "OldRoot")]
    pub old_root: String,
    #[serde(rename = "PublicInputHash")]
    pub public_input_hash: String,
    #[serde(rename = "StartIndex")]
    pub start_index: usize,
    #[serde(rename = "TreeHeight")]
    pub tree_height: usize,
}

impl BatchAddressAppendInputsJson {
    pub fn from_inputs(inputs: &BatchAddressAppendInputs) -> Self {
        Self {
            batch_size: inputs.batch_size,
            hashchain_hash: big_uint_to_string(&inputs.hashchain_hash),
            low_element_values: inputs.low_element_values.iter().map(big_uint_to_string).collect(),
            low_element_indices: inputs.low_element_indices.iter().map(big_uint_to_string).collect(),
            low_element_next_indices: inputs.low_element_next_indices.iter().map(big_uint_to_string).collect(),
            low_element_next_values: inputs.low_element_next_values.iter().map(big_uint_to_string).collect(),
            low_element_proofs: inputs
                .low_element_proofs
                .iter()
                .map(|proof| proof.iter().map(big_uint_to_string).collect())
                .collect(),
            new_element_values: inputs.new_element_values.iter().map(big_uint_to_string).collect(),
            new_element_proofs: inputs
                .new_element_proofs
                .iter()
                .map(|proof| proof.iter().map(big_uint_to_string).collect())
                .collect(),
            new_root: big_uint_to_string(&inputs.new_root),
            old_root: big_uint_to_string(&inputs.old_root),
            public_input_hash: big_uint_to_string(&inputs.public_input_hash),
            start_index: inputs.start_index,
            tree_height: inputs.tree_height,
        }
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        create_json_from_struct(&self)
    }
}

pub fn to_json(inputs: &BatchAddressAppendInputs) -> String {
    let json_struct = BatchAddressAppendInputsJson::from_inputs(inputs);
    json_struct.to_string()
}
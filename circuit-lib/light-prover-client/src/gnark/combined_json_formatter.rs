use light_batched_merkle_tree::constants::{
    DEFAULT_BATCH_ADDRESS_TREE_HEIGHT, DEFAULT_BATCH_STATE_TREE_HEIGHT,
};
use light_utils::hashchain::create_hash_chain_from_array;
use num_bigint::BigInt;
use serde::Serialize;

use super::{
    helpers::{big_int_to_string, create_json_from_struct},
    inclusion_json_formatter::InclusionJsonStruct,
    non_inclusion_json_formatter::NonInclusionJsonStruct,
};
use crate::{
    combined::merkle_combined_proof_inputs::CombinedProofInputs,
    errors::ProverClientError,
    gnark::{
        inclusion_json_formatter::BatchInclusionJsonStruct,
        non_inclusion_json_formatter::BatchNonInclusionJsonStruct,
    },
    prove_utils::CircuitType,
};

#[derive(Serialize, Debug)]
pub struct CombinedJsonStruct {
    #[serde(rename = "circuitType")]
    pub circuit_type: String,
    #[serde(rename = "stateTreeHeight")]
    pub state_tree_height: u32,
    #[serde(rename = "addressTreeHeight")]
    pub address_tree_height: u32,
    #[serde(rename = "publicInputHash")]
    pub public_input_hash: String,
    #[serde(rename(serialize = "inputCompressedAccounts"))]
    pub inclusion: Vec<InclusionJsonStruct>,

    #[serde(rename(serialize = "newAddresses"))]
    pub non_inclusion: Vec<NonInclusionJsonStruct>,
}

impl CombinedJsonStruct {
    fn new_with_public_inputs(
        num_inclusion: usize,
        num_non_inclusion: usize,
    ) -> Result<Self, ProverClientError> {
        let (inclusion, inclusion_public_input_hash) =
            BatchInclusionJsonStruct::new_with_public_inputs(num_inclusion);
        let (non_inclusion, non_inclusion_public_input_hash) =
            BatchNonInclusionJsonStruct::new_with_public_inputs(num_non_inclusion)?;

        let public_inputs_hash = create_hash_chain_from_array([
            inclusion_public_input_hash,
            non_inclusion_public_input_hash,
        ])?;

        Ok(Self {
            circuit_type: CircuitType::Combined.to_string(),
            state_tree_height: DEFAULT_BATCH_STATE_TREE_HEIGHT,
            address_tree_height: DEFAULT_BATCH_ADDRESS_TREE_HEIGHT,
            public_input_hash: big_int_to_string(&BigInt::from_bytes_be(
                num_bigint::Sign::Plus,
                public_inputs_hash.as_slice(),
            )),
            inclusion: inclusion.inputs,
            non_inclusion: non_inclusion.inputs,
        })
    }

    pub fn from_combined_inputs(inputs: &CombinedProofInputs) -> Self {
        let inclusion_parameters =
            BatchInclusionJsonStruct::from_inclusion_proof_inputs(&inputs.inclusion_parameters);
        let non_inclusion_parameters = BatchNonInclusionJsonStruct::from_non_inclusion_proof_inputs(
            &inputs.non_inclusion_parameters,
        );

        Self {
            circuit_type: CircuitType::Combined.to_string(),
            state_tree_height: DEFAULT_BATCH_STATE_TREE_HEIGHT,
            address_tree_height: DEFAULT_BATCH_ADDRESS_TREE_HEIGHT,
            public_input_hash: big_int_to_string(&inputs.public_input_hash),
            inclusion: inclusion_parameters.inputs,
            non_inclusion: non_inclusion_parameters.inputs,
        }
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        create_json_from_struct(&self)
    }
}

pub fn combined_inputs_string(num_inclusion: usize, num_non_inclusion: usize) -> String {
    let json_struct = CombinedJsonStruct::new_with_public_inputs(num_inclusion, num_non_inclusion);
    json_struct.unwrap().to_string()
}

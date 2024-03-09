use std::{collections::HashMap, fs::File};

use ark_circom::{circom::Inputs, read_zkey};

use crate::{
    arkworks_prover::{prove, ArkProvingKey},
    errors::CircuitsError,
    inclusion::{
        merkle_inclusion_proof_inputs::{InclusionMerkleProofInputs, InclusionProofInputs},
        merkle_tree_info::MerkleTreeInfo,
    },
    prove_utils::ProofResult,
};

pub fn test_merkle_inclusion_proof(
    merkle_tree_info: &MerkleTreeInfo,
    merkle_proof_inputs: &[InclusionMerkleProofInputs],
) -> Result<ProofResult, CircuitsError> {
    let merkle_proof_inputs_len = merkle_proof_inputs.len();

    let proof_inputs = InclusionProofInputs(merkle_proof_inputs);
    let public_inputs = proof_inputs.public_inputs();
    let inputs_hashmap: HashMap<String, Inputs> = proof_inputs
        .try_into()
        .map_err(|_| CircuitsError::ChangeEndiannessError)?;

    let zk_path = merkle_tree_info.test_zk_path(merkle_proof_inputs_len);
    let mut file = File::open(zk_path).unwrap();
    let pk: ArkProvingKey = read_zkey(&mut file).unwrap();
    let wasm_path = merkle_tree_info.test_wasm_path(merkle_proof_inputs_len);
    let proof = prove(
        merkle_tree_info.height(),
        merkle_proof_inputs_len,
        inputs_hashmap,
        &pk,
        &wasm_path,
    )?;

    let proof = crate::prove_utils::convert_arkworks_proof_to_solana_groth16(&proof)?;
    Ok(ProofResult {
        proof,
        public_inputs,
    })
}

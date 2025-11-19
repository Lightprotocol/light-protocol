use anyhow::Result;
use light_batched_merkle_tree::merkle_tree::{
    InstructionDataAddressAppendInputs, InstructionDataBatchAppendInputs,
    InstructionDataBatchNullifyInputs,
};
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_hasher::bigint::bigint_to_be_bytes_array;
use light_prover_client::{
    errors::ProverClientError,
    proof::ProofCompressed,
    proof_types::{
        batch_address_append::BatchAddressAppendInputs,
        batch_append::BatchAppendsCircuitInputs, batch_update::BatchUpdateCircuitInputs,
    },
};

/// Converts batch append circuit inputs and proof into instruction data.
pub fn create_append_proof_result(
    circuit_inputs: &BatchAppendsCircuitInputs,
    proof: ProofCompressed,
) -> Result<InstructionDataBatchAppendInputs> {
    let big_uint = circuit_inputs
        .new_root
        .to_biguint()
        .ok_or_else(|| ProverClientError::GenericError("Failed to convert new_root to BigUint".to_string()))?;

    let new_root = bigint_to_be_bytes_array::<32>(&big_uint)
        .map_err(|e| ProverClientError::GenericError(format!("Failed to convert new_root to bytes: {}", e)))?;

    Ok(InstructionDataBatchAppendInputs {
        new_root,
        compressed_proof: CompressedProof {
            a: proof.a,
            b: proof.b,
            c: proof.c,
        },
    })
}

/// Converts batch update (nullify) circuit inputs and proof into instruction data.
pub fn create_nullify_proof_result(
    circuit_inputs: &BatchUpdateCircuitInputs,
    proof: ProofCompressed,
) -> Result<InstructionDataBatchNullifyInputs> {
    let big_uint = circuit_inputs
        .new_root
        .to_biguint()
        .ok_or_else(|| ProverClientError::GenericError("Failed to convert new_root to BigUint".to_string()))?;

    let new_root = bigint_to_be_bytes_array::<32>(&big_uint)
        .map_err(|e| ProverClientError::GenericError(format!("Failed to convert new_root to bytes: {}", e)))?;

    Ok(InstructionDataBatchNullifyInputs {
        new_root,
        compressed_proof: CompressedProof {
            a: proof.a,
            b: proof.b,
            c: proof.c,
        },
    })
}

/// Converts batch address append circuit inputs and proof into instruction data.
pub fn create_address_proof_result(
    circuit_inputs: &BatchAddressAppendInputs,
    proof: ProofCompressed,
) -> Result<InstructionDataAddressAppendInputs> {
    let new_root = bigint_to_be_bytes_array::<32>(&circuit_inputs.new_root)
        .map_err(|e| ProverClientError::GenericError(format!("Failed to convert new_root to bytes: {}", e)))?;

    Ok(InstructionDataAddressAppendInputs {
        new_root,
        compressed_proof: CompressedProof {
            a: proof.a,
            b: proof.b,
            c: proof.c,
        },
    })
}

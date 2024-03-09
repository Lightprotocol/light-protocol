use groth16_solana::groth16::{Groth16Verifier, Groth16Verifyingkey};

use crate::{
    arkworks_prover::ArkProof,
    errors::CircuitsError,
    prove_utils::{convert_arkworks_proof_to_solana_groth16, ProofCompressed},
};

pub fn groth16_solana_verify<const NR_INPUTS: usize>(
    proof: &ProofCompressed,
    proof_inputs: &[[u8; 32]; NR_INPUTS],
    verifyingkey: Groth16Verifyingkey,
) -> Result<bool, CircuitsError> {
    let proof = proof.try_decompress()?;
    let mut verifier =
        Groth16Verifier::new(&proof.a, &proof.b, &proof.c, proof_inputs, &verifyingkey)?;
    let result = verifier.verify()?;
    Ok(result)
}

pub fn groth16_solana_verify_arkworks_proof<const NR_INPUTS: usize>(
    proof: &ArkProof,
    proof_inputs: &[[u8; 32]; NR_INPUTS],
    verifyingkey: Groth16Verifyingkey,
) -> Result<bool, CircuitsError> {
    let proof = convert_arkworks_proof_to_solana_groth16(proof)?.try_decompress()?;
    let mut verifier =
        Groth16Verifier::new(&proof.a, &proof.b, &proof.c, proof_inputs, &verifyingkey)?;
    let result = verifier.verify()?;
    Ok(result)
}

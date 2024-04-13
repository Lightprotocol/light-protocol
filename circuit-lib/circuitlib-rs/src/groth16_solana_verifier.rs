use groth16_solana::groth16::{Groth16Verifier, Groth16Verifyingkey};

use crate::{errors::CircuitsError, prove_utils::ProofCompressed};

// TODO: move to groth16_solana ?
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

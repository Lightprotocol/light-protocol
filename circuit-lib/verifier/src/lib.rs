use crate::verifying_keys::*;
use borsh::{BorshDeserialize, BorshSerialize};
use groth16_solana::decompression::{decompress_g1, decompress_g2};
use groth16_solana::groth16::{Groth16Verifier, Groth16Verifyingkey};
use thiserror::Error;

pub mod verifying_keys;
#[derive(Debug, Error)]
pub enum VerifierError {
    #[error("PublicInputsTryIntoFailed")]
    PublicInputsTryIntoFailed,
    #[error("DecompressG1Failed")]
    DecompressG1Failed,
    #[error("DecompressG2Failed")]
    DecompressG2Failed,
    #[error("InvalidPublicInputsLength")]
    InvalidPublicInputsLength,
    #[error("CreateGroth16VerifierFailed")]
    CreateGroth16VerifierFailed,
    #[error("ProofVerificationFailed")]
    ProofVerificationFailed,
    #[error("InvalidBatchSize supported batch sizes are 1, 10, 100, 500, 1000")]
    InvalidBatchSize,
}

#[cfg(feature = "solana")]
impl From<VerifierError> for u32 {
    fn from(e: VerifierError) -> u32 {
        match e {
            VerifierError::PublicInputsTryIntoFailed => 13001,
            VerifierError::DecompressG1Failed => 13002,
            VerifierError::DecompressG2Failed => 13003,
            VerifierError::InvalidPublicInputsLength => 13004,
            VerifierError::CreateGroth16VerifierFailed => 13005,
            VerifierError::ProofVerificationFailed => 13006,
            VerifierError::InvalidBatchSize => 13007,
        }
    }
}

#[cfg(feature = "solana")]
impl From<VerifierError> for solana_program::program_error::ProgramError {
    fn from(e: VerifierError) -> Self {
        solana_program::program_error::ProgramError::Custom(e.into())
    }
}

use VerifierError::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct CompressedProof {
    pub a: [u8; 32],
    pub b: [u8; 64],
    pub c: [u8; 32],
}

impl Default for CompressedProof {
    fn default() -> Self {
        Self {
            a: [0; 32],
            b: [0; 64],
            c: [0; 32],
        }
    }
}

pub fn select_verifying_key<'a>(
    num_leaves: usize,
    num_addresses: usize,
) -> Result<&'a Groth16Verifyingkey<'static>, VerifierError> {
    match (num_leaves, num_addresses) {
        // Combined cases (depend on both num_leaves and num_addresses)
        (1, 1) => Ok(&combined_26_40_1_1::VERIFYINGKEY),
        (1, 2) => Ok(&combined_26_40_1_2::VERIFYINGKEY),
        (1, 3) => Ok(&combined_26_40_1_3::VERIFYINGKEY),
        (1, 4) => Ok(&combined_26_40_1_4::VERIFYINGKEY),
        (2, 1) => Ok(&combined_26_40_2_1::VERIFYINGKEY),
        (2, 2) => Ok(&combined_26_40_2_2::VERIFYINGKEY),
        (2, 3) => Ok(&combined_26_40_2_3::VERIFYINGKEY),
        (2, 4) => Ok(&combined_26_40_2_4::VERIFYINGKEY),
        (3, 1) => Ok(&combined_26_40_3_1::VERIFYINGKEY),
        (3, 2) => Ok(&combined_26_40_3_2::VERIFYINGKEY),
        (3, 3) => Ok(&combined_26_40_3_3::VERIFYINGKEY),
        (3, 4) => Ok(&combined_26_40_3_4::VERIFYINGKEY),
        (4, 1) => Ok(&combined_26_40_4_1::VERIFYINGKEY),
        (4, 2) => Ok(&combined_26_40_4_2::VERIFYINGKEY),
        (4, 3) => Ok(&combined_26_40_4_3::VERIFYINGKEY),
        (4, 4) => Ok(&combined_26_40_4_4::VERIFYINGKEY),

        // Inclusion cases (depend on num_leaves)
        (1, _) => Ok(&inclusion_26_1::VERIFYINGKEY),
        (2, _) => Ok(&inclusion_26_2::VERIFYINGKEY),
        (3, _) => Ok(&inclusion_26_3::VERIFYINGKEY),
        (4, _) => Ok(&inclusion_26_4::VERIFYINGKEY),
        (8, _) => Ok(&inclusion_26_8::VERIFYINGKEY),

        // Non-inclusion cases (depend on num_addresses)
        (_, 1) => Ok(&non_inclusion_40_1::VERIFYINGKEY),
        (_, 2) => Ok(&non_inclusion_40_2::VERIFYINGKEY),
        (_, 3) => Ok(&non_inclusion_40_3::VERIFYINGKEY),
        (_, 4) => Ok(&non_inclusion_40_4::VERIFYINGKEY),
        (_, 8) => Ok(&non_inclusion_40_8::VERIFYINGKEY),

        // Invalid configuration
        _ => Err(VerifierError::InvalidPublicInputsLength),
    }
}

#[inline(never)]
pub fn verify<const N: usize>(
    public_inputs: &[[u8; 32]; N],
    proof: &CompressedProof,
    vk: &Groth16Verifyingkey,
) -> Result<(), VerifierError> {
    let proof_a = decompress_g1(&proof.a).map_err(|_| crate::DecompressG1Failed)?;
    let proof_b = decompress_g2(&proof.b).map_err(|_| crate::DecompressG2Failed)?;
    let proof_c = decompress_g1(&proof.c).map_err(|_| crate::DecompressG1Failed)?;
    let mut verifier = Groth16Verifier::new(&proof_a, &proof_b, &proof_c, public_inputs, vk)
        .map_err(|_| {
            #[cfg(target_os = "solana")]
            {
                use solana_program::msg;
                msg!("Proof verification failed");
                msg!("Public inputs: {:?}", public_inputs);
                msg!("Proof A: {:?}", proof_a);
                msg!("Proof B: {:?}", proof_b);
                msg!("Proof C: {:?}", proof_c);
            }
            CreateGroth16VerifierFailed
        })?;
    verifier.verify().map_err(|_| {
        #[cfg(target_os = "solana")]
        {
            use solana_program::msg;
            msg!("Proof verification failed");
            msg!("Public inputs: {:?}", public_inputs);
            msg!("Proof A: {:?}", proof_a);
            msg!("Proof B: {:?}", proof_b);
            msg!("Proof C: {:?}", proof_c);
        }
        ProofVerificationFailed
    })?;
    Ok(())
}

#[inline(never)]
pub fn verify_batch_append_with_subtrees(
    batch_size: usize,
    public_input_hash: [u8; 32],
    compressed_proof: &CompressedProof,
) -> Result<(), VerifierError> {
    match batch_size {
        1 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &append_with_subtrees_26_1::VERIFYINGKEY,
        ),
        10 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &append_with_subtrees_26_10::VERIFYINGKEY,
        ),
        100 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &append_with_subtrees_26_100::VERIFYINGKEY,
        ),
        500 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &append_with_subtrees_26_500::VERIFYINGKEY,
        ),
        1000 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &append_with_subtrees_26_1000::VERIFYINGKEY,
        ),
        _ => Err(crate::InvalidPublicInputsLength),
    }
}

#[inline(never)]
pub fn verify_batch_append_with_proofs(
    batch_size: usize,
    public_input_hash: [u8; 32],
    compressed_proof: &CompressedProof,
) -> Result<(), VerifierError> {
    match batch_size {
        1 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &append_with_proofs_26_1::VERIFYINGKEY,
        ),
        10 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &append_with_proofs_26_10::VERIFYINGKEY,
        ),
        100 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &append_with_proofs_26_100::VERIFYINGKEY,
        ),
        500 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &append_with_proofs_26_500::VERIFYINGKEY,
        ),
        1000 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &append_with_proofs_26_1000::VERIFYINGKEY,
        ),
        _ => Err(crate::InvalidPublicInputsLength),
    }
}

#[inline(never)]
pub fn verify_batch_update(
    batch_size: usize,
    public_input_hash: [u8; 32],
    compressed_proof: &CompressedProof,
) -> Result<(), VerifierError> {
    match batch_size {
        1 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &update_26_1::VERIFYINGKEY,
        ),
        10 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &update_26_10::VERIFYINGKEY,
        ),
        100 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &update_26_100::VERIFYINGKEY,
        ),
        500 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &update_26_500::VERIFYINGKEY,
        ),
        1000 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &update_26_1000::VERIFYINGKEY,
        ),
        _ => Err(crate::InvalidPublicInputsLength),
    }
}

#[inline(never)]
pub fn verify_batch_address_update(
    batch_size: usize,
    public_input_hash: [u8; 32],
    compressed_proof: &CompressedProof,
) -> Result<(), VerifierError> {
    match batch_size {
        1 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &crate::verifying_keys::address_append_40_1::VERIFYINGKEY,
        ),
        10 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &crate::verifying_keys::address_append_40_10::VERIFYINGKEY,
        ),
        100 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &crate::verifying_keys::address_append_40_100::VERIFYINGKEY,
        ),
        500 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &crate::verifying_keys::address_append_40_500::VERIFYINGKEY,
        ),
        1000 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &crate::verifying_keys::address_append_40_1000::VERIFYINGKEY,
        ),
        _ => Err(crate::InvalidPublicInputsLength),
    }
}

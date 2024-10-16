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
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Copy)]
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

pub fn verify_create_addresses_zkp(
    address_roots: &[[u8; 32]],
    addresses: &[[u8; 32]],
    compressed_proof: &CompressedProof,
) -> Result<(), VerifierError> {
    let public_inputs = [address_roots, addresses].concat();

    match addresses.len() {
        1 => verify::<2>(
            &public_inputs
                .try_into()
                .map_err(|_| PublicInputsTryIntoFailed)?,
            compressed_proof,
            &crate::verifying_keys::non_inclusion_26_1::VERIFYINGKEY,
        ),
        2 => verify::<4>(
            &public_inputs
                .try_into()
                .map_err(|_| PublicInputsTryIntoFailed)?,
            compressed_proof,
            &crate::verifying_keys::non_inclusion_26_2::VERIFYINGKEY,
        ),
        _ => Err(InvalidPublicInputsLength),
    }
}

#[inline(never)]
pub fn verify_create_addresses_and_merkle_proof_zkp(
    roots: &[[u8; 32]],
    leaves: &[[u8; 32]],
    address_roots: &[[u8; 32]],
    addresses: &[[u8; 32]],
    compressed_proof: &CompressedProof,
) -> Result<(), VerifierError> {
    let public_inputs = [roots, leaves, address_roots, addresses].concat();
    // The public inputs are expected to be a multiple of 2
    // 4 inputs means 1 inclusion proof (1 root, 1 leaf, 1 address root, 1 created address)
    // 6 inputs means 1 inclusion proof (1 root, 1 leaf, 2 address roots, 2 created address) or
    // 6 inputs means 2 inclusion proofs (2 roots and 2 leaves, 1 address root, 1 created address)
    // 8 inputs means 2 inclusion proofs (2 roots and 2 leaves, 2 address roots, 2 created address) or
    // 8 inputs means 3 inclusion proofs (3 roots and 3 leaves, 1 address root, 1 created address)
    // 10 inputs means 3 inclusion proofs (3 roots and 3 leaves, 2 address roots, 2 created address) or
    // 10 inputs means 4 inclusion proofs (4 roots and 4 leaves, 1 address root, 1 created address)
    // 12 inputs means 4 inclusion proofs (4 roots and 4 leaves, 2 address roots, 2 created address)
    match public_inputs.len() {
        4 => verify::<4>(
            &public_inputs
                .try_into()
                .map_err(|_| PublicInputsTryIntoFailed)?,
            compressed_proof,
            &crate::verifying_keys::combined_26_1_1::VERIFYINGKEY,
        ),
        6 => {
            let verifying_key = if address_roots.len() == 1 {
                &crate::verifying_keys::combined_26_2_1::VERIFYINGKEY
            } else {
                &crate::verifying_keys::combined_26_1_2::VERIFYINGKEY
            };
            verify::<6>(
                &public_inputs
                    .try_into()
                    .map_err(|_| PublicInputsTryIntoFailed)?,
                compressed_proof,
                verifying_key,
            )
        }
        8 => {
            let verifying_key = if address_roots.len() == 1 {
                &crate::verifying_keys::combined_26_3_1::VERIFYINGKEY
            } else {
                &crate::verifying_keys::combined_26_2_2::VERIFYINGKEY
            };
            verify::<8>(
                &public_inputs
                    .try_into()
                    .map_err(|_| PublicInputsTryIntoFailed)?,
                compressed_proof,
                verifying_key,
            )
        }
        10 => {
            let verifying_key = if address_roots.len() == 1 {
                &crate::verifying_keys::combined_26_4_1::VERIFYINGKEY
            } else {
                &crate::verifying_keys::combined_26_3_2::VERIFYINGKEY
            };
            verify::<10>(
                &public_inputs
                    .try_into()
                    .map_err(|_| PublicInputsTryIntoFailed)?,
                compressed_proof,
                verifying_key,
            )
        }
        12 => verify::<12>(
            &public_inputs
                .try_into()
                .map_err(|_| PublicInputsTryIntoFailed)?,
            compressed_proof,
            &crate::verifying_keys::combined_26_4_2::VERIFYINGKEY,
        ),
        _ => Err(crate::InvalidPublicInputsLength),
    }
}

#[inline(never)]
pub fn verify_merkle_proof_zkp(
    roots: &[[u8; 32]],
    leaves: &[[u8; 32]],
    compressed_proof: &CompressedProof,
) -> Result<(), VerifierError> {
    let public_inputs = [roots, leaves].concat();

    // The public inputs are expected to be a multiple of 2
    // 2 inputs means 1 inclusion proof (1 root and 1 leaf)
    // 4 inputs means 2 inclusion proofs (2 roots and 2 leaves)
    // 6 inputs means 3 inclusion proofs (3 roots and 3 leaves)
    // 8 inputs means 4 inclusion proofs (4 roots and 4 leaves)
    // 16 inputs means 8 inclusion proofs (8 roots and 8 leaves)
    match public_inputs.len() {
        2 => verify::<2>(
            &public_inputs
                .try_into()
                .map_err(|_| PublicInputsTryIntoFailed)?,
            compressed_proof,
            &crate::verifying_keys::inclusion_26_1::VERIFYINGKEY,
        ),
        4 => verify::<4>(
            &public_inputs
                .try_into()
                .map_err(|_| PublicInputsTryIntoFailed)?,
            compressed_proof,
            &crate::verifying_keys::inclusion_26_2::VERIFYINGKEY,
        ),
        6 => verify::<6>(
            &public_inputs
                .try_into()
                .map_err(|_| PublicInputsTryIntoFailed)?,
            compressed_proof,
            &crate::verifying_keys::inclusion_26_3::VERIFYINGKEY,
        ),
        8 => verify::<8>(
            &public_inputs
                .try_into()
                .map_err(|_| PublicInputsTryIntoFailed)?,
            compressed_proof,
            &crate::verifying_keys::inclusion_26_4::VERIFYINGKEY,
        ),
        16 => verify::<16>(
            &public_inputs
                .try_into()
                .map_err(|_| PublicInputsTryIntoFailed)?,
            compressed_proof,
            &crate::verifying_keys::inclusion_26_8::VERIFYINGKEY,
        ),
        _ => Err(crate::InvalidPublicInputsLength),
    }
}

#[inline(never)]
fn verify<const N: usize>(
    public_inputs: &[[u8; 32]; N],
    proof: &CompressedProof,
    vk: &Groth16Verifyingkey,
) -> Result<(), VerifierError> {
    let proof_a = decompress_g1(&proof.a).map_err(|_| crate::DecompressG1Failed)?;
    let proof_b = decompress_g2(&proof.b).map_err(|_| crate::DecompressG2Failed)?;
    let proof_c = decompress_g1(&proof.c).map_err(|_| crate::DecompressG1Failed)?;
    let mut verifier = Groth16Verifier::new(&proof_a, &proof_b, &proof_c, public_inputs, vk)
        .map_err(|_| CreateGroth16VerifierFailed)?;
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
pub fn verify_batch_append(
    batch_size: usize,
    public_input_hash: [u8; 32],
    compressed_proof: &CompressedProof,
) -> Result<(), VerifierError> {
    match batch_size {
        1 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &crate::verifying_keys::append_26_1::VERIFYINGKEY,
        ),
        10 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &crate::verifying_keys::append_26_10::VERIFYINGKEY,
        ),
        100 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &crate::verifying_keys::append_26_100::VERIFYINGKEY,
        ),
        500 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &crate::verifying_keys::append_26_500::VERIFYINGKEY,
        ),
        1000 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &crate::verifying_keys::append_26_1000::VERIFYINGKEY,
        ),
        _ => Err(crate::InvalidBatchSize),
    }
}

#[inline(never)]
pub fn verify_batch_append2(
    batch_size: usize,
    public_input_hash: [u8; 32],
    compressed_proof: &CompressedProof,
) -> Result<(), VerifierError> {
    match batch_size {
        1 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &crate::verifying_keys::append2_26_1::VERIFYINGKEY,
        ),
        10 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &crate::verifying_keys::append2_26_10::VERIFYINGKEY,
        ),
        // 100 => verify::<1>(
        //     &[public_input_hash],
        //     compressed_proof,
        //     &crate::verifying_keys::append_26_100::VERIFYINGKEY,
        // ),
        // 500 => verify::<1>(
        //     &[public_input_hash],
        //     compressed_proof,
        //     &crate::verifying_keys::append_26_500::VERIFYINGKEY,
        // ),
        // 1000 => verify::<1>(
        //     &[public_input_hash],
        //     compressed_proof,
        //     &crate::verifying_keys::append_26_1000::VERIFYINGKEY,
        // ),
        _ => Err(crate::InvalidBatchSize),
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
            &crate::verifying_keys::update_26_1::VERIFYINGKEY,
        ),
        10 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &crate::verifying_keys::update_26_10::VERIFYINGKEY,
        ),
        100 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &crate::verifying_keys::update_26_100::VERIFYINGKEY,
        ),
        500 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &crate::verifying_keys::update_26_500::VERIFYINGKEY,
        ),
        1000 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &crate::verifying_keys::update_26_1000::VERIFYINGKEY,
        ),
        _ => Err(crate::InvalidBatchSize),
    }
}

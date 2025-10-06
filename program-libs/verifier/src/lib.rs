use groth16_solana::{
    decompression::{decompress_g1, decompress_g2},
    groth16::{Groth16Verifier, Groth16Verifyingkey},
};
use thiserror::Error;

use crate::verifying_keys::*;

pub mod verifying_keys;
#[derive(Debug, Error, PartialEq)]
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

impl From<VerifierError> for u32 {
    fn from(e: VerifierError) -> u32 {
        match e {
            PublicInputsTryIntoFailed => 13001,
            DecompressG1Failed => 13002,
            DecompressG2Failed => 13003,
            InvalidPublicInputsLength => 13004,
            CreateGroth16VerifierFailed => 13005,
            ProofVerificationFailed => 13006,
            InvalidBatchSize => 13007,
        }
    }
}

#[cfg(feature = "solana")]
impl From<VerifierError> for solana_program_error::ProgramError {
    fn from(e: VerifierError) -> Self {
        solana_program_error::ProgramError::Custom(e.into())
    }
}

#[cfg(feature = "pinocchio")]
impl From<VerifierError> for pinocchio::program_error::ProgramError {
    fn from(e: VerifierError) -> Self {
        pinocchio::program_error::ProgramError::Custom(e.into())
    }
}

pub use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use VerifierError::*;

pub fn verify_create_addresses_proof(
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
            &v1_non_inclusion_26_1::VERIFYINGKEY,
        ),
        2 => verify::<4>(
            &public_inputs
                .try_into()
                .map_err(|_| PublicInputsTryIntoFailed)?,
            compressed_proof,
            &v1_non_inclusion_26_2::VERIFYINGKEY,
        ),
        3 => verify::<6>(
            &public_inputs
                .try_into()
                .map_err(|_| PublicInputsTryIntoFailed)?,
            compressed_proof,
            &v1_non_inclusion_26_3::VERIFYINGKEY,
        ),
        4 => verify::<8>(
            &public_inputs
                .try_into()
                .map_err(|_| PublicInputsTryIntoFailed)?,
            compressed_proof,
            &v1_non_inclusion_26_4::VERIFYINGKEY,
        ),
        8 => verify::<16>(
            &public_inputs
                .try_into()
                .map_err(|_| PublicInputsTryIntoFailed)?,
            compressed_proof,
            &v1_non_inclusion_26_8::VERIFYINGKEY,
        ),
        _ => Err(InvalidPublicInputsLength),
    }
}

#[inline(never)]
pub fn verify_create_addresses_and_inclusion_proof(
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
            &v1_combined_26_26_1_1::VERIFYINGKEY,
        ),
        6 => {
            let verifying_key = if address_roots.len() == 1 {
                &v1_combined_26_26_2_1::VERIFYINGKEY
            } else {
                &v1_combined_26_26_1_2::VERIFYINGKEY
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
                &v1_combined_26_26_3_1::VERIFYINGKEY
            } else {
                &v1_combined_26_26_2_2::VERIFYINGKEY
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
                &v1_combined_26_26_4_1::VERIFYINGKEY
            } else {
                &v1_combined_26_26_3_2::VERIFYINGKEY
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
            &v1_combined_26_26_4_2::VERIFYINGKEY,
        ),
        _ => Err(InvalidPublicInputsLength),
    }
}

#[inline(never)]
pub fn verify_inclusion_proof(
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
            &v1_inclusion_26_1::VERIFYINGKEY,
        ),
        4 => verify::<4>(
            &public_inputs
                .try_into()
                .map_err(|_| PublicInputsTryIntoFailed)?,
            compressed_proof,
            &v1_inclusion_26_2::VERIFYINGKEY,
        ),
        6 => verify::<6>(
            &public_inputs
                .try_into()
                .map_err(|_| PublicInputsTryIntoFailed)?,
            compressed_proof,
            &v1_inclusion_26_3::VERIFYINGKEY,
        ),
        8 => verify::<8>(
            &public_inputs
                .try_into()
                .map_err(|_| PublicInputsTryIntoFailed)?,
            compressed_proof,
            &v1_inclusion_26_4::VERIFYINGKEY,
        ),
        16 => verify::<16>(
            &public_inputs
                .try_into()
                .map_err(|_| PublicInputsTryIntoFailed)?,
            compressed_proof,
            &v1_inclusion_26_8::VERIFYINGKEY,
        ),
        _ => Err(InvalidPublicInputsLength),
    }
}

pub fn select_verifying_key<'a>(
    num_leaves: usize,
    num_addresses: usize,
) -> Result<&'a Groth16Verifyingkey<'static>, VerifierError> {
    #[cfg(all(feature = "solana", target_os = "solana"))]
    solana_msg::msg!(
        "select_verifying_key num_leaves: {}, num_addresses: {}",
        num_leaves,
        num_addresses
    );
    match (num_leaves, num_addresses) {
        // Combined cases (depend on both num_leaves and num_addresses)
        (1, 1) => Ok(&v2_combined_32_40_1_1::VERIFYINGKEY),
        (1, 2) => Ok(&v2_combined_32_40_1_2::VERIFYINGKEY),
        (1, 3) => Ok(&v2_combined_32_40_1_3::VERIFYINGKEY),
        (1, 4) => Ok(&v2_combined_32_40_1_4::VERIFYINGKEY),
        (2, 1) => Ok(&v2_combined_32_40_2_1::VERIFYINGKEY),
        (2, 2) => Ok(&v2_combined_32_40_2_2::VERIFYINGKEY),
        (2, 3) => Ok(&v2_combined_32_40_2_3::VERIFYINGKEY),
        (2, 4) => Ok(&v2_combined_32_40_2_4::VERIFYINGKEY),
        (3, 1) => Ok(&v2_combined_32_40_3_1::VERIFYINGKEY),
        (3, 2) => Ok(&v2_combined_32_40_3_2::VERIFYINGKEY),
        (3, 3) => Ok(&v2_combined_32_40_3_3::VERIFYINGKEY),
        (3, 4) => Ok(&v2_combined_32_40_3_4::VERIFYINGKEY),
        (4, 1) => Ok(&v2_combined_32_40_4_1::VERIFYINGKEY),
        (4, 2) => Ok(&v2_combined_32_40_4_2::VERIFYINGKEY),
        (4, 3) => Ok(&v2_combined_32_40_4_3::VERIFYINGKEY),
        (4, 4) => Ok(&v2_combined_32_40_4_4::VERIFYINGKEY),

        // Inclusion cases (depend on num_leaves)
        (1, _) => Ok(&v2_inclusion_32_1::VERIFYINGKEY),
        (2, _) => Ok(&v2_inclusion_32_2::VERIFYINGKEY),
        (3, _) => Ok(&v2_inclusion_32_3::VERIFYINGKEY),
        (4, _) => Ok(&v2_inclusion_32_4::VERIFYINGKEY),
        (5, _) => Ok(&v2_inclusion_32_5::VERIFYINGKEY),
        (6, _) => Ok(&v2_inclusion_32_6::VERIFYINGKEY),
        (7, _) => Ok(&v2_inclusion_32_7::VERIFYINGKEY),
        (8, _) => Ok(&v2_inclusion_32_8::VERIFYINGKEY),
        (9, _) => Ok(&v2_inclusion_32_9::VERIFYINGKEY),
        (10, _) => Ok(&v2_inclusion_32_10::VERIFYINGKEY),
        (11, _) => Ok(&v2_inclusion_32_11::VERIFYINGKEY),
        (12, _) => Ok(&v2_inclusion_32_12::VERIFYINGKEY),
        (13, _) => Ok(&v2_inclusion_32_13::VERIFYINGKEY),
        (14, _) => Ok(&v2_inclusion_32_14::VERIFYINGKEY),
        (15, _) => Ok(&v2_inclusion_32_15::VERIFYINGKEY),
        (16, _) => Ok(&v2_inclusion_32_16::VERIFYINGKEY),
        (17, _) => Ok(&v2_inclusion_32_17::VERIFYINGKEY),
        (18, _) => Ok(&v2_inclusion_32_18::VERIFYINGKEY),
        (19, _) => Ok(&v2_inclusion_32_19::VERIFYINGKEY),
        (20, _) => Ok(&v2_inclusion_32_20::VERIFYINGKEY),

        // Non-inclusion cases (depend on num_addresses)
        (_, 1) => Ok(&v2_non_inclusion_40_1::VERIFYINGKEY),
        (_, 2) => Ok(&v2_non_inclusion_40_2::VERIFYINGKEY),
        (_, 3) => Ok(&v2_non_inclusion_40_3::VERIFYINGKEY),
        (_, 4) => Ok(&v2_non_inclusion_40_4::VERIFYINGKEY),
        (_, 5) => Ok(&v2_non_inclusion_40_5::VERIFYINGKEY),
        (_, 6) => Ok(&v2_non_inclusion_40_6::VERIFYINGKEY),
        (_, 7) => Ok(&v2_non_inclusion_40_7::VERIFYINGKEY),
        (_, 8) => Ok(&v2_non_inclusion_40_8::VERIFYINGKEY),
        (_, 9) => Ok(&v2_non_inclusion_40_9::VERIFYINGKEY),
        (_, 10) => Ok(&v2_non_inclusion_40_10::VERIFYINGKEY),
        (_, 11) => Ok(&v2_non_inclusion_40_11::VERIFYINGKEY),
        (_, 12) => Ok(&v2_non_inclusion_40_12::VERIFYINGKEY),
        (_, 13) => Ok(&v2_non_inclusion_40_13::VERIFYINGKEY),
        (_, 14) => Ok(&v2_non_inclusion_40_14::VERIFYINGKEY),
        (_, 15) => Ok(&v2_non_inclusion_40_15::VERIFYINGKEY),
        (_, 16) => Ok(&v2_non_inclusion_40_16::VERIFYINGKEY),
        (_, 17) => Ok(&v2_non_inclusion_40_17::VERIFYINGKEY),
        (_, 18) => Ok(&v2_non_inclusion_40_18::VERIFYINGKEY),
        (_, 19) => Ok(&v2_non_inclusion_40_19::VERIFYINGKEY),
        (_, 20) => Ok(&v2_non_inclusion_40_20::VERIFYINGKEY),
        (_, 21) => Ok(&v2_non_inclusion_40_21::VERIFYINGKEY),
        (_, 22) => Ok(&v2_non_inclusion_40_22::VERIFYINGKEY),
        (_, 23) => Ok(&v2_non_inclusion_40_23::VERIFYINGKEY),
        (_, 24) => Ok(&v2_non_inclusion_40_24::VERIFYINGKEY),
        (_, 25) => Ok(&v2_non_inclusion_40_25::VERIFYINGKEY),
        (_, 26) => Ok(&v2_non_inclusion_40_26::VERIFYINGKEY),
        (_, 27) => Ok(&v2_non_inclusion_40_27::VERIFYINGKEY),
        (_, 28) => Ok(&v2_non_inclusion_40_28::VERIFYINGKEY),
        (_, 29) => Ok(&v2_non_inclusion_40_29::VERIFYINGKEY),
        (_, 30) => Ok(&v2_non_inclusion_40_30::VERIFYINGKEY),
        (_, 31) => Ok(&v2_non_inclusion_40_31::VERIFYINGKEY),
        (_, 32) => Ok(&v2_non_inclusion_40_32::VERIFYINGKEY),

        // Invalid configuration
        _ => Err(InvalidPublicInputsLength),
    }
}

#[inline(never)]
pub fn verify<const N: usize>(
    public_inputs: &[[u8; 32]; N],
    proof: &CompressedProof,
    vk: &Groth16Verifyingkey,
) -> Result<(), VerifierError> {
    let proof_a = decompress_g1(&proof.a).map_err(|_| DecompressG1Failed)?;
    let proof_b = decompress_g2(&proof.b).map_err(|_| DecompressG2Failed)?;
    let proof_c = decompress_g1(&proof.c).map_err(|_| DecompressG1Failed)?;
    let mut verifier = Groth16Verifier::new(&proof_a, &proof_b, &proof_c, public_inputs, vk)
        .map_err(|_| {
            #[cfg(all(target_os = "solana", feature = "solana"))]
            {
                use solana_msg::msg;
                msg!("Proof verification failed");
                msg!("Public inputs: {:?}", public_inputs);
                msg!("Proof A: {:?}", proof_a);
                msg!("Proof B: {:?}", proof_b);
                msg!("Proof C: {:?}", proof_c);
            }
            CreateGroth16VerifierFailed
        })?;
    verifier.verify().map_err(|_| {
        #[cfg(all(target_os = "solana", feature = "solana"))]
        {
            use solana_msg::msg;
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
pub fn verify_batch_append_with_proofs(
    batch_size: u64,
    public_input_hash: [u8; 32],
    compressed_proof: &CompressedProof,
) -> Result<(), VerifierError> {
    match batch_size {
        10 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &batch_append_32_10::VERIFYINGKEY,
        ),
        500 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &batch_append_32_500::VERIFYINGKEY,
        ),
        _ => Err(InvalidPublicInputsLength),
    }
}

#[inline(never)]
pub fn verify_batch_update(
    batch_size: u64,
    public_input_hash: [u8; 32],
    compressed_proof: &CompressedProof,
) -> Result<(), VerifierError> {
    match batch_size {
        10 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &batch_update_32_10::VERIFYINGKEY,
        ),
        500 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &batch_update_32_500::VERIFYINGKEY,
        ),
        _ => Err(InvalidPublicInputsLength),
    }
}

#[inline(never)]
pub fn verify_batch_address_update(
    batch_size: u64,
    public_input_hash: [u8; 32],
    compressed_proof: &CompressedProof,
) -> Result<(), VerifierError> {
    match batch_size {
        10 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &batch_address_append_40_10::VERIFYINGKEY,
        ),
        250 => verify::<1>(
            &[public_input_hash],
            compressed_proof,
            &batch_address_append_40_250::VERIFYINGKEY,
        ),
        _ => Err(InvalidPublicInputsLength),
    }
}

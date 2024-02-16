use std::{collections::HashMap, ops::Neg};
use std::fs::File;

use ark_bn254::Bn254;
use ark_circom::circom::Inputs;
use ark_circom::read_zkey;
use ark_groth16::Proof as ArkGroth16Proof;
use ark_serialize::{CanonicalSerialize, Compress};
use groth16_solana::{
    decompression::{decompress_g1, decompress_g2},
    groth16::{Groth16Verifier, Groth16Verifyingkey},
};

use crate::{
    arkworks_prover::{prove, ArkProof},
    errors::CircuitsError,
    helpers::{change_endianness, convert_endianness_128},
    merkle_proof_inputs::{public_inputs, MerkleTreeInfo, MerkleTreeProofInput, ProofInputs}
};
use crate::arkworks_prover::ArkProvingKey;

pub struct ProofCompressed {
    pub a: [u8; 32],
    pub b: [u8; 64],
    pub c: [u8; 32],
}

impl ProofCompressed {
    pub fn try_decompress(&self) -> Result<Proof, CircuitsError> {
        let proof_a = decompress_g1(&self.a)?;
        let proof_b = decompress_g2(&self.b)?;
        let proof_c = decompress_g1(&self.c)?;
        Ok(Proof {
            a: proof_a,
            b: proof_b,
            c: proof_c,
        })
    }
}

pub struct Proof {
    pub a: [u8; 64],
    pub b: [u8; 128],
    pub c: [u8; 64],
}

pub struct ProofResult {
    pub proof: ProofCompressed,
    pub public_inputs: Vec<[u8; 32]>,
}

pub fn merkle_inclusion_proof(
    merkle_tree_info: &MerkleTreeInfo,
    merkle_proof_inputs: &[MerkleTreeProofInput],
) -> Result<ProofResult, CircuitsError> {
    let merkle_proof_inputs_len = merkle_proof_inputs.len();

    let proof_inputs = ProofInputs(merkle_proof_inputs);
    let inputs_hashmap: HashMap<String, Inputs> = proof_inputs
        .try_into()
        .map_err(|_| CircuitsError::ChangeEndiannessError)?;

    let path = format!("test-data/merkle{}_{}/circuit.zkey", merkle_tree_info.height(), merkle_proof_inputs_len);
    let mut file = File::open(path).unwrap();
    let pk: ArkProvingKey = read_zkey(&mut file).unwrap();
    let wasm_path = merkle_tree_info.wasm_path(merkle_proof_inputs_len);
    let proof = prove(
        merkle_tree_info,
        merkle_proof_inputs_len,
        inputs_hashmap,
        &pk,
        &wasm_path,
    )?;

    let proof = convert_arkworks_proof_to_solana_groth16(&proof)?;
    let public_inputs = public_inputs(merkle_proof_inputs);
    Ok(ProofResult {
        proof,
        public_inputs,
    })
}

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

fn convert_arkworks_proof_to_solana_groth16(
    proof: &ArkProof,
) -> Result<ProofCompressed, CircuitsError> {
    let proof_bn254: ArkGroth16Proof<Bn254> = proof.clone().0;
    let proof_a = proof_bn254.a.neg();
    let proof_b = proof_bn254.b;
    let proof_c = proof_bn254.c;

    let mut a: [u8; 32] = [0; 32];
    proof_a.serialize_with_mode(&mut a[..], Compress::Yes)?;
    a = change_endianness(&a)
        .try_into()
        .map_err(|_| CircuitsError::ChangeEndiannessError)?;
    let mut b: [u8; 64] = [0; 64];
    proof_b.serialize_with_mode(&mut b[..], Compress::Yes)?;
    b = convert_endianness_128(&b)
        .try_into()
        .map_err(|_| CircuitsError::ChangeEndiannessError)?;
    let mut c: [u8; 32] = [0; 32];
    proof_c.serialize_with_mode(&mut c[..], Compress::Yes)?;
    c = change_endianness(&c)
        .try_into()
        .map_err(|_| CircuitsError::ChangeEndiannessError)?;

    Ok(ProofCompressed { a, b, c })
}

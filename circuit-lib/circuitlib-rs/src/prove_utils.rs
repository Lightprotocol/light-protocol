use std::ops::Neg;

use ark_bn254::Bn254;
use ark_groth16::Proof as ArkGroth16Proof;
use ark_serialize::{CanonicalSerialize, Compress};
use groth16_solana::decompression::{decompress_g1, decompress_g2};

use crate::{
    arkworks_prover::ArkProof,
    errors::CircuitsError,
    helpers::{change_endianness, convert_endianness_128},
};

pub struct ProofResult {
    pub proof: ProofCompressed,
    pub public_inputs: Vec<[u8; 32]>,
}

#[derive(Debug)]
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

pub fn convert_arkworks_proof_to_solana_groth16(
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

use groth16_solana::decompression::{decompress_g1, decompress_g2};

use crate::errors::ProverClientError;

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
    pub fn try_decompress(&self) -> Result<Proof, ProverClientError> {
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

#[derive(Debug, PartialEq, Eq)]
pub enum CircuitType {
    Combined,
    Inclusion,
    NonInclusion,
    BatchAppendWithSubtrees,
    BatchAppendWithProofs,
    BatchUpdate,
    BatchAddressAppend,
}

impl CircuitType {
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        match self {
            Self::Combined => "combined".to_string(),
            Self::Inclusion => "inclusion".to_string(),
            Self::NonInclusion => "non-inclusion".to_string(),
            Self::BatchAppendWithSubtrees => "append-with-subtrees".to_string(),
            Self::BatchAppendWithProofs => "append-with-proofs".to_string(),
            Self::BatchUpdate => "update".to_string(),
            Self::BatchAddressAppend => "address-append".to_string(),
        }
    }
}

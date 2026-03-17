use serde::{Deserialize, Serialize};

use crate::errors::ProverClientError;
type G1 = ark_bn254::g1::G1Affine;
use std::ops::Neg;

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Compress, Validate};
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use num_traits::Num;
use solana_bn254::compression::prelude::{
    alt_bn128_g1_compress, alt_bn128_g1_decompress, alt_bn128_g2_compress, alt_bn128_g2_decompress,
    convert_endianness,
};

#[derive(Debug, Clone, Copy)]
pub struct ProofCompressed {
    pub a: [u8; 32],
    pub b: [u8; 64],
    pub c: [u8; 32],
}

#[derive(Debug, Clone, Copy)]
pub struct ProofResult {
    pub proof: ProofCompressed,
    pub proof_duration_ms: u64,
}

impl From<ProofCompressed> for CompressedProof {
    fn from(proof: ProofCompressed) -> Self {
        CompressedProof {
            a: proof.a,
            b: proof.b,
            c: proof.c,
        }
    }
}

impl ProofCompressed {
    pub fn try_decompress(&self) -> Result<Proof, ProverClientError> {
        let proof_a = alt_bn128_g1_decompress(&self.a)?;
        let proof_b = alt_bn128_g2_decompress(&self.b)?;
        let proof_c = alt_bn128_g1_decompress(&self.c)?;
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

#[derive(Serialize, Deserialize, Debug)]
pub struct GnarkProofJson {
    pub ar: Vec<String>,
    pub bs: Vec<Vec<String>>,
    pub krs: Vec<String>,
}

pub fn deserialize_gnark_proof_json(json_data: &str) -> serde_json::Result<GnarkProofJson> {
    let deserialized_data: GnarkProofJson = serde_json::from_str(json_data)?;
    Ok(deserialized_data)
}

pub fn deserialize_hex_string_to_be_bytes(hex_str: &str) -> Result<[u8; 32], ProverClientError> {
    let trimmed_str = hex_str
        .strip_prefix("0x")
        .or_else(|| hex_str.strip_prefix("0X"))
        .unwrap_or(hex_str);
    let big_uint = num_bigint::BigUint::from_str_radix(trimmed_str, 16).map_err(|error| {
        ProverClientError::InvalidHexString(format!("{hex_str}: {error}"))
    })?;
    let big_uint_bytes = big_uint.to_bytes_be();
    if big_uint_bytes.len() > 32 {
        return Err(ProverClientError::InvalidHexString(format!(
            "{hex_str}: exceeds 32 bytes"
        )));
    }
    if big_uint_bytes.len() < 32 {
        let mut result = [0u8; 32];
        result[32 - big_uint_bytes.len()..].copy_from_slice(&big_uint_bytes);
        Ok(result)
    } else {
        big_uint_bytes.try_into().map_err(|_| {
            ProverClientError::InvalidHexString(format!("{hex_str}: invalid 32-byte encoding"))
        })
    }
}

pub fn compress_proof(
    proof_a: &[u8; 64],
    proof_b: &[u8; 128],
    proof_c: &[u8; 64],
) -> Result<([u8; 32], [u8; 64], [u8; 32]), ProverClientError> {
    let proof_a = alt_bn128_g1_compress(proof_a)?;
    let proof_b = alt_bn128_g2_compress(proof_b)?;
    let proof_c = alt_bn128_g1_compress(proof_c)?;
    Ok((proof_a, proof_b, proof_c))
}

pub fn proof_from_json_struct(
    json: GnarkProofJson,
) -> Result<([u8; 64], [u8; 128], [u8; 64]), ProverClientError> {
    let proof_a_x = deserialize_hex_string_to_be_bytes(json.ar.first().ok_or_else(|| {
        ProverClientError::InvalidProofData("missing proof A x coordinate".to_string())
    })?)?;
    let proof_a_y = deserialize_hex_string_to_be_bytes(json.ar.get(1).ok_or_else(|| {
        ProverClientError::InvalidProofData("missing proof A y coordinate".to_string())
    })?)?;
    let proof_a: [u8; 64] = [proof_a_x, proof_a_y]
        .concat()
        .try_into()
        .map_err(|_| ProverClientError::InvalidProofData("invalid proof A length".to_string()))?;
    let proof_a = negate_g1(&proof_a)?;
    let proof_b_x_0 = deserialize_hex_string_to_be_bytes(
        json.bs
            .first()
            .and_then(|row| row.first())
            .ok_or_else(|| {
                ProverClientError::InvalidProofData("missing proof B x0 coordinate".to_string())
            })?,
    )?;
    let proof_b_x_1 = deserialize_hex_string_to_be_bytes(
        json.bs
            .first()
            .and_then(|row| row.get(1))
            .ok_or_else(|| {
                ProverClientError::InvalidProofData("missing proof B x1 coordinate".to_string())
            })?,
    )?;
    let proof_b_y_0 = deserialize_hex_string_to_be_bytes(
        json.bs
            .get(1)
            .and_then(|row| row.first())
            .ok_or_else(|| {
                ProverClientError::InvalidProofData("missing proof B y0 coordinate".to_string())
            })?,
    )?;
    let proof_b_y_1 = deserialize_hex_string_to_be_bytes(
        json.bs
            .get(1)
            .and_then(|row| row.get(1))
            .ok_or_else(|| {
                ProverClientError::InvalidProofData("missing proof B y1 coordinate".to_string())
            })?,
    )?;
    let proof_b: [u8; 128] = [proof_b_x_0, proof_b_x_1, proof_b_y_0, proof_b_y_1]
        .concat()
        .try_into()
        .map_err(|_| ProverClientError::InvalidProofData("invalid proof B length".to_string()))?;

    let proof_c_x = deserialize_hex_string_to_be_bytes(json.krs.first().ok_or_else(|| {
        ProverClientError::InvalidProofData("missing proof C x coordinate".to_string())
    })?)?;
    let proof_c_y = deserialize_hex_string_to_be_bytes(json.krs.get(1).ok_or_else(|| {
        ProverClientError::InvalidProofData("missing proof C y coordinate".to_string())
    })?)?;
    let proof_c: [u8; 64] = [proof_c_x, proof_c_y]
        .concat()
        .try_into()
        .map_err(|_| ProverClientError::InvalidProofData("invalid proof C length".to_string()))?;
    Ok((proof_a, proof_b, proof_c))
}

pub fn negate_g1(g1_be: &[u8; 64]) -> Result<[u8; 64], ProverClientError> {
    let g1_le = convert_endianness::<32, 64>(g1_be);
    let g1: G1 = G1::deserialize_with_mode(g1_le.as_slice(), Compress::No, Validate::Yes)
        .map_err(|error| ProverClientError::InvalidProofData(error.to_string()))?;

    let g1_neg = g1.neg();
    let mut g1_neg_be = [0u8; 64];
    g1_neg
        .x
        .serialize_with_mode(&mut g1_neg_be[..32], Compress::No)
        .map_err(|error| ProverClientError::InvalidProofData(error.to_string()))?;
    g1_neg
        .y
        .serialize_with_mode(&mut g1_neg_be[32..], Compress::No)
        .map_err(|error| ProverClientError::InvalidProofData(error.to_string()))?;
    let g1_neg_be: [u8; 64] = convert_endianness::<32, 64>(&g1_neg_be);
    Ok(g1_neg_be)
}

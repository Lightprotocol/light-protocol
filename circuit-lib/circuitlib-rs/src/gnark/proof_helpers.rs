use serde::{Deserialize, Serialize};
type G1 = ark_bn254::g1::G1Affine;
use std::ops::Neg;

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Compress, Validate};
use groth16_solana::syscalls::alt_bn128::compression::target_arch::{
    alt_bn128_g1_compress, alt_bn128_g2_compress, convert_endianness,
};
use num_traits::Num;

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

pub fn deserialize_hex_string_to_be_bytes(hex_str: &str) -> [u8; 32] {
    let trimmed_str = hex_str.trim_start_matches("0x");
    let big_int = num_bigint::BigInt::from_str_radix(trimmed_str, 16).unwrap();
    let big_int_bytes = big_int.to_bytes_be().1;
    if big_int_bytes.len() < 32 {
        let mut result = [0u8; 32];
        result[32 - big_int_bytes.len()..].copy_from_slice(&big_int_bytes);
        result
    } else {
        big_int_bytes.try_into().unwrap()
    }
}

pub fn compress_proof(
    proof_a: &[u8; 64],
    proof_b: &[u8; 128],
    proof_c: &[u8; 64],
) -> ([u8; 32], [u8; 64], [u8; 32]) {
    let proof_a = alt_bn128_g1_compress(proof_a).unwrap();
    let proof_b = alt_bn128_g2_compress(proof_b).unwrap();
    let proof_c = alt_bn128_g1_compress(proof_c).unwrap();
    (proof_a, proof_b, proof_c)
}

pub fn proof_from_json_struct(json: GnarkProofJson) -> ([u8; 64], [u8; 128], [u8; 64]) {
    let proof_a_x = deserialize_hex_string_to_be_bytes(&json.ar[0]);
    let proof_a_y = deserialize_hex_string_to_be_bytes(&json.ar[1]);
    let proof_a: [u8; 64] = [proof_a_x, proof_a_y].concat().try_into().unwrap();
    let proof_a = negate_g1(&proof_a);
    let proof_b_x_0 = deserialize_hex_string_to_be_bytes(&json.bs[0][0]);
    let proof_b_x_1 = deserialize_hex_string_to_be_bytes(&json.bs[0][1]);
    let proof_b_y_0 = deserialize_hex_string_to_be_bytes(&json.bs[1][0]);
    let proof_b_y_1 = deserialize_hex_string_to_be_bytes(&json.bs[1][1]);
    let proof_b: [u8; 128] = [proof_b_x_0, proof_b_x_1, proof_b_y_0, proof_b_y_1]
        .concat()
        .try_into()
        .unwrap();

    let proof_c_x = deserialize_hex_string_to_be_bytes(&json.krs[0]);
    let proof_c_y = deserialize_hex_string_to_be_bytes(&json.krs[1]);
    let proof_c: [u8; 64] = [proof_c_x, proof_c_y].concat().try_into().unwrap();
    (proof_a, proof_b, proof_c)
}

pub fn negate_g1(g1_be: &[u8; 64]) -> [u8; 64] {
    let g1_le = convert_endianness::<32, 64>(g1_be);
    let g1: G1 = G1::deserialize_with_mode(g1_le.as_slice(), Compress::No, Validate::No).unwrap();

    let g1_neg = g1.neg();
    let mut g1_neg_be = [0u8; 64];
    g1_neg
        .x
        .serialize_with_mode(&mut g1_neg_be[..32], Compress::No)
        .unwrap();
    g1_neg
        .y
        .serialize_with_mode(&mut g1_neg_be[32..], Compress::No)
        .unwrap();
    let g1_neg_be: [u8; 64] = convert_endianness::<32, 64>(&g1_neg_be);
    g1_neg_be
}

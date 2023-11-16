use ark_ff::bytes::FromBytes;
use ark_ff::bytes::ToBytes;
use std::ops::Neg;

type G1 = ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>;

pub fn negate_proof_a(proof_a: [u8; 64]) -> [u8; 64] {
    let proof_a_neg_g1: G1 =
        <G1 as FromBytes>::read(&*[&change_endianness(&proof_a)[..], &[0u8][..]].concat()).unwrap();
    let mut proof_a_neg_buf = [0u8; 65];
    <G1 as ToBytes>::write(&proof_a_neg_g1.neg(), &mut proof_a_neg_buf[..]).unwrap();
    let mut proof_a_neg = [0u8; 64];
    proof_a_neg.copy_from_slice(&proof_a_neg_buf[..64]);

    let proof_a_neg = change_endianness(&proof_a_neg);
    proof_a_neg
}

const CHUNK_SIZE: usize = 32;
pub fn change_endianness<const SIZE: usize>(bytes: &[u8; SIZE]) -> [u8; SIZE] {
    let mut arr = [0u8; SIZE];
    for (i, b) in bytes.chunks(CHUNK_SIZE).enumerate() {
        for (j, byte) in b.iter().rev().enumerate() {
            arr[i * CHUNK_SIZE + j] = *byte;
        }
    }
    arr
}

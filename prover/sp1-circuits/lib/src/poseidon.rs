//! Poseidon hash functions matching the Gnark implementation.
//!
//! Uses the light-poseidon crate with Circom-compatible parameters.
//! The Gnark implementation uses the same Circom parameters.

use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use light_poseidon::{Poseidon, PoseidonBytesHasher, PoseidonError};

/// Hash result type - 32 bytes in big-endian format.
pub type Hash = [u8; 32];

/// Poseidon hash with 1 input (internally padded with zero).
///
/// Matches Gnark's `Poseidon1` gadget: `poseidon([0, input])`
pub fn poseidon1(input: &[u8; 32]) -> Result<Hash, PoseidonError> {
    let mut hasher = Poseidon::<Fr>::new_circom(1)?;
    hasher.hash_bytes_be(&[input])
}

/// Poseidon hash with 2 inputs.
///
/// Matches Gnark's `Poseidon2` gadget: `poseidon([0, in1, in2])`
pub fn poseidon2(in1: &[u8; 32], in2: &[u8; 32]) -> Result<Hash, PoseidonError> {
    let mut hasher = Poseidon::<Fr>::new_circom(2)?;
    hasher.hash_bytes_be(&[in1, in2])
}

/// Poseidon hash with 3 inputs.
///
/// Matches Gnark's `Poseidon3` gadget: `poseidon([0, in1, in2, in3])`
pub fn poseidon3(in1: &[u8; 32], in2: &[u8; 32], in3: &[u8; 32]) -> Result<Hash, PoseidonError> {
    let mut hasher = Poseidon::<Fr>::new_circom(3)?;
    hasher.hash_bytes_be(&[in1, in2, in3])
}

/// Poseidon hash with 4 inputs.
///
/// Matches Gnark's `Poseidon4` gadget.
pub fn poseidon4(
    in1: &[u8; 32],
    in2: &[u8; 32],
    in3: &[u8; 32],
    in4: &[u8; 32],
) -> Result<Hash, PoseidonError> {
    let mut hasher = Poseidon::<Fr>::new_circom(4)?;
    hasher.hash_bytes_be(&[in1, in2, in3, in4])
}

/// Create a hash chain from a list of hashes.
///
/// Computes: H(H(H(h[0], h[1]), h[2]), h[3], ...)
///
/// Used for computing `leavesHashchainHash` in batch circuits.
pub fn hash_chain(hashes: &[[u8; 32]]) -> Result<Hash, PoseidonError> {
    if hashes.is_empty() {
        return Ok([0u8; 32]);
    }

    let mut result = hashes[0];
    for hash in hashes.iter().skip(1) {
        result = poseidon2(&result, hash)?;
    }
    Ok(result)
}

/// Convert a u32 to a 32-byte big-endian representation for hashing.
pub fn u32_to_bytes(value: u32) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[28..32].copy_from_slice(&value.to_be_bytes());
    bytes
}

/// Convert a u64 to a 32-byte big-endian representation for hashing.
pub fn u64_to_bytes(value: u64) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[24..32].copy_from_slice(&value.to_be_bytes());
    bytes
}

/// Convert Fr field element to 32 bytes big-endian.
pub fn fr_to_bytes(fr: &Fr) -> [u8; 32] {
    let bytes = fr.into_bigint().to_bytes_be();
    let mut result = [0u8; 32];
    result.copy_from_slice(&bytes);
    result
}

/// Convert 32 bytes big-endian to Fr field element.
pub fn bytes_to_fr(bytes: &[u8; 32]) -> Result<Fr, &'static str> {
    Fr::from_be_bytes_mod_order(bytes)
        .try_into()
        .map_err(|_| "Failed to convert bytes to Fr")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poseidon2_basic() {
        let a = [0u8; 32];
        let b = [0u8; 32];
        let result = poseidon2(&a, &b).unwrap();
        // Result should be non-zero for zero inputs
        assert_ne!(result, [0u8; 32]);
    }

    #[test]
    fn test_hash_chain_empty() {
        let result = hash_chain(&[]).unwrap();
        assert_eq!(result, [0u8; 32]);
    }

    #[test]
    fn test_hash_chain_single() {
        let input = [1u8; 32];
        let result = hash_chain(&[input]).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn test_hash_chain_multiple() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        let c = [3u8; 32];

        let chain_result = hash_chain(&[a, b, c]).unwrap();

        // Manual: H(H(a, b), c)
        let h_ab = poseidon2(&a, &b).unwrap();
        let expected = poseidon2(&h_ab, &c).unwrap();

        assert_eq!(chain_result, expected);
    }

    #[test]
    fn test_u32_to_bytes() {
        let value = 12345u32;
        let bytes = u32_to_bytes(value);
        assert_eq!(&bytes[28..32], &value.to_be_bytes());
        assert_eq!(&bytes[0..28], &[0u8; 28]);
    }
}

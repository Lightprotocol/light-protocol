#![allow(unexpected_cfgs)]

use ark_ff::PrimeField;
use light_hasher::HasherError;
use num_bigint::BigUint;
use solana_program::keccak::hashv;
use thiserror::Error;

pub mod address;
pub mod bigint;
pub mod compressed_account;
pub mod constants;
pub mod discriminators;
pub mod hash_chain;
pub mod indexer_event;
pub mod instruction_data;
pub mod nullifier;
pub mod pubkey;
pub mod tx_hash;

#[derive(Debug, Error, PartialEq)]
pub enum CompressedAccountError {
    #[error("Invalid input size, expected at most {0}")]
    InputTooLarge(usize),
    #[error("Invalid chunk size")]
    InvalidChunkSize,
    #[error("Invalid seeds")]
    InvalidSeeds,
    #[error("Invalid rollover thresold")]
    InvalidRolloverThreshold,
    #[error("Invalid input lenght")]
    InvalidInputLength,
    #[error("Hasher error {0}")]
    HasherError(#[from] HasherError),
    #[error("Invalid Account size.")]
    InvalidAccountSize,
    #[error("Account is mutable.")]
    AccountMutable,
    #[error("Account is already initialized.")]
    AlreadyInitialized,
    #[error("Invalid account balance.")]
    InvalidAccountBalance,
    #[error("Failed to borrow rent sysvar.")]
    FailedBorrowRentSysvar,
    #[error("Derive address error.")]
    DeriveAddressError,
    #[error("Invalid argument.")]
    InvalidArgument,
}

// NOTE(vadorovsky): Unfortunately, we need to do it by hand.
// `num_derive::ToPrimitive` doesn't support data-carrying enums.
impl From<CompressedAccountError> for u32 {
    fn from(e: CompressedAccountError) -> u32 {
        match e {
            CompressedAccountError::InputTooLarge(_) => 12001,
            CompressedAccountError::InvalidChunkSize => 12002,
            CompressedAccountError::InvalidSeeds => 12003,
            CompressedAccountError::InvalidRolloverThreshold => 12004,
            CompressedAccountError::InvalidInputLength => 12005,
            CompressedAccountError::InvalidAccountSize => 12010,
            CompressedAccountError::AccountMutable => 12011,
            CompressedAccountError::AlreadyInitialized => 12012,
            CompressedAccountError::InvalidAccountBalance => 12013,
            CompressedAccountError::FailedBorrowRentSysvar => 12014,
            CompressedAccountError::DeriveAddressError => 12015,
            CompressedAccountError::InvalidArgument => 12016,
            CompressedAccountError::HasherError(e) => u32::from(e),
        }
    }
}

impl From<CompressedAccountError> for solana_program::program_error::ProgramError {
    fn from(e: CompressedAccountError) -> Self {
        solana_program::program_error::ProgramError::Custom(e.into())
    }
}

pub fn is_smaller_than_bn254_field_size_be(bytes: &[u8; 32]) -> bool {
    let bigint = BigUint::from_bytes_be(bytes);
    bigint < ark_bn254::Fr::MODULUS.into()
}

pub fn hash_to_bn254_field_size_be(bytes: &[u8]) -> Option<([u8; 32], u8)> {
    let mut bump_seed = [u8::MAX];
    // Loops with decreasing bump seed to find a valid hash which is less than
    // bn254 Fr modulo field size.
    for _ in 0..u8::MAX {
        {
            let mut hashed_value: [u8; 32] = hashv(&[bytes, bump_seed.as_ref()]).to_bytes();
            // Truncates to 31 bytes so that value is less than bn254 Fr modulo
            // field size.
            hashed_value[0] = 0;
            if is_smaller_than_bn254_field_size_be(&hashed_value) {
                return Some((hashed_value, bump_seed[0]));
            }
        }
        bump_seed[0] -= 1;
    }
    None
}

/// Hashes the provided `bytes` with Keccak256 and ensures the result fits
/// in the BN254 prime field by repeatedly hashing the inputs with various
/// "bump seeds" and truncating the resulting hash to 31 bytes.
///
/// The attempted "bump seeds" are bytes from 255 to 0.
///
/// # Examples
///
/// ```
/// use light_compressed_account::hashv_to_bn254_field_size_be;
///
/// hashv_to_bn254_field_size_be(&[b"foo", b"bar"]);
/// ```
pub fn hashv_to_bn254_field_size_be(bytes: &[&[u8]]) -> [u8; 32] {
    let mut hashed_value: [u8; 32] = hashv(bytes).to_bytes();
    // Truncates to 31 bytes so that value is less than bn254 Fr modulo
    // field size.
    hashed_value[0] = 0;
    hashed_value
}

#[cfg(test)]
mod tests {
    use num_bigint::ToBigUint;
    use solana_program::pubkey::Pubkey;

    use super::*;
    use crate::bigint::bigint_to_be_bytes_array;

    #[test]
    fn test_is_smaller_than_bn254_field_size_be() {
        let modulus: BigUint = ark_bn254::Fr::MODULUS.into();
        let modulus_bytes: [u8; 32] = bigint_to_be_bytes_array(&modulus).unwrap();
        assert!(!is_smaller_than_bn254_field_size_be(&modulus_bytes));

        let bigint = modulus.clone() - 1.to_biguint().unwrap();
        let bigint_bytes: [u8; 32] = bigint_to_be_bytes_array(&bigint).unwrap();
        assert!(is_smaller_than_bn254_field_size_be(&bigint_bytes));

        let bigint = modulus + 1.to_biguint().unwrap();
        let bigint_bytes: [u8; 32] = bigint_to_be_bytes_array(&bigint).unwrap();
        assert!(!is_smaller_than_bn254_field_size_be(&bigint_bytes));
    }

    #[test]
    fn test_hash_to_bn254_field_size_be() {
        for _ in 0..10_000 {
            let input_bytes = Pubkey::new_unique().to_bytes(); // Sample input
            let (hashed_value, bump) = hash_to_bn254_field_size_be(input_bytes.as_slice())
                .expect("Failed to find a hash within BN254 field size");
            assert_eq!(bump, 255, "Bump seed should be 0");
            assert!(
                is_smaller_than_bn254_field_size_be(&hashed_value),
                "Hashed value should be within BN254 field size"
            );
        }

        let max_input = [u8::MAX; 32];
        let (hashed_value, bump) = hash_to_bn254_field_size_be(max_input.as_slice())
            .expect("Failed to find a hash within BN254 field size");
        assert_eq!(bump, 255, "Bump seed should be 255");
        assert!(
            is_smaller_than_bn254_field_size_be(&hashed_value),
            "Hashed value should be within BN254 field size"
        );
    }

    #[test]
    fn test_hashv_to_bn254_field_size_be() {
        for _ in 0..10_000 {
            let input_bytes = [Pubkey::new_unique().to_bytes(); 4];
            let input_bytes = input_bytes.iter().map(|x| x.as_slice()).collect::<Vec<_>>();
            let hashed_value = hashv_to_bn254_field_size_be(input_bytes.as_slice());
            assert!(
                is_smaller_than_bn254_field_size_be(&hashed_value),
                "Hashed value should be within BN254 field size"
            );
        }

        let max_input = [[u8::MAX; 32]; 16];
        let max_input = max_input.iter().map(|x| x.as_slice()).collect::<Vec<_>>();
        let hashed_value = hashv_to_bn254_field_size_be(max_input.as_slice());
        assert!(
            is_smaller_than_bn254_field_size_be(&hashed_value),
            "Hashed value should be within BN254 field size"
        );
    }
}

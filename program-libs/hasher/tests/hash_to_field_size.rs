#![cfg(feature = "poseidon")]

use ark_ff::PrimeField;
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::{
    bigint::bigint_to_be_bytes_array,
    hash_to_field_size::{
        hash_to_bn254_field_size_be, hashv_to_bn254_field_size_be,
        is_smaller_than_bn254_field_size_be, HashToFieldSize,
    },
};
use num_bigint::{BigUint, ToBigUint};

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
fn hash_to_field_size_borsh() {
    #[derive(BorshSerialize, BorshDeserialize)]
    pub struct TestStruct {
        a: u32,
        b: u32,
        c: u64,
    }
    let test_struct = TestStruct { a: 1, b: 2, c: 3 };
    let serialized = test_struct.try_to_vec().unwrap();
    let hash = test_struct.hash_to_field_size().unwrap();
    let manual_hash = hash_to_bn254_field_size_be(&serialized);
    assert_eq!(hash, manual_hash);
}

#[test]
fn test_hash_to_bn254_field_size_be() {
    use solana_pubkey::Pubkey;
    for _ in 0..10_000 {
        let input_bytes = Pubkey::new_unique().to_bytes(); // Sample input
        let hashed_value = hash_to_bn254_field_size_be(input_bytes.as_slice());
        assert!(
            is_smaller_than_bn254_field_size_be(&hashed_value),
            "Hashed value should be within BN254 field size"
        );
    }

    let max_input = [u8::MAX; 32];
    let hashed_value = hash_to_bn254_field_size_be(max_input.as_slice());
    assert!(
        is_smaller_than_bn254_field_size_be(&hashed_value),
        "Hashed value should be within BN254 field size"
    );
}

#[test]
fn test_hashv_to_bn254_field_size_be() {
    use solana_pubkey::Pubkey;
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

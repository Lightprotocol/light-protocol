#![cfg(feature = "poseidon")]

use light_compressed_account::{hash_to_bn254_field_size_be, Pubkey};
use light_ctoken_types::state::{CompressedTokenAccountState, TokenData};
use light_hasher::HasherError;
use num_bigint::BigUint;
use rand::Rng;

#[test]
fn equivalency_of_hash_functions() {
    let token_data = TokenData {
        mint: Pubkey::new_unique(),
        owner: Pubkey::new_unique(),
        amount: 100,
        delegate: Some(Pubkey::new_unique()),
        state: CompressedTokenAccountState::Initialized as u8,
        tlv: None,
    };
    let hashed_token_data = token_data.hash_v1().unwrap();
    let hashed_mint = hash_to_bn254_field_size_be(token_data.mint.to_bytes().as_slice());
    let hashed_owner = hash_to_bn254_field_size_be(token_data.owner.to_bytes().as_slice());
    let hashed_delegate =
        hash_to_bn254_field_size_be(token_data.delegate.unwrap().to_bytes().as_slice());
    let mut amount_bytes = [0u8; 32];
    amount_bytes[24..].copy_from_slice(token_data.amount.to_le_bytes().as_slice());
    let hashed_token_data_with_hashed_values = TokenData::hash_inputs_with_hashed_values::<false>(
        &hashed_mint,
        &hashed_owner,
        &amount_bytes,
        &Some(&hashed_delegate),
    )
    .unwrap();
    assert_eq!(hashed_token_data, hashed_token_data_with_hashed_values);

    let token_data = TokenData {
        mint: Pubkey::new_unique(),
        owner: Pubkey::new_unique(),
        amount: 101,
        delegate: None,
        state: CompressedTokenAccountState::Initialized as u8,
        tlv: None,
    };
    let hashed_token_data = token_data.hash_v1().unwrap();
    let hashed_mint = hash_to_bn254_field_size_be(token_data.mint.to_bytes().as_slice());
    let hashed_owner = hash_to_bn254_field_size_be(token_data.owner.to_bytes().as_slice());
    let mut amount_bytes = [0u8; 32];
    amount_bytes[24..].copy_from_slice(token_data.amount.to_le_bytes().as_slice());
    let hashed_token_data_with_hashed_values =
        TokenData::hash_with_hashed_values(&hashed_mint, &hashed_owner, &amount_bytes, &None)
            .unwrap();
    assert_eq!(hashed_token_data, hashed_token_data_with_hashed_values);
}

fn legacy_hash(token_data: &TokenData) -> std::result::Result<[u8; 32], HasherError> {
    let hashed_mint = hash_to_bn254_field_size_be(token_data.mint.to_bytes().as_slice());
    let hashed_owner = hash_to_bn254_field_size_be(token_data.owner.to_bytes().as_slice());
    let mut amount_bytes = [0u8; 32];
    amount_bytes[24..].copy_from_slice(token_data.amount.to_le_bytes().as_slice());
    let hashed_delegate;
    let hashed_delegate_option = if let Some(delegate) = token_data.delegate {
        hashed_delegate = hash_to_bn254_field_size_be(delegate.to_bytes().as_slice());
        Some(&hashed_delegate)
    } else {
        None
    };
    if token_data.state != CompressedTokenAccountState::Initialized as u8 {
        TokenData::hash_inputs_with_hashed_values::<true>(
            &hashed_mint,
            &hashed_owner,
            &amount_bytes,
            &hashed_delegate_option,
        )
    } else {
        TokenData::hash_inputs_with_hashed_values::<false>(
            &hashed_mint,
            &hashed_owner,
            &amount_bytes,
            &hashed_delegate_option,
        )
    }
}

fn equivalency_of_hash_functions_rnd_iters<const ITERS: usize>() {
    let mut rng = rand::thread_rng();

    for _ in 0..ITERS {
        let token_data = TokenData {
            mint: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            amount: rng.gen(),
            delegate: Some(Pubkey::new_unique()),
            state: CompressedTokenAccountState::Initialized as u8,
            tlv: None,
        };
        let hashed_token_data = token_data.hash_v1().unwrap();
        let hashed_mint = hash_to_bn254_field_size_be(token_data.mint.to_bytes().as_slice());
        let hashed_owner = hash_to_bn254_field_size_be(token_data.owner.to_bytes().as_slice());
        let hashed_delegate =
            hash_to_bn254_field_size_be(token_data.delegate.unwrap().to_bytes().as_slice());
        let mut amount_bytes = [0u8; 32];
        amount_bytes[24..].copy_from_slice(token_data.amount.to_le_bytes().as_slice());
        let hashed_token_data_with_hashed_values = TokenData::hash_with_hashed_values(
            &hashed_mint,
            &hashed_owner,
            &amount_bytes,
            &Some(&hashed_delegate),
        )
        .unwrap();
        assert_eq!(hashed_token_data, hashed_token_data_with_hashed_values);
        {
            let legacy_hash = legacy_hash(&token_data).unwrap();
            assert_eq!(hashed_token_data, legacy_hash);
        }
        let token_data = TokenData {
            mint: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            amount: rng.gen(),
            delegate: None,
            state: CompressedTokenAccountState::Initialized as u8,
            tlv: None,
        };
        let hashed_token_data = token_data.hash_v1().unwrap();
        let hashed_mint = hash_to_bn254_field_size_be(token_data.mint.to_bytes().as_slice());
        let hashed_owner = hash_to_bn254_field_size_be(token_data.owner.to_bytes().as_slice());
        let mut amount_bytes = [0u8; 32];
        amount_bytes[24..].copy_from_slice(token_data.amount.to_le_bytes().as_slice());
        let hashed_token_data_with_hashed_values: [u8; 32] =
            TokenData::hash_with_hashed_values(&hashed_mint, &hashed_owner, &amount_bytes, &None)
                .unwrap();
        assert_eq!(hashed_token_data, hashed_token_data_with_hashed_values);
        let legacy_hash = legacy_hash(&token_data).unwrap();
        assert_eq!(hashed_token_data, legacy_hash);
    }
}

#[test]
fn equivalency_of_hash_functions_iters_poseidon() {
    equivalency_of_hash_functions_rnd_iters::<10_000>();
}

#[test]
fn test_circuit_equivalence() {
    // Convert hex strings to Pubkeys
    let mint_pubkey = Pubkey::new_from_array([
        0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]);
    let owner_pubkey = Pubkey::new_from_array([
        0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]);
    let delegate_pubkey = Pubkey::new_from_array([
        0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]);

    let token_data = TokenData {
        mint: mint_pubkey,
        owner: owner_pubkey,
        amount: 1000000u64,
        delegate: Some(delegate_pubkey),
        state: CompressedTokenAccountState::Initialized as u8, // Using Frozen state to match our circuit test
        tlv: None,
    };

    // Calculate the hash with the Rust code
    let rust_hash = token_data.hash_v2().unwrap();

    let circuit_hash_str =
        "12698830169693734517877055378728747723888091986541703429186543307137690361131";
    use std::str::FromStr;
    let circuit_hash = BigUint::from_str(circuit_hash_str).unwrap().to_bytes_be();
    let rust_hash_string = BigUint::from_bytes_be(rust_hash.as_slice()).to_string();
    println!("Circuit hash string: {}", circuit_hash_str);
    println!("rust_hash_string {}", rust_hash_string);
    assert_eq!(rust_hash.to_vec(), circuit_hash);
}

#[test]
fn test_frozen_equivalence() {
    let token_data = TokenData {
        mint: Pubkey::new_unique(),
        owner: Pubkey::new_unique(),
        amount: 100,
        delegate: Some(Pubkey::new_unique()),
        state: CompressedTokenAccountState::Initialized as u8,
        tlv: None,
    };
    let hashed_mint = hash_to_bn254_field_size_be(token_data.mint.to_bytes().as_slice());
    let hashed_owner = hash_to_bn254_field_size_be(token_data.owner.to_bytes().as_slice());
    let hashed_delegate =
        hash_to_bn254_field_size_be(token_data.delegate.unwrap().to_bytes().as_slice());
    let mut amount_bytes = [0u8; 32];
    amount_bytes[24..].copy_from_slice(token_data.amount.to_le_bytes().as_slice());
    let hash = TokenData::hash_with_hashed_values(
        &hashed_mint,
        &hashed_owner,
        &amount_bytes,
        &Some(&hashed_delegate),
    )
    .unwrap();
    let other_hash = token_data.hash_v1().unwrap();
    assert_eq!(hash, other_hash);
}

#[test]
fn failing_tests_hashing() {
    let mut vec_previous_hashes = Vec::new();
    let token_data = TokenData {
        mint: Pubkey::new_unique(),
        owner: Pubkey::new_unique(),
        amount: 100,
        delegate: None,
        state: CompressedTokenAccountState::Initialized as u8,
        tlv: None,
    };
    let hashed_mint = hash_to_bn254_field_size_be(token_data.mint.to_bytes().as_slice());
    let hashed_owner = hash_to_bn254_field_size_be(token_data.owner.to_bytes().as_slice());
    let mut amount_bytes = [0u8; 32];
    amount_bytes[24..].copy_from_slice(token_data.amount.to_le_bytes().as_slice());
    let hash =
        TokenData::hash_with_hashed_values(&hashed_mint, &hashed_owner, &amount_bytes, &None)
            .unwrap();
    vec_previous_hashes.push(hash);
    // different mint
    let hashed_mint_2 = hash_to_bn254_field_size_be(Pubkey::new_unique().to_bytes().as_slice());
    let mut amount_bytes = [0u8; 32];
    amount_bytes[24..].copy_from_slice(token_data.amount.to_le_bytes().as_slice());
    let hash2 =
        TokenData::hash_with_hashed_values(&hashed_mint_2, &hashed_owner, &amount_bytes, &None)
            .unwrap();
    assert_to_previous_hashes(hash2, &mut vec_previous_hashes);

    // different owner
    let hashed_owner_2 = hash_to_bn254_field_size_be(Pubkey::new_unique().to_bytes().as_slice());
    let mut amount_bytes = [0u8; 32];
    amount_bytes[24..].copy_from_slice(token_data.amount.to_le_bytes().as_slice());
    let hash3 =
        TokenData::hash_with_hashed_values(&hashed_mint, &hashed_owner_2, &amount_bytes, &None)
            .unwrap();
    assert_to_previous_hashes(hash3, &mut vec_previous_hashes);

    // different amount
    let different_amount: u64 = 101;
    let mut different_amount_bytes = [0u8; 32];
    different_amount_bytes[24..].copy_from_slice(different_amount.to_le_bytes().as_slice());
    let hash4 = TokenData::hash_with_hashed_values(
        &hashed_mint,
        &hashed_owner,
        &different_amount_bytes,
        &None,
    )
    .unwrap();
    assert_to_previous_hashes(hash4, &mut vec_previous_hashes);

    // different delegate
    let delegate = Pubkey::new_unique();
    let hashed_delegate = hash_to_bn254_field_size_be(delegate.to_bytes().as_slice());
    let mut amount_bytes = [0u8; 32];
    amount_bytes[24..].copy_from_slice(token_data.amount.to_le_bytes().as_slice());
    let hash7 = TokenData::hash_with_hashed_values(
        &hashed_mint,
        &hashed_owner,
        &amount_bytes,
        &Some(&hashed_delegate),
    )
    .unwrap();

    assert_to_previous_hashes(hash7, &mut vec_previous_hashes);
    // different account state
    let mut token_data = token_data;
    token_data.state = CompressedTokenAccountState::Frozen as u8;
    let hash9 = token_data.hash_v1().unwrap();
    assert_to_previous_hashes(hash9, &mut vec_previous_hashes);
    // different account state with delegate
    token_data.delegate = Some(delegate);
    let hash10 = token_data.hash_v1().unwrap();
    assert_to_previous_hashes(hash10, &mut vec_previous_hashes);
}

fn assert_to_previous_hashes(hash: [u8; 32], previous_hashes: &mut Vec<[u8; 32]>) {
    for previous_hash in previous_hashes.iter() {
        assert_ne!(hash, *previous_hash);
    }
    println!("len previous hashes: {}", previous_hashes.len());
    previous_hashes.push(hash);
}

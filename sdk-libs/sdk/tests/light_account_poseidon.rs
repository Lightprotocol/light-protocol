//! Tests for Poseidon-based LightAccount (HASH_FLAT = false).
//!
//! Each test uses a single assert_eq against a complete expected struct.
//! Tests cover: new_init, new_mut, new_empty, new_close, new_burn
//!
//! Requires the `poseidon` feature flag.

#![cfg(feature = "poseidon")]

use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::{
    compressed_account::PackedMerkleContext,
    instruction_data::with_account_info::{CompressedAccountInfo, InAccountInfo, OutAccountInfo},
};
use light_sdk::{
    account::poseidon::LightAccount,
    instruction::{
        account_meta::{CompressedAccountMeta, CompressedAccountMetaBurn},
        PackedStateTreeInfo,
    },
    AnchorDiscriminator, LightDiscriminator, LightHasher,
};
use solana_pubkey::Pubkey;

/// Test struct for Poseidon hashing tests.
/// Fields > 31 bytes (like Pubkey) need the `#[hash]` attribute to hash them to field size.
/// Uses AnchorDiscriminator for Anchor compatibility (SHA256("account:TestPoseidonAccount")[0..8]).
#[derive(
    Clone,
    Debug,
    Default,
    LightHasher,
    AnchorDiscriminator,
    BorshSerialize,
    BorshDeserialize,
    PartialEq,
)]
pub struct TestPoseidonAccount {
    #[hash]
    pub owner: Pubkey,
    pub counter: u64,
}

// Hardcoded discriminator for TestPoseidonAccount (derived from AnchorDiscriminator)
// SHA256("account:TestPoseidonAccount")[0..8]
const TEST_POSEIDON_DISCRIMINATOR: [u8; 8] = [250, 202, 237, 234, 244, 147, 165, 166];

// Hardcoded Poseidon data hash for TestPoseidonAccount { owner: [1u8; 32], counter: 42 }
// Poseidon(hash_to_field(owner), counter)
const TEST_POSEIDON_DATA_HASH: [u8; 32] = [
    30, 49, 141, 11, 21, 190, 7, 27, 48, 25, 227, 164, 36, 37, 140, 76, 209, 159, 198, 111, 102,
    73, 56, 44, 165, 20, 220, 53, 47, 237, 64, 203,
];

// ============================================================================
// Hash Regression Test
// ============================================================================

/// Regression test ensuring Poseidon hashing remains stable.
#[test]
fn test_poseidon_hash_regression() {
    let owner = Pubkey::new_from_array([1u8; 32]);
    let counter = 42u64;
    let program_id = Pubkey::new_from_array([2u8; 32]);

    let tree_info = PackedStateTreeInfo {
        root_index: 0,
        prove_by_index: false,
        merkle_tree_pubkey_index: 0,
        queue_pubkey_index: 1,
        leaf_index: 100,
    };
    let account_meta = CompressedAccountMeta {
        tree_info,
        address: [3u8; 32],
        output_state_tree_index: 0,
    };
    let account_data = TestPoseidonAccount { owner, counter };

    let account =
        LightAccount::<TestPoseidonAccount>::new_mut(&program_id, &account_meta, account_data)
            .expect("Failed to create LightAccount");

    let input_info = account
        .in_account_info()
        .as_ref()
        .expect("Should have input");

    assert_eq!(
        input_info.data_hash, TEST_POSEIDON_DATA_HASH,
        "Poseidon data hash must match hardcoded value"
    );
}

// ============================================================================
// new_init Tests
// ============================================================================

/// Test new_init: creates account with output only (no input).
#[test]
fn test_new_init() {
    let program_id = Pubkey::new_from_array([2u8; 32]);
    let address = [3u8; 32];
    let output_tree_index = 5u8;

    let mut account = LightAccount::<TestPoseidonAccount>::new_init(
        &program_id,
        Some(address),
        output_tree_index,
    );

    // Verify no input (init accounts have no input)
    assert!(
        account.in_account_info().is_none(),
        "Init account should have no input"
    );

    // Verify output
    let expected_out = OutAccountInfo {
        discriminator: TEST_POSEIDON_DISCRIMINATOR,
        data_hash: [0u8; 32], // Default, will be computed on to_account_info
        output_merkle_tree_index: 5,
        lamports: 0,
        data: vec![],
    };
    assert_eq!(
        *account.out_account_info().as_ref().unwrap(),
        expected_out,
        "OutAccountInfo should match expected"
    );
}

// ============================================================================
// new_mut Tests
// ============================================================================

/// Test new_mut: creates account with both input and output.
#[test]
fn test_new_mut() {
    let owner = Pubkey::new_from_array([1u8; 32]);
    let counter = 42u64;
    let program_id = Pubkey::new_from_array([2u8; 32]);
    let address = [3u8; 32];

    let tree_info = PackedStateTreeInfo {
        root_index: 10,
        prove_by_index: false,
        merkle_tree_pubkey_index: 0,
        queue_pubkey_index: 1,
        leaf_index: 500,
    };
    let account_meta = CompressedAccountMeta {
        tree_info,
        address,
        output_state_tree_index: 2,
    };
    let account_data = TestPoseidonAccount { owner, counter };

    let mut account = LightAccount::<TestPoseidonAccount>::new_mut(
        &program_id,
        &account_meta,
        account_data.clone(),
    )
    .expect("Failed to create LightAccount");

    // Expected InAccountInfo
    let expected_in = InAccountInfo {
        discriminator: TEST_POSEIDON_DISCRIMINATOR,
        data_hash: TEST_POSEIDON_DATA_HASH,
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: 0,
            queue_pubkey_index: 1,
            leaf_index: 500,
            prove_by_index: false,
        },
        root_index: 10,
        lamports: 0,
    };
    assert_eq!(
        *account.in_account_info().as_ref().unwrap(),
        expected_in,
        "InAccountInfo should match expected"
    );

    // Expected OutAccountInfo
    let expected_out = OutAccountInfo {
        discriminator: TEST_POSEIDON_DISCRIMINATOR,
        data_hash: [0u8; 32], // Default, will be computed on to_account_info
        output_merkle_tree_index: 2,
        lamports: 0,
        data: vec![],
    };
    assert_eq!(
        *account.out_account_info().as_ref().unwrap(),
        expected_out,
        "OutAccountInfo should match expected"
    );
}

// ============================================================================
// new_empty Tests
// ============================================================================

/// Test new_empty: creates account with zeroed input hash (for address-only accounts).
#[test]
fn test_new_empty() {
    let program_id = Pubkey::new_from_array([2u8; 32]);
    let address = [3u8; 32];

    let tree_info = PackedStateTreeInfo {
        root_index: 5,
        prove_by_index: true,
        merkle_tree_pubkey_index: 1,
        queue_pubkey_index: 2,
        leaf_index: 200,
    };
    let account_meta = CompressedAccountMeta {
        tree_info,
        address,
        output_state_tree_index: 3,
    };

    let mut account = LightAccount::<TestPoseidonAccount>::new_empty(&program_id, &account_meta)
        .expect("Failed to create empty LightAccount");

    // Expected InAccountInfo with zeroed data_hash and discriminator
    // Note: root_index=0 because prove_by_index=true -> get_root_index returns None -> defaults to 0
    let expected_in = InAccountInfo {
        discriminator: [0u8; 8], // Zero for empty accounts
        data_hash: [0u8; 32],    // Zero for empty accounts
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: 1,
            queue_pubkey_index: 2,
            leaf_index: 200,
            prove_by_index: true,
        },
        root_index: 0, // 0 because prove_by_index=true -> root_index ignored
        lamports: 0,
    };
    assert_eq!(
        *account.in_account_info().as_ref().unwrap(),
        expected_in,
        "InAccountInfo for empty account should have zeroed data_hash and discriminator"
    );

    // Expected OutAccountInfo (discriminator is set for output)
    let expected_out = OutAccountInfo {
        discriminator: TEST_POSEIDON_DISCRIMINATOR, // Output has discriminator set
        data_hash: [0u8; 32],
        output_merkle_tree_index: 3,
        lamports: 0,
        data: vec![],
    };
    assert_eq!(
        *account.out_account_info().as_ref().unwrap(),
        expected_out,
        "OutAccountInfo for empty account should have discriminator set"
    );
}

// ============================================================================
// new_close Tests
// ============================================================================

/// Test new_close: creates account that will be closed (output with zeroed data).
#[test]
fn test_new_close() {
    let owner = Pubkey::new_from_array([1u8; 32]);
    let counter = 42u64;
    let program_id = Pubkey::new_from_array([2u8; 32]);
    let address = [3u8; 32];

    let tree_info = PackedStateTreeInfo {
        root_index: 0,
        prove_by_index: false,
        merkle_tree_pubkey_index: 0,
        queue_pubkey_index: 1,
        leaf_index: 100,
    };
    let account_meta = CompressedAccountMeta {
        tree_info,
        address,
        output_state_tree_index: 0,
    };
    let account_data = TestPoseidonAccount { owner, counter };

    let account =
        LightAccount::<TestPoseidonAccount>::new_close(&program_id, &account_meta, account_data)
            .expect("Failed to create close LightAccount");

    // Verify to_account_info produces zeroed output
    let account_info = account
        .to_account_info()
        .expect("Should convert to account info");

    // Expected CompressedAccountInfo for closed account
    let expected = CompressedAccountInfo {
        address: Some(address),
        input: Some(InAccountInfo {
            discriminator: TEST_POSEIDON_DISCRIMINATOR,
            data_hash: TEST_POSEIDON_DATA_HASH,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: 0,
                queue_pubkey_index: 1,
                leaf_index: 100,
                prove_by_index: false,
            },
            root_index: 0,
            lamports: 0,
        }),
        output: Some(OutAccountInfo {
            discriminator: [0u8; 8], // Zeroed for close
            data_hash: [0u8; 32],    // Zeroed for close
            output_merkle_tree_index: 0,
            lamports: 0,
            data: vec![], // Empty for close
        }),
    };
    assert_eq!(
        account_info, expected,
        "Closed account should have zeroed output data_hash and discriminator"
    );
}

// ============================================================================
// new_burn Tests
// ============================================================================

/// Test new_burn: creates account with input only (no output).
#[test]
fn test_new_burn() {
    let owner = Pubkey::new_from_array([1u8; 32]);
    let counter = 42u64;
    let program_id = Pubkey::new_from_array([2u8; 32]);
    let address = [3u8; 32];

    let tree_info = PackedStateTreeInfo {
        root_index: 7,
        prove_by_index: true,
        merkle_tree_pubkey_index: 2,
        queue_pubkey_index: 3,
        leaf_index: 999,
    };
    let account_meta = CompressedAccountMetaBurn { tree_info, address };
    let account_data = TestPoseidonAccount { owner, counter };

    let mut account =
        LightAccount::<TestPoseidonAccount>::new_burn(&program_id, &account_meta, account_data)
            .expect("Failed to create burn LightAccount");

    // Expected InAccountInfo
    // Note: root_index=0 because prove_by_index=true -> get_root_index returns None -> defaults to 0
    let expected_in = InAccountInfo {
        discriminator: TEST_POSEIDON_DISCRIMINATOR,
        data_hash: TEST_POSEIDON_DATA_HASH,
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: 2,
            queue_pubkey_index: 3,
            leaf_index: 999,
            prove_by_index: true,
        },
        root_index: 0, // 0 because prove_by_index=true -> root_index ignored
        lamports: 0,
    };
    assert_eq!(
        *account.in_account_info().as_ref().unwrap(),
        expected_in,
        "InAccountInfo for burn should match expected"
    );

    // Verify no output (burn accounts have no output)
    assert!(
        account.out_account_info().is_none(),
        "Burn account should have no output"
    );
}

// ============================================================================
// to_account_info Tests
// ============================================================================

/// Test to_account_info for normal mutable account.
#[test]
fn test_to_account_info_mut() {
    let owner = Pubkey::new_from_array([1u8; 32]);
    let counter = 42u64;
    let program_id = Pubkey::new_from_array([2u8; 32]);
    let address = [3u8; 32];

    let tree_info = PackedStateTreeInfo {
        root_index: 0,
        prove_by_index: false,
        merkle_tree_pubkey_index: 0,
        queue_pubkey_index: 1,
        leaf_index: 100,
    };
    let account_meta = CompressedAccountMeta {
        tree_info,
        address,
        output_state_tree_index: 5,
    };
    let account_data = TestPoseidonAccount { owner, counter };

    let account = LightAccount::<TestPoseidonAccount>::new_mut(
        &program_id,
        &account_meta,
        account_data.clone(),
    )
    .expect("Failed to create LightAccount");

    let account_info = account
        .to_account_info()
        .expect("Should convert to account info");

    // Expected serialized data
    let expected_data = account_data.try_to_vec().expect("Should serialize");

    // Expected CompressedAccountInfo
    let expected = CompressedAccountInfo {
        address: Some(address),
        input: Some(InAccountInfo {
            discriminator: TEST_POSEIDON_DISCRIMINATOR,
            data_hash: TEST_POSEIDON_DATA_HASH,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: 0,
                queue_pubkey_index: 1,
                leaf_index: 100,
                prove_by_index: false,
            },
            root_index: 0,
            lamports: 0,
        }),
        output: Some(OutAccountInfo {
            discriminator: TEST_POSEIDON_DISCRIMINATOR,
            data_hash: TEST_POSEIDON_DATA_HASH, // Same hash for unchanged data
            output_merkle_tree_index: 5,
            lamports: 0,
            data: expected_data,
        }),
    };
    assert_eq!(
        account_info, expected,
        "to_account_info should produce expected CompressedAccountInfo"
    );
}

// ============================================================================
// Helper Method Tests
// ============================================================================

/// Test discriminator() method returns correct hardcoded value.
#[test]
fn test_discriminator_method() {
    let program_id = Pubkey::new_from_array([2u8; 32]);
    let account = LightAccount::<TestPoseidonAccount>::new_init(&program_id, None, 0);

    assert_eq!(
        *account.discriminator(),
        TEST_POSEIDON_DISCRIMINATOR,
        "discriminator() should return hardcoded discriminator"
    );
}

/// Test lamports() and lamports_mut() methods.
#[test]
fn test_lamports_methods() {
    let program_id = Pubkey::new_from_array([2u8; 32]);
    let tree_info = PackedStateTreeInfo {
        root_index: 0,
        prove_by_index: false,
        merkle_tree_pubkey_index: 0,
        queue_pubkey_index: 1,
        leaf_index: 100,
    };
    let account_meta = light_sdk::instruction::account_meta::CompressedAccountMetaWithLamports {
        tree_info,
        lamports: 1000,
        address: [3u8; 32],
        output_state_tree_index: 0,
    };

    let mut account = LightAccount::<TestPoseidonAccount>::new_mut(
        &program_id,
        &account_meta,
        TestPoseidonAccount::default(),
    )
    .expect("Failed to create LightAccount");

    assert_eq!(account.lamports(), 1000, "Initial lamports should be 1000");

    *account.lamports_mut() = 2000;
    assert_eq!(
        account.lamports(),
        2000,
        "Lamports should be updated to 2000"
    );
}

/// Test Deref and DerefMut to access inner account data.
#[test]
fn test_deref() {
    let program_id = Pubkey::new_from_array([2u8; 32]);
    let owner = Pubkey::new_from_array([1u8; 32]);
    let counter = 42u64;
    let tree_info = PackedStateTreeInfo {
        root_index: 0,
        prove_by_index: false,
        merkle_tree_pubkey_index: 0,
        queue_pubkey_index: 1,
        leaf_index: 100,
    };
    let account_meta = CompressedAccountMeta {
        tree_info,
        address: [3u8; 32],
        output_state_tree_index: 0,
    };

    let mut account = LightAccount::<TestPoseidonAccount>::new_mut(
        &program_id,
        &account_meta,
        TestPoseidonAccount { owner, counter },
    )
    .expect("Failed to create LightAccount");

    // Test Deref - access inner fields
    assert_eq!(account.owner, owner, "Deref should give access to owner");
    assert_eq!(
        account.counter, counter,
        "Deref should give access to counter"
    );

    // Test DerefMut - modify inner fields
    account.counter = 100;
    assert_eq!(
        account.counter, 100,
        "DerefMut should allow modifying counter"
    );
}

/// Test remove_data functionality.
#[test]
fn test_remove_data() {
    let program_id = Pubkey::new_from_array([2u8; 32]);
    let tree_info = PackedStateTreeInfo {
        root_index: 0,
        prove_by_index: false,
        merkle_tree_pubkey_index: 0,
        queue_pubkey_index: 1,
        leaf_index: 100,
    };
    let account_meta = CompressedAccountMeta {
        tree_info,
        address: [3u8; 32],
        output_state_tree_index: 0,
    };

    let mut account = LightAccount::<TestPoseidonAccount>::new_mut(
        &program_id,
        &account_meta,
        TestPoseidonAccount::default(),
    )
    .expect("Failed to create LightAccount");

    account.remove_data();

    let account_info = account
        .to_account_info()
        .expect("Should convert to account info");

    let output = account_info.output.expect("Should have output");

    // After remove_data, output should have zeroed hash and discriminator
    let expected_output = OutAccountInfo {
        discriminator: [0u8; 8],
        data_hash: [0u8; 32],
        output_merkle_tree_index: 0,
        lamports: 0,
        data: vec![],
    };
    assert_eq!(
        output, expected_output,
        "Output after remove_data should have zeroed data_hash and discriminator"
    );
}

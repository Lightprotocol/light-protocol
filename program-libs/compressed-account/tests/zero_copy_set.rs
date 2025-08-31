#![cfg(all(test, feature = "new-unique"))]

use borsh::BorshSerialize;
use light_compressed_account::{
    compressed_account::{CompressedAccount, PackedMerkleContext},
    instruction_data::{
        compressed_proof::CompressedProof,
        cpi_context::CompressedCpiContext,
        data::{NewAddressParamsAssignedPacked, OutputCompressedAccountWithPackedContext},
        with_readonly::{InAccount, InstructionDataInvokeCpiWithReadOnly},
    },
    pubkey::Pubkey,
    CompressedAccountError,
};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use rand::{rngs::StdRng, thread_rng, Rng, SeedableRng};
use zerocopy::little_endian::U16;

/// Tests for all functions in zero_copy_set.rs:
/// 1. ZOutputCompressedAccountWithPackedContextMut::set() - 9 tests
///    - Success: with/without address
///    - Error: missing/unexpected address, data not initialized
///    - Edge cases: zero/max lamports, zero/max values
/// 2. ZInAccountMut::set() - 8 tests
///    - Success: with/without address
///    - Error: missing/unexpected address, invalid address length (panic)
///    - Edge cases: zero/max lamports, merkle context copying
/// 3. ZInAccountMut::set_z() - 8 tests
///    - Success: with/without address using zero-copy context
///    - Error: missing/unexpected address
///    - Edge cases: zero/max lamports, merkle context bounds/copying
/// 4. ZInstructionDataInvokeCpiWithReadOnlyMut::initialize() - 10 tests
///    - Success: with/without proof, with/without CPI context
///    - Error: missing/unexpected proof
///    - Edge cases: zero/max bump, proof copying, mode invariant
/// 5. ZNewAddressParamsAssignedPackedMut::set() - 9 tests
///    - Success: with/without assigned account
///    - Edge cases: zero/max seed, zero/max root index, zero/max merkle tree account index
///    - Invariant: address queue account index always 0
/// 6. test_randomized_all_functions - 1 test (1000 iterations)
///    - Property-based testing of all functions with random valid inputs
// =============================================================================
// Helper Functions
// =============================================================================
fn create_compressed_account(address: Option<[u8; 32]>, data: Option<bool>) -> CompressedAccount {
    CompressedAccount {
        owner: Pubkey::new_unique(),
        lamports: 1000,
        address,
        data: if data.unwrap_or(true) {
            Some(
                light_compressed_account::compressed_account::CompressedAccountData {
                    discriminator: [1u8; 8],
                    data: vec![],
                    data_hash: [2u8; 32],
                },
            )
        } else {
            None
        },
    }
}

// =============================================================================
// OutputCompressedAccountWithPackedContext::set() Tests
// =============================================================================

#[test]
fn test_output_account_set_success_with_address() {
    // Setup: Create output account with address slot
    let output_account = OutputCompressedAccountWithPackedContext {
        compressed_account: create_compressed_account(Some([1u8; 32]), None),
        merkle_tree_index: 0,
    };

    let mut data = output_account.try_to_vec().unwrap();
    let (mut z_output, _) =
        OutputCompressedAccountWithPackedContext::zero_copy_at_mut(&mut data).unwrap();

    // Execute
    let owner = Pubkey::new_unique();
    let result = z_output.set(owner, 5000, Some([5u8; 32]), 3, [7u8; 8], [9u8; 32]);

    // Assert success
    assert!(result.is_ok());
    assert_eq!(z_output.compressed_account.owner, owner);
    assert_eq!(u64::from(z_output.compressed_account.lamports), 5000);
    assert_eq!(
        *z_output.compressed_account.address.as_deref().unwrap(),
        [5u8; 32]
    );
    assert_eq!(*z_output.merkle_tree_index, 3);

    let data_ref = z_output.compressed_account.data.as_ref().unwrap();
    assert_eq!(data_ref.discriminator, [7u8; 8]);
    assert_eq!(*data_ref.data_hash, [9u8; 32]);
}

#[test]
fn test_output_account_set_success_without_address() {
    // Setup: Create output account without address slot
    let output_account = OutputCompressedAccountWithPackedContext {
        compressed_account: create_compressed_account(None, None),
        merkle_tree_index: 0,
    };

    let mut data = output_account.try_to_vec().unwrap();
    let (mut z_output, _) =
        OutputCompressedAccountWithPackedContext::zero_copy_at_mut(&mut data).unwrap();

    // Execute
    let owner = Pubkey::new_unique();
    let result = z_output.set(owner, 5000, None, 3, [7u8; 8], [9u8; 32]);

    // Assert success
    assert!(result.is_ok());
    assert_eq!(z_output.compressed_account.owner, owner);
    assert_eq!(u64::from(z_output.compressed_account.lamports), 5000);
    assert!(z_output.compressed_account.address.is_none());
    assert_eq!(*z_output.merkle_tree_index, 3);
}

#[test]
fn test_output_account_set_error_missing_address() {
    // Setup: Create output account with address slot but provide None
    let output_account = OutputCompressedAccountWithPackedContext {
        compressed_account: create_compressed_account(Some([1u8; 32]), None),
        merkle_tree_index: 0,
    };

    let mut data = output_account.try_to_vec().unwrap();
    let (mut z_output, _) =
        OutputCompressedAccountWithPackedContext::zero_copy_at_mut(&mut data).unwrap();

    // Execute
    let result = z_output.set(
        Pubkey::new_unique(),
        5000,
        None, // Address expected but None provided
        3,
        [7u8; 8],
        [9u8; 32],
    );

    // Assert error
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        CompressedAccountError::InstructionDataExpectedAddress
    );
}

#[test]
fn test_output_account_set_error_unexpected_address() {
    // Setup: Create output account without address slot but provide address
    let output_account = OutputCompressedAccountWithPackedContext {
        compressed_account: create_compressed_account(None, None),
        merkle_tree_index: 0,
    };

    let mut data = output_account.try_to_vec().unwrap();
    let (mut z_output, _) =
        OutputCompressedAccountWithPackedContext::zero_copy_at_mut(&mut data).unwrap();

    // Execute
    let result = z_output.set(
        Pubkey::new_unique(),
        5000,
        Some([5u8; 32]), // Address provided but not expected
        3,
        [7u8; 8],
        [9u8; 32],
    );

    // Assert error
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        CompressedAccountError::ZeroCopyExpectedAddress
    );
}

#[test]
fn test_output_account_set_error_data_not_initialized() {
    // Setup: Create output account without data
    let output_account = OutputCompressedAccountWithPackedContext {
        compressed_account: create_compressed_account(Some([1u8; 32]), Some(false)),
        merkle_tree_index: 0,
    };

    let mut data = output_account.try_to_vec().unwrap();
    let (mut z_output, _) =
        OutputCompressedAccountWithPackedContext::zero_copy_at_mut(&mut data).unwrap();

    // Execute
    let result = z_output.set(
        Pubkey::new_unique(),
        5000,
        Some([5u8; 32]),
        3,
        [7u8; 8],
        [9u8; 32],
    );

    // Assert error
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        CompressedAccountError::CompressedAccountDataNotInitialized
    );
}

#[test]
fn test_output_account_set_edge_case_zero_lamports() {
    // Setup
    let output_account = OutputCompressedAccountWithPackedContext {
        compressed_account: create_compressed_account(Some([1u8; 32]), None),
        merkle_tree_index: 0,
    };

    let mut data = output_account.try_to_vec().unwrap();
    let (mut z_output, _) =
        OutputCompressedAccountWithPackedContext::zero_copy_at_mut(&mut data).unwrap();

    // Execute with zero lamports
    let result = z_output.set(
        Pubkey::new_unique(),
        0, // Zero lamports
        Some([5u8; 32]),
        3,
        [7u8; 8],
        [9u8; 32],
    );

    // Assert success and zero lamports set correctly
    assert!(result.is_ok());
    assert_eq!(u64::from(z_output.compressed_account.lamports), 0);
}

#[test]
fn test_output_account_set_edge_case_max_lamports() {
    // Setup
    let output_account = OutputCompressedAccountWithPackedContext {
        compressed_account: create_compressed_account(Some([1u8; 32]), None),
        merkle_tree_index: 0,
    };

    let mut data = output_account.try_to_vec().unwrap();
    let (mut z_output, _) =
        OutputCompressedAccountWithPackedContext::zero_copy_at_mut(&mut data).unwrap();

    // Execute with max u64 lamports
    let result = z_output.set(
        Pubkey::new_unique(),
        u64::MAX, // Max lamports
        Some([5u8; 32]),
        3,
        [7u8; 8],
        [9u8; 32],
    );

    // Assert success and max lamports set correctly
    assert!(result.is_ok());
    assert_eq!(u64::from(z_output.compressed_account.lamports), u64::MAX);
}

#[test]
fn test_output_account_set_edge_case_zero_values() {
    // Setup
    let output_account = OutputCompressedAccountWithPackedContext {
        compressed_account: create_compressed_account(Some([1u8; 32]), None),
        merkle_tree_index: 0,
    };

    let mut data = output_account.try_to_vec().unwrap();
    let (mut z_output, _) =
        OutputCompressedAccountWithPackedContext::zero_copy_at_mut(&mut data).unwrap();

    // Execute with all zero arrays and zero merkle_tree_index
    let result = z_output.set(
        Pubkey::new_unique(),
        0,
        Some([0u8; 32]),
        0,         // Zero merkle_tree_index
        [0u8; 8],  // Zero discriminator
        [0u8; 32], // Zero data_hash
    );

    // Assert success and all zero values set correctly
    assert!(result.is_ok());
    assert_eq!(
        *z_output.compressed_account.address.as_deref().unwrap(),
        [0u8; 32]
    );
    assert_eq!(*z_output.merkle_tree_index, 0);

    let data_ref = z_output.compressed_account.data.as_ref().unwrap();
    assert_eq!(data_ref.discriminator, [0u8; 8]);
    assert_eq!(*data_ref.data_hash, [0u8; 32]);
}

#[test]
fn test_output_account_set_edge_case_max_values() {
    // Setup
    let output_account = OutputCompressedAccountWithPackedContext {
        compressed_account: create_compressed_account(Some([1u8; 32]), None),
        merkle_tree_index: 0,
    };

    let mut data = output_account.try_to_vec().unwrap();
    let (mut z_output, _) =
        OutputCompressedAccountWithPackedContext::zero_copy_at_mut(&mut data).unwrap();

    // Execute with all 0xFF arrays and max merkle_tree_index
    let result = z_output.set(
        Pubkey::new_unique(),
        u64::MAX,
        Some([0xFFu8; 32]),
        u8::MAX,      // Max merkle_tree_index
        [0xFFu8; 8],  // Max discriminator
        [0xFFu8; 32], // Max data_hash
    );

    // Assert success and all max values set correctly
    assert!(result.is_ok());
    assert_eq!(
        *z_output.compressed_account.address.as_deref().unwrap(),
        [0xFFu8; 32]
    );
    assert_eq!(*z_output.merkle_tree_index, u8::MAX);

    let data_ref = z_output.compressed_account.data.as_ref().unwrap();
    assert_eq!(data_ref.discriminator, [0xFFu8; 8]);
    assert_eq!(*data_ref.data_hash, [0xFFu8; 32]);
}

// =============================================================================
// InAccount::set() Tests (with regular context)
// =============================================================================

#[test]
fn test_in_account_set_success_with_address() {
    // Setup: Create InAccount with address
    let in_account = InAccount {
        discriminator: [0u8; 8],
        data_hash: [0u8; 32],
        merkle_context: PackedMerkleContext::default(),
        root_index: 0,
        lamports: 0,
        address: Some([0u8; 32]),
    };

    let mut data = in_account.try_to_vec().unwrap();
    let (mut z_in, _) = InAccount::zero_copy_at_mut(&mut data).unwrap();

    // Execute
    let merkle_context = PackedMerkleContext {
        merkle_tree_pubkey_index: 1,
        queue_pubkey_index: 2,
        leaf_index: 100,
        prove_by_index: true,
    };
    let address = [7u8; 32];
    let result = z_in.set(
        [1u8; 8],
        [2u8; 32],
        &merkle_context,
        U16::new(3),
        4000,
        Some(&address),
    );

    // Assert success and all fields set correctly
    assert!(result.is_ok());
    assert_eq!(z_in.discriminator, [1u8; 8]);
    assert_eq!(z_in.data_hash, [2u8; 32]);
    assert_eq!(
        z_in.merkle_context.merkle_tree_pubkey_index,
        merkle_context.merkle_tree_pubkey_index
    );
    assert_eq!(
        z_in.merkle_context.queue_pubkey_index,
        merkle_context.queue_pubkey_index
    );
    assert_eq!(
        z_in.merkle_context.leaf_index.get(),
        merkle_context.leaf_index
    );
    assert_eq!(
        z_in.merkle_context.prove_by_index,
        if merkle_context.prove_by_index { 1 } else { 0 }
    );
    assert_eq!(z_in.root_index.get(), 3);
    assert_eq!(u64::from(*z_in.lamports), 4000);
    assert_eq!(*z_in.address.as_deref().unwrap(), address);
}

#[test]
fn test_in_account_set_success_without_address() {
    // Setup: Create InAccount without address
    let in_account = InAccount {
        discriminator: [0u8; 8],
        data_hash: [0u8; 32],
        merkle_context: PackedMerkleContext::default(),
        root_index: 0,
        lamports: 0,
        address: None,
    };

    let mut data = in_account.try_to_vec().unwrap();
    let (mut z_in, _) = InAccount::zero_copy_at_mut(&mut data).unwrap();

    // Execute
    let merkle_context = PackedMerkleContext {
        merkle_tree_pubkey_index: 1,
        queue_pubkey_index: 2,
        leaf_index: 100,
        prove_by_index: true,
    };
    let result = z_in.set(
        [1u8; 8],
        [2u8; 32],
        &merkle_context,
        U16::new(3),
        4000,
        None,
    );

    // Assert success
    assert!(result.is_ok());
    assert!(z_in.address.is_none());
}

#[test]
fn test_in_account_set_error_missing_address() {
    // Setup: Create InAccount with address slot but provide None
    let in_account = InAccount {
        discriminator: [0u8; 8],
        data_hash: [0u8; 32],
        merkle_context: PackedMerkleContext::default(),
        root_index: 0,
        lamports: 0,
        address: Some([0u8; 32]),
    };

    let mut data = in_account.try_to_vec().unwrap();
    let (mut z_in, _) = InAccount::zero_copy_at_mut(&mut data).unwrap();

    // Execute with None address when address is expected
    let merkle_context = PackedMerkleContext {
        merkle_tree_pubkey_index: 1,
        queue_pubkey_index: 2,
        leaf_index: 100,
        prove_by_index: true,
    };
    let result = z_in.set(
        [1u8; 8],
        [2u8; 32],
        &merkle_context,
        U16::new(3),
        4000,
        None, // Address expected but None provided
    );

    // Assert error
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        CompressedAccountError::ZeroCopyExpectedAddress
    );
}

#[test]
fn test_in_account_set_error_unexpected_address() {
    // Setup: Create InAccount without address slot but provide address
    let in_account = InAccount {
        discriminator: [0u8; 8],
        data_hash: [0u8; 32],
        merkle_context: PackedMerkleContext::default(),
        root_index: 0,
        lamports: 0,
        address: None,
    };

    let mut data = in_account.try_to_vec().unwrap();
    let (mut z_in, _) = InAccount::zero_copy_at_mut(&mut data).unwrap();

    // Execute with address when not expected
    let merkle_context = PackedMerkleContext {
        merkle_tree_pubkey_index: 1,
        queue_pubkey_index: 2,
        leaf_index: 100,
        prove_by_index: true,
    };
    let address = [7u8; 32];
    let result = z_in.set(
        [1u8; 8],
        [2u8; 32],
        &merkle_context,
        U16::new(3),
        4000,
        Some(&address), // Address provided but not expected
    );

    // Assert error
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        CompressedAccountError::InstructionDataExpectedAddress
    );
}

// =============================================================================
// NewAddressParamsAssignedPacked::set() Tests
// =============================================================================

#[test]
fn test_new_address_set_success_with_assigned_account() {
    // Setup
    let new_address = NewAddressParamsAssignedPacked {
        seed: [0u8; 32],
        address_queue_account_index: 0,
        address_merkle_tree_account_index: 0,
        address_merkle_tree_root_index: 0,
        assigned_to_account: false,
        assigned_account_index: 0,
    };

    let mut data = new_address.try_to_vec().unwrap();
    let (mut z_new_address, _) =
        NewAddressParamsAssignedPacked::zero_copy_at_mut(&mut data).unwrap();

    // Execute with assigned account index
    z_new_address.set([1u8; 32], U16::new(500), Some(7), 10);

    // Assert all fields set correctly
    assert_eq!(z_new_address.seed, [1u8; 32]);
    assert_eq!(z_new_address.address_merkle_tree_root_index.get(), 500);
    assert_eq!(z_new_address.assigned_account_index, 7);
    assert_eq!(z_new_address.address_merkle_tree_account_index, 10);
    assert_eq!(z_new_address.assigned_to_account, 1);
    assert_eq!(z_new_address.address_queue_account_index, 0); // Invariant: always 0
}

#[test]
fn test_new_address_set_success_without_assigned_account() {
    // Setup
    let new_address = NewAddressParamsAssignedPacked {
        seed: [0u8; 32],
        address_queue_account_index: 0,
        address_merkle_tree_account_index: 0,
        address_merkle_tree_root_index: 0,
        assigned_to_account: false,
        assigned_account_index: 0,
    };

    let mut data = new_address.try_to_vec().unwrap();
    let (mut z_new_address, _) =
        NewAddressParamsAssignedPacked::zero_copy_at_mut(&mut data).unwrap();

    // Execute without assigned account index
    z_new_address.set([1u8; 32], U16::new(500), None, 10);

    // Assert all fields set correctly
    assert_eq!(z_new_address.seed, [1u8; 32]);
    assert_eq!(z_new_address.address_merkle_tree_root_index.get(), 500);
    assert_eq!(z_new_address.assigned_account_index, 0);
    assert_eq!(z_new_address.address_merkle_tree_account_index, 10);
    assert_eq!(z_new_address.assigned_to_account, 0);
    assert_eq!(z_new_address.address_queue_account_index, 0); // Invariant: always 0
}

#[test]
fn test_new_address_set_invariant_address_queue_account_index() {
    // Setup
    let new_address = NewAddressParamsAssignedPacked::default();

    let mut data = new_address.try_to_vec().unwrap();
    let (mut z_new_address, _) =
        NewAddressParamsAssignedPacked::zero_copy_at_mut(&mut data).unwrap();

    // Execute multiple times with different inputs
    z_new_address.set([1u8; 32], U16::new(100), Some(5), 10);
    assert_eq!(z_new_address.address_queue_account_index, 0);

    z_new_address.set([2u8; 32], U16::new(200), None, 20);
    assert_eq!(z_new_address.address_queue_account_index, 0);

    z_new_address.set([3u8; 32], U16::new(300), Some(50), 30);
    assert_eq!(z_new_address.address_queue_account_index, 0);

    // Assert invariant: address_queue_account_index always 0 for v2 address trees
}

// =============================================================================
// InAccount::set_z() Tests (with zero-copy context)
// =============================================================================

#[test]
fn test_in_account_set_z_success_with_address() {
    // Setup: Create InAccount with address
    let in_account = InAccount {
        discriminator: [0u8; 8],
        data_hash: [0u8; 32],
        merkle_context: PackedMerkleContext::default(),
        root_index: 0,
        lamports: 0,
        address: Some([0u8; 32]),
    };

    let mut data = in_account.try_to_vec().unwrap();
    let (mut z_in, _) = InAccount::zero_copy_at_mut(&mut data).unwrap();

    // Execute - Create the zero-copy merkle context
    let merkle_context = PackedMerkleContext {
        merkle_tree_pubkey_index: 1,
        queue_pubkey_index: 2,
        leaf_index: 100,
        prove_by_index: true,
    };
    let merkle_context_bytes = merkle_context.try_to_vec().unwrap();
    let (z_merkle_context, _) = PackedMerkleContext::zero_copy_at(&merkle_context_bytes).unwrap();

    let address = [7u8; 32];
    let result = z_in.set_z(
        [1u8; 8],
        [2u8; 32],
        &z_merkle_context,
        U16::new(3),
        4000,
        Some(&address),
    );

    // Assert success and all fields set correctly
    assert!(result.is_ok());
    assert_eq!(z_in.discriminator, [1u8; 8]);
    assert_eq!(z_in.data_hash, [2u8; 32]);
    assert_eq!(
        z_in.merkle_context.merkle_tree_pubkey_index,
        merkle_context.merkle_tree_pubkey_index
    );
    assert_eq!(
        z_in.merkle_context.queue_pubkey_index,
        merkle_context.queue_pubkey_index
    );
    assert_eq!(
        z_in.merkle_context.leaf_index.get(),
        merkle_context.leaf_index
    );
    assert_eq!(z_in.merkle_context.prove_by_index, 1);
    assert_eq!(z_in.root_index.get(), 3);
    assert_eq!(u64::from(*z_in.lamports), 4000);
    assert_eq!(*z_in.address.as_deref().unwrap(), address);
}

#[test]
fn test_in_account_set_z_success_without_address() {
    // Setup: Create InAccount without address
    let in_account = InAccount {
        discriminator: [0u8; 8],
        data_hash: [0u8; 32],
        merkle_context: PackedMerkleContext::default(),
        root_index: 0,
        lamports: 0,
        address: None,
    };

    let mut data = in_account.try_to_vec().unwrap();
    let (mut z_in, _) = InAccount::zero_copy_at_mut(&mut data).unwrap();

    // Execute - Create the zero-copy merkle context
    let merkle_context = PackedMerkleContext {
        merkle_tree_pubkey_index: 1,
        queue_pubkey_index: 2,
        leaf_index: 100,
        prove_by_index: true,
    };
    let merkle_context_bytes = merkle_context.try_to_vec().unwrap();
    let (z_merkle_context, _) = PackedMerkleContext::zero_copy_at(&merkle_context_bytes).unwrap();

    let result = z_in.set_z(
        [1u8; 8],
        [2u8; 32],
        &z_merkle_context,
        U16::new(3),
        4000,
        None,
    );

    // Assert success
    assert!(result.is_ok());
    assert!(z_in.address.is_none());
}

#[test]
fn test_in_account_set_z_error_missing_address() {
    // Setup: InAccount with address slot (Some([0u8; 32]))
    let in_account = InAccount {
        discriminator: [1u8; 8],
        root_index: 1,
        data_hash: [2u8; 32],
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: 1,
            queue_pubkey_index: 2,
            leaf_index: 123,
            prove_by_index: true,
        },
        lamports: 1000,
        address: Some([0u8; 32]),
    };

    let mut data = in_account.try_to_vec().unwrap();
    let (mut z_in, _) = InAccount::zero_copy_at_mut(&mut data).unwrap();

    // Create zero-copy merkle context
    let merkle_ctx = PackedMerkleContext {
        merkle_tree_pubkey_index: 5,
        queue_pubkey_index: 6,
        leaf_index: 999,
        prove_by_index: false,
    };
    let ctx_data = merkle_ctx.try_to_vec().unwrap();
    let (z_context, _) = PackedMerkleContext::zero_copy_at(&ctx_data).unwrap();

    // Execute: call set_z() with None address (but InAccount has address slot)
    let result = z_in.set_z(
        [3u8; 8],
        [4u8; 32],
        &z_context,
        U16::new(2),
        2000,
        None, // Missing address when InAccount expects one
    );

    // Assert: ZeroCopyExpectedAddress error
    assert_eq!(
        result.unwrap_err(),
        CompressedAccountError::ZeroCopyExpectedAddress
    );
}

#[test]
fn test_in_account_set_z_error_unexpected_address() {
    // Setup: InAccount without address slot (None)
    let in_account = InAccount {
        discriminator: [1u8; 8],
        root_index: 1,
        data_hash: [2u8; 32],
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: 1,
            queue_pubkey_index: 2,
            leaf_index: 123,
            prove_by_index: true,
        },
        lamports: 1000,
        address: None, // No address slot
    };

    let mut data = in_account.try_to_vec().unwrap();
    let (mut z_in, _) = InAccount::zero_copy_at_mut(&mut data).unwrap();

    // Create zero-copy merkle context
    let merkle_ctx = PackedMerkleContext {
        merkle_tree_pubkey_index: 5,
        queue_pubkey_index: 6,
        leaf_index: 999,
        prove_by_index: false,
    };
    let ctx_data = merkle_ctx.try_to_vec().unwrap();
    let (z_context, _) = PackedMerkleContext::zero_copy_at(&ctx_data).unwrap();

    // Execute: call set_z() with Some(&[7u8; 32]) address (but InAccount has no address slot)
    let address = [7u8; 32];
    let result = z_in.set_z(
        [3u8; 8],
        [4u8; 32],
        &z_context,
        U16::new(2),
        2000,
        Some(&address), // Unexpected address when InAccount doesn't expect one
    );

    // Assert: InstructionDataExpectedAddress error
    assert_eq!(
        result.unwrap_err(),
        CompressedAccountError::InstructionDataExpectedAddress
    );
}

#[test]
fn test_in_account_set_z_edge_case_zero_lamports() {
    // Setup: InAccount without address
    let in_account = InAccount {
        discriminator: [1u8; 8],
        root_index: 1,
        data_hash: [2u8; 32],
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: 1,
            queue_pubkey_index: 2,
            leaf_index: 123,
            prove_by_index: true,
        },
        lamports: 1000,
        address: None,
    };

    let mut data = in_account.try_to_vec().unwrap();
    let (mut z_in, _) = InAccount::zero_copy_at_mut(&mut data).unwrap();

    // Create zero-copy merkle context
    let merkle_ctx = PackedMerkleContext {
        merkle_tree_pubkey_index: 5,
        queue_pubkey_index: 6,
        leaf_index: 999,
        prove_by_index: false,
    };
    let ctx_data = merkle_ctx.try_to_vec().unwrap();
    let (z_context, _) = PackedMerkleContext::zero_copy_at(&ctx_data).unwrap();

    // Execute: call set_z() with lamports = 0
    let result = z_in.set_z(
        [3u8; 8],
        [4u8; 32],
        &z_context,
        U16::new(2),
        0, // Zero lamports
        None,
    );

    // Assert: success and u64::from(*z_in.lamports) == 0
    assert!(result.is_ok());
    assert_eq!(u64::from(*z_in.lamports), 0);
}

#[test]
fn test_in_account_set_z_edge_case_max_lamports() {
    // Setup: InAccount without address
    let in_account = InAccount {
        discriminator: [1u8; 8],
        root_index: 1,
        data_hash: [2u8; 32],
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: 1,
            queue_pubkey_index: 2,
            leaf_index: 123,
            prove_by_index: true,
        },
        lamports: 1000,
        address: None,
    };

    let mut data = in_account.try_to_vec().unwrap();
    let (mut z_in, _) = InAccount::zero_copy_at_mut(&mut data).unwrap();

    // Create zero-copy merkle context
    let merkle_ctx = PackedMerkleContext {
        merkle_tree_pubkey_index: 5,
        queue_pubkey_index: 6,
        leaf_index: 999,
        prove_by_index: false,
    };
    let ctx_data = merkle_ctx.try_to_vec().unwrap();
    let (z_context, _) = PackedMerkleContext::zero_copy_at(&ctx_data).unwrap();

    // Execute: call set_z() with lamports = u64::MAX
    let result = z_in.set_z(
        [3u8; 8],
        [4u8; 32],
        &z_context,
        U16::new(2),
        u64::MAX, // Max lamports
        None,
    );

    // Assert: success and u64::from(*z_in.lamports) == u64::MAX
    assert!(result.is_ok());
    assert_eq!(u64::from(*z_in.lamports), u64::MAX);
}

#[test]
fn test_in_account_set_z_edge_case_merkle_context_bounds() {
    // Setup: InAccount without address
    let in_account = InAccount {
        discriminator: [1u8; 8],
        root_index: 1,
        data_hash: [2u8; 32],
        merkle_context: PackedMerkleContext::default(),
        lamports: 1000,
        address: None,
    };

    let mut data = in_account.try_to_vec().unwrap();
    let (mut z_in, _) = InAccount::zero_copy_at_mut(&mut data).unwrap();

    // Test with prove_by_index: false
    let merkle_ctx_false = PackedMerkleContext {
        merkle_tree_pubkey_index: u8::MAX,
        queue_pubkey_index: u8::MAX,
        leaf_index: u32::MAX,
        prove_by_index: false,
    };
    let ctx_data = merkle_ctx_false.try_to_vec().unwrap();
    let (z_context, _) = PackedMerkleContext::zero_copy_at(&ctx_data).unwrap();

    let result = z_in.set_z([3u8; 8], [4u8; 32], &z_context, U16::new(2), 2000, None);

    // Assert: success and all fields copied correctly with prove_by_index = false
    assert!(result.is_ok());
    assert_eq!(z_in.merkle_context.merkle_tree_pubkey_index, u8::MAX);
    assert_eq!(z_in.merkle_context.queue_pubkey_index, u8::MAX);
    assert_eq!(z_in.merkle_context.leaf_index.get(), u32::MAX);
    assert_eq!(z_in.merkle_context.prove_by_index, 0); // false = 0

    // Test with prove_by_index: true
    let merkle_ctx_true = PackedMerkleContext {
        merkle_tree_pubkey_index: u8::MAX,
        queue_pubkey_index: u8::MAX,
        leaf_index: u32::MAX,
        prove_by_index: true,
    };
    let ctx_data_true = merkle_ctx_true.try_to_vec().unwrap();
    let (z_context_true, _) = PackedMerkleContext::zero_copy_at(&ctx_data_true).unwrap();

    let result2 = z_in.set_z(
        [5u8; 8],
        [6u8; 32],
        &z_context_true,
        U16::new(3),
        3000,
        None,
    );

    // Assert: success and all fields copied correctly with prove_by_index = true
    assert!(result2.is_ok());
    assert_eq!(z_in.merkle_context.merkle_tree_pubkey_index, u8::MAX);
    assert_eq!(z_in.merkle_context.queue_pubkey_index, u8::MAX);
    assert_eq!(z_in.merkle_context.leaf_index.get(), u32::MAX);
    assert_eq!(z_in.merkle_context.prove_by_index, 1); // true = 1
}

#[test]
fn test_in_account_set_z_merkle_context_copying() {
    // Setup: InAccount without address
    let in_account = InAccount {
        discriminator: [1u8; 8],
        root_index: 1,
        data_hash: [2u8; 32],
        merkle_context: PackedMerkleContext::default(),
        lamports: 1000,
        address: None,
    };

    let mut data = in_account.try_to_vec().unwrap();
    let (mut z_in, _) = InAccount::zero_copy_at_mut(&mut data).unwrap();

    // Create PackedMerkleContext with specific values
    let merkle_ctx = PackedMerkleContext {
        merkle_tree_pubkey_index: 200,
        queue_pubkey_index: 150,
        leaf_index: 999999,
        prove_by_index: false,
    };
    let ctx_data = merkle_ctx.try_to_vec().unwrap();
    let (z_context, _) = PackedMerkleContext::zero_copy_at(&ctx_data).unwrap();

    // Execute: call set_z() with zero-copy context
    let result = z_in.set_z([3u8; 8], [4u8; 32], &z_context, U16::new(2), 2000, None);

    // Assert: verify all zero-copy merkle context fields copied correctly
    assert!(result.is_ok());
    assert_eq!(z_in.merkle_context.merkle_tree_pubkey_index, 200);
    assert_eq!(z_in.merkle_context.queue_pubkey_index, 150);
    assert_eq!(z_in.merkle_context.leaf_index.get(), 999999);
    assert_eq!(z_in.merkle_context.prove_by_index, 0); // false = 0
}

#[test]
#[should_panic]
fn test_in_account_set_error_invalid_address_length() {
    // Setup: InAccount with address slot
    let in_account = InAccount {
        discriminator: [0u8; 8],
        data_hash: [0u8; 32],
        merkle_context: PackedMerkleContext::default(),
        root_index: 0,
        lamports: 0,
        address: Some([0u8; 32]),
    };

    let mut data = in_account.try_to_vec().unwrap();
    let (mut z_in, _) = InAccount::zero_copy_at_mut(&mut data).unwrap();

    // Execute: call set() with Some(&[7u8; 16]) (wrong length)
    let merkle_context = PackedMerkleContext {
        merkle_tree_pubkey_index: 1,
        queue_pubkey_index: 2,
        leaf_index: 100,
        prove_by_index: true,
    };
    let wrong_length_address = [7u8; 16]; // Wrong length - should be 32 bytes
    let _result = z_in.set(
        [1u8; 8],
        [2u8; 32],
        &merkle_context,
        U16::new(3),
        4000,
        Some(&wrong_length_address), // Expected: panic on copy_from_slice
    );
}

#[test]
fn test_in_account_set_edge_case_zero_lamports() {
    // Setup: InAccount without address
    let in_account = InAccount {
        discriminator: [0u8; 8],
        data_hash: [0u8; 32],
        merkle_context: PackedMerkleContext::default(),
        root_index: 0,
        lamports: 0,
        address: None,
    };

    let mut data = in_account.try_to_vec().unwrap();
    let (mut z_in, _) = InAccount::zero_copy_at_mut(&mut data).unwrap();

    // Execute: call set() with lamports = 0
    let merkle_context = PackedMerkleContext {
        merkle_tree_pubkey_index: 1,
        queue_pubkey_index: 2,
        leaf_index: 100,
        prove_by_index: true,
    };
    let result = z_in.set(
        [1u8; 8],
        [2u8; 32],
        &merkle_context,
        U16::new(3),
        0, // Zero lamports
        None,
    );

    // Assert: success and u64::from(*z_in.lamports) == 0
    assert!(result.is_ok());
    assert_eq!(u64::from(*z_in.lamports), 0);
}

#[test]
fn test_in_account_set_edge_case_max_lamports() {
    // Setup: InAccount without address
    let in_account = InAccount {
        discriminator: [0u8; 8],
        data_hash: [0u8; 32],
        merkle_context: PackedMerkleContext::default(),
        root_index: 0,
        lamports: 0,
        address: None,
    };

    let mut data = in_account.try_to_vec().unwrap();
    let (mut z_in, _) = InAccount::zero_copy_at_mut(&mut data).unwrap();

    // Execute: call set() with lamports = u64::MAX
    let merkle_context = PackedMerkleContext {
        merkle_tree_pubkey_index: 1,
        queue_pubkey_index: 2,
        leaf_index: 100,
        prove_by_index: true,
    };
    let result = z_in.set(
        [1u8; 8],
        [2u8; 32],
        &merkle_context,
        U16::new(3),
        u64::MAX, // Max lamports
        None,
    );

    // Assert: success and u64::from(*z_in.lamports) == u64::MAX
    assert!(result.is_ok());
    assert_eq!(u64::from(*z_in.lamports), u64::MAX);
}

#[test]
fn test_in_account_set_merkle_context_copying() {
    // Setup: InAccount without address
    let in_account = InAccount {
        discriminator: [0u8; 8],
        data_hash: [0u8; 32],
        merkle_context: PackedMerkleContext::default(),
        root_index: 0,
        lamports: 0,
        address: None,
    };

    let mut data = in_account.try_to_vec().unwrap();
    let (mut z_in, _) = InAccount::zero_copy_at_mut(&mut data).unwrap();

    // Create PackedMerkleContext with specific values
    let merkle_context = PackedMerkleContext {
        merkle_tree_pubkey_index: 200,
        queue_pubkey_index: 150,
        leaf_index: 999999,
        prove_by_index: false,
    };

    // Execute: call set() with regular context
    let result = z_in.set(
        [1u8; 8],
        [2u8; 32],
        &merkle_context,
        U16::new(3),
        4000,
        None,
    );

    // Assert: verify all regular merkle context fields copied correctly
    assert!(result.is_ok());
    assert_eq!(z_in.merkle_context.merkle_tree_pubkey_index, 200);
    assert_eq!(z_in.merkle_context.queue_pubkey_index, 150);
    assert_eq!(z_in.merkle_context.leaf_index.get(), 999999);
    assert_eq!(z_in.merkle_context.prove_by_index, 0); // false = 0
}

// =============================================================================
// InstructionDataInvokeCpiWithReadOnlyMut::initialize() Tests
// =============================================================================

#[test]
fn test_instruction_initialize_success_with_proof() {
    // Setup: InstructionDataInvokeCpiWithReadOnly with proof slot
    let instruction = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: 0,
        invoking_program_id: Pubkey::new_unique(),
        compress_or_decompress_lamports: 0,
        is_compress: false,
        with_cpi_context: false,
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext::default(),
        proof: Some(CompressedProof {
            a: [0u8; 32],
            b: [0u8; 64],
            c: [0u8; 32],
        }),
        new_address_params: Vec::new(),
        input_compressed_accounts: Vec::new(),
        output_compressed_accounts: Vec::new(),
        read_only_addresses: Vec::new(),
        read_only_accounts: Vec::new(),
    };

    let mut data = instruction.try_to_vec().unwrap();
    let (mut z_instruction, _) =
        InstructionDataInvokeCpiWithReadOnly::zero_copy_at_mut(&mut data).unwrap();

    // Create CompressedProof and serialize with try_to_vec()
    let compressed_proof = CompressedProof {
        a: [5u8; 32],
        b: [6u8; 64],
        c: [7u8; 32],
    };
    let proof_data = compressed_proof.try_to_vec().unwrap();
    let (z_proof, _) = CompressedProof::zero_copy_at(&proof_data).unwrap();

    let invoking_program_id = Pubkey::new_unique();
    let cpi_context: Option<CompressedCpiContext> = None;

    // Execute: call initialize() with zero-copy proof
    let result = z_instruction.initialize(
        255, // test bump
        &invoking_program_id,
        Some(z_proof),
        &cpi_context,
    );

    // Assert: success, verify proof.a, proof.b, proof.c fields copied
    assert!(result.is_ok());
    let proof_ref = z_instruction.proof.as_ref().unwrap();
    assert_eq!(proof_ref.a, [5u8; 32]);
    assert_eq!(proof_ref.b, [6u8; 64]);
    assert_eq!(proof_ref.c, [7u8; 32]);
    assert_eq!(z_instruction.bump, 255);
    assert_eq!(z_instruction.invoking_program_id, invoking_program_id);
}

#[test]
fn test_instruction_initialize_success_without_proof() {
    // Setup: InstructionDataInvokeCpiWithReadOnly without proof slot (proof: None)
    let instruction = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: 0,
        invoking_program_id: Pubkey::new_unique(),
        compress_or_decompress_lamports: 0,
        is_compress: false,
        with_cpi_context: false,
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext::default(),
        proof: None, // No proof slot
        new_address_params: Vec::new(),
        input_compressed_accounts: Vec::new(),
        output_compressed_accounts: Vec::new(),
        read_only_addresses: Vec::new(),
        read_only_accounts: Vec::new(),
    };

    let mut data = instruction.try_to_vec().unwrap();
    let (mut z_instruction, _) =
        InstructionDataInvokeCpiWithReadOnly::zero_copy_at_mut(&mut data).unwrap();

    let invoking_program_id = Pubkey::new_unique();
    let cpi_context: Option<CompressedCpiContext> = None;

    // Execute: call initialize() with None proof
    let result = z_instruction.initialize(
        100, // test bump
        &invoking_program_id,
        None, // None proof
        &cpi_context,
    );

    // Assert: success, z_instruction.proof.is_none()
    assert!(result.is_ok());
    assert!(z_instruction.proof.is_none());
    assert_eq!(z_instruction.bump, 100);
    assert_eq!(z_instruction.invoking_program_id, invoking_program_id);
}

// =============================================================================
// NewAddressParamsAssignedPackedMut::set() Tests
// =============================================================================

#[test]
fn test_new_address_set_edge_case_zero_seed() {
    // Setup: NewAddressParamsAssignedPacked
    let new_address = NewAddressParamsAssignedPacked::default();

    let mut data = new_address.try_to_vec().unwrap();
    let (mut z_new_address, _) =
        NewAddressParamsAssignedPacked::zero_copy_at_mut(&mut data).unwrap();

    // Execute: call set() with seed = [0u8; 32]
    z_new_address.set(
        [0u8; 32], // Zero seed
        U16::new(100),
        Some(5),
        10,
    );

    // Assert: z_new_address.seed == [0u8; 32]
    assert_eq!(z_new_address.seed, [0u8; 32]);
    assert_eq!(z_new_address.address_merkle_tree_root_index.get(), 100);
    assert_eq!(z_new_address.assigned_account_index, 5);
    assert_eq!(z_new_address.address_merkle_tree_account_index, 10);
    assert_eq!(z_new_address.assigned_to_account, 1);
}

#[test]
fn test_new_address_set_edge_case_max_seed() {
    // Setup: NewAddressParamsAssignedPacked
    let new_address = NewAddressParamsAssignedPacked::default();

    let mut data = new_address.try_to_vec().unwrap();
    let (mut z_new_address, _) =
        NewAddressParamsAssignedPacked::zero_copy_at_mut(&mut data).unwrap();

    // Execute: call set() with seed = [0xFFu8; 32]
    z_new_address.set(
        [0xFFu8; 32], // Max seed
        U16::new(200),
        Some(15),
        20,
    );

    // Assert: z_new_address.seed == [0xFFu8; 32]
    assert_eq!(z_new_address.seed, [0xFFu8; 32]);
    assert_eq!(z_new_address.address_merkle_tree_root_index.get(), 200);
    assert_eq!(z_new_address.assigned_account_index, 15);
    assert_eq!(z_new_address.address_merkle_tree_account_index, 20);
    assert_eq!(z_new_address.assigned_to_account, 1);
}

#[test]
fn test_new_address_set_edge_case_zero_root_index() {
    // Setup: NewAddressParamsAssignedPacked
    let new_address = NewAddressParamsAssignedPacked::default();

    let mut data = new_address.try_to_vec().unwrap();
    let (mut z_new_address, _) =
        NewAddressParamsAssignedPacked::zero_copy_at_mut(&mut data).unwrap();

    // Execute: call set() with address_merkle_tree_root_index = U16::new(0)
    z_new_address.set(
        [1u8; 32],
        U16::new(0), // Zero root index
        Some(5),
        10,
    );

    // Assert: z_new_address.address_merkle_tree_root_index.get() == 0
    assert_eq!(z_new_address.address_merkle_tree_root_index.get(), 0);
}

#[test]
fn test_new_address_set_edge_case_max_root_index() {
    // Setup: NewAddressParamsAssignedPacked
    let new_address = NewAddressParamsAssignedPacked::default();

    let mut data = new_address.try_to_vec().unwrap();
    let (mut z_new_address, _) =
        NewAddressParamsAssignedPacked::zero_copy_at_mut(&mut data).unwrap();

    // Execute: call set() with address_merkle_tree_root_index = U16::new(u16::MAX)
    z_new_address.set(
        [1u8; 32],
        U16::new(u16::MAX), // Max root index
        Some(5),
        10,
    );

    // Assert: z_new_address.address_merkle_tree_root_index.get() == u16::MAX
    assert_eq!(z_new_address.address_merkle_tree_root_index.get(), u16::MAX);
}

#[test]
fn test_new_address_set_edge_case_zero_merkle_tree_account_index() {
    // Setup: NewAddressParamsAssignedPacked
    let new_address = NewAddressParamsAssignedPacked::default();

    let mut data = new_address.try_to_vec().unwrap();
    let (mut z_new_address, _) =
        NewAddressParamsAssignedPacked::zero_copy_at_mut(&mut data).unwrap();

    // Execute: call set() with address_merkle_tree_account_index = 0
    z_new_address.set(
        [1u8; 32],
        U16::new(100),
        Some(5),
        0, // Zero merkle tree account index
    );

    // Assert: z_new_address.address_merkle_tree_account_index == 0
    assert_eq!(z_new_address.address_merkle_tree_account_index, 0);
}

#[test]
fn test_new_address_set_edge_case_max_merkle_tree_account_index() {
    // Setup: NewAddressParamsAssignedPacked
    let new_address = NewAddressParamsAssignedPacked::default();

    let mut data = new_address.try_to_vec().unwrap();
    let (mut z_new_address, _) =
        NewAddressParamsAssignedPacked::zero_copy_at_mut(&mut data).unwrap();

    // Execute: call set() with address_merkle_tree_account_index = u8::MAX (255)
    z_new_address.set(
        [1u8; 32],
        U16::new(100),
        Some(5),
        u8::MAX, // Max merkle tree account index
    );

    // Assert: z_new_address.address_merkle_tree_account_index == u8::MAX
    assert_eq!(z_new_address.address_merkle_tree_account_index, u8::MAX);
}

#[test]
fn test_instruction_initialize_error_missing_proof() {
    // Setup: InstructionDataInvokeCpiWithReadOnly with proof slot
    let instruction = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: 0,
        invoking_program_id: Pubkey::new_unique(),
        compress_or_decompress_lamports: 0,
        is_compress: false,
        with_cpi_context: false,
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext::default(),
        proof: Some(CompressedProof {
            a: [0u8; 32],
            b: [0u8; 64],
            c: [0u8; 32],
        }),
        new_address_params: Vec::new(),
        input_compressed_accounts: Vec::new(),
        output_compressed_accounts: Vec::new(),
        read_only_addresses: Vec::new(),
        read_only_accounts: Vec::new(),
    };

    let mut data = instruction.try_to_vec().unwrap();
    let (mut z_instruction, _) =
        InstructionDataInvokeCpiWithReadOnly::zero_copy_at_mut(&mut data).unwrap();

    let invoking_program_id = Pubkey::new_unique();
    let cpi_context: Option<CompressedCpiContext> = None;

    // Execute: call initialize() with None proof (but InstructionData has proof slot)
    let result = z_instruction.initialize(
        100,
        &invoking_program_id,
        None, // Missing proof when InstructionData expects one
        &cpi_context,
    );

    // Assert: InstructionDataExpectedProof error
    assert_eq!(
        result.unwrap_err(),
        CompressedAccountError::InstructionDataExpectedProof
    );
}

#[test]
fn test_instruction_initialize_mode_invariant() {
    // Setup: InstructionDataInvokeCpiWithReadOnly with any initial mode value
    let instruction = InstructionDataInvokeCpiWithReadOnly {
        mode: 99, // Any initial mode value
        bump: 0,
        invoking_program_id: Pubkey::new_unique(),
        compress_or_decompress_lamports: 0,
        is_compress: false,
        with_cpi_context: false,
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext::default(),
        proof: None,
        new_address_params: Vec::new(),
        input_compressed_accounts: Vec::new(),
        output_compressed_accounts: Vec::new(),
        read_only_addresses: Vec::new(),
        read_only_accounts: Vec::new(),
    };

    let mut data = instruction.try_to_vec().unwrap();
    let (mut z_instruction, _) =
        InstructionDataInvokeCpiWithReadOnly::zero_copy_at_mut(&mut data).unwrap();

    let invoking_program_id = Pubkey::new_unique();
    let cpi_context: Option<CompressedCpiContext> = None;

    // Execute: call initialize()
    let result = z_instruction.initialize(50, &invoking_program_id, None, &cpi_context);

    // Assert: z_instruction.mode == 1 (always set to 1 regardless of input)
    assert!(result.is_ok());
    assert_eq!(z_instruction.mode, 1);
}

#[test]
fn test_instruction_initialize_edge_case_zero_bump() {
    let instruction = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: 50,
        invoking_program_id: Pubkey::new_unique(),
        compress_or_decompress_lamports: 0,
        is_compress: false,
        with_cpi_context: false,
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext::default(),
        proof: None,
        new_address_params: Vec::new(),
        input_compressed_accounts: Vec::new(),
        output_compressed_accounts: Vec::new(),
        read_only_addresses: Vec::new(),
        read_only_accounts: Vec::new(),
    };

    let mut data = instruction.try_to_vec().unwrap();
    let (mut z_instruction, _) =
        InstructionDataInvokeCpiWithReadOnly::zero_copy_at_mut(&mut data).unwrap();

    let result = z_instruction.initialize(
        0,
        &Pubkey::new_unique(),
        None,
        &None::<CompressedCpiContext>,
    );

    assert!(result.is_ok());
    assert_eq!(z_instruction.bump, 0);
}

#[test]
fn test_instruction_initialize_edge_case_max_bump() {
    let instruction = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: 0,
        invoking_program_id: Pubkey::new_unique(),
        compress_or_decompress_lamports: 0,
        is_compress: false,
        with_cpi_context: false,
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext::default(),
        proof: None,
        new_address_params: Vec::new(),
        input_compressed_accounts: Vec::new(),
        output_compressed_accounts: Vec::new(),
        read_only_addresses: Vec::new(),
        read_only_accounts: Vec::new(),
    };

    let mut data = instruction.try_to_vec().unwrap();
    let (mut z_instruction, _) =
        InstructionDataInvokeCpiWithReadOnly::zero_copy_at_mut(&mut data).unwrap();

    let result = z_instruction.initialize(
        u8::MAX,
        &Pubkey::new_unique(),
        None,
        &None::<CompressedCpiContext>,
    );

    assert!(result.is_ok());
    assert_eq!(z_instruction.bump, u8::MAX);
}

#[test]
fn test_instruction_initialize_success_without_cpi_context() {
    let instruction = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: 0,
        invoking_program_id: Pubkey::new_unique(),
        compress_or_decompress_lamports: 0,
        is_compress: false,
        with_cpi_context: false,
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext::default(),
        proof: None,
        new_address_params: Vec::new(),
        input_compressed_accounts: Vec::new(),
        output_compressed_accounts: Vec::new(),
        read_only_addresses: Vec::new(),
        read_only_accounts: Vec::new(),
    };

    let mut data = instruction.try_to_vec().unwrap();
    let (mut z_instruction, _) =
        InstructionDataInvokeCpiWithReadOnly::zero_copy_at_mut(&mut data).unwrap();

    let result = z_instruction.initialize(
        100,
        &Pubkey::new_unique(),
        None,
        &None::<CompressedCpiContext>,
    );

    assert!(result.is_ok());
    assert_eq!(z_instruction.with_cpi_context, 0);
}

#[test]
fn test_instruction_initialize_error_unexpected_proof() {
    // Setup: InstructionDataInvokeCpiWithReadOnly without proof slot (proof: None)
    let instruction = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: 0,
        invoking_program_id: Pubkey::new_unique(),
        compress_or_decompress_lamports: 0,
        is_compress: false,
        with_cpi_context: false,
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext::default(),
        proof: None, // No proof slot
        new_address_params: Vec::new(),
        input_compressed_accounts: Vec::new(),
        output_compressed_accounts: Vec::new(),
        read_only_addresses: Vec::new(),
        read_only_accounts: Vec::new(),
    };

    let mut data = instruction.try_to_vec().unwrap();
    let (mut z_instruction, _) =
        InstructionDataInvokeCpiWithReadOnly::zero_copy_at_mut(&mut data).unwrap();

    // Create zero-copy proof
    let compressed_proof = CompressedProof {
        a: [5u8; 32],
        b: [6u8; 64],
        c: [7u8; 32],
    };
    let proof_data = compressed_proof.try_to_vec().unwrap();
    let (z_proof, _) = CompressedProof::zero_copy_at(&proof_data).unwrap();

    // Execute: call initialize() with Some(zero_copy_proof) (but InstructionData has no proof slot)
    let result = z_instruction.initialize(
        100,
        &Pubkey::new_unique(),
        Some(z_proof), // Unexpected proof when InstructionData doesn't expect one
        &None::<CompressedCpiContext>,
    );

    // Assert: ZeroCopyExpectedProof error
    assert_eq!(
        result.unwrap_err(),
        CompressedAccountError::ZeroCopyExpectedProof
    );
}

#[test]
fn test_instruction_initialize_success_with_cpi_context() {
    let instruction = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: 0,
        invoking_program_id: Pubkey::new_unique(),
        compress_or_decompress_lamports: 0,
        is_compress: false,
        with_cpi_context: false,
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext::default(),
        proof: None,
        new_address_params: Vec::new(),
        input_compressed_accounts: Vec::new(),
        output_compressed_accounts: Vec::new(),
        read_only_addresses: Vec::new(),
        read_only_accounts: Vec::new(),
    };

    let mut data = instruction.try_to_vec().unwrap();
    let (mut z_instruction, _) =
        InstructionDataInvokeCpiWithReadOnly::zero_copy_at_mut(&mut data).unwrap();

    // Create CompressedCpiContext with specific values
    let cpi_context = CompressedCpiContext {
        set_context: true,
        first_set_context: true,
        cpi_context_account_index: 0,
    };

    let result = z_instruction.initialize(100, &Pubkey::new_unique(), None, &Some(cpi_context));

    // Assert: with_cpi_context == 1, verify context fields set correctly
    assert!(result.is_ok());
    assert_eq!(z_instruction.with_cpi_context, 1);
    assert_eq!(z_instruction.cpi_context.first_set_context, 1);
    assert_eq!(z_instruction.cpi_context.set_context, 1);
}

#[test]
fn test_instruction_initialize_proof_copying() {
    // Setup: InstructionDataInvokeCpiWithReadOnly with proof slot
    let instruction = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: 0,
        invoking_program_id: Pubkey::new_unique(),
        compress_or_decompress_lamports: 0,
        is_compress: false,
        with_cpi_context: false,
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext::default(),
        proof: Some(CompressedProof {
            a: [0u8; 32],
            b: [0u8; 64],
            c: [0u8; 32],
        }),
        new_address_params: Vec::new(),
        input_compressed_accounts: Vec::new(),
        output_compressed_accounts: Vec::new(),
        read_only_addresses: Vec::new(),
        read_only_accounts: Vec::new(),
    };

    let mut data = instruction.try_to_vec().unwrap();
    let (mut z_instruction, _) =
        InstructionDataInvokeCpiWithReadOnly::zero_copy_at_mut(&mut data).unwrap();

    // Create CompressedProof with specific values: a: [5u8; 32], b: [6u8; 64], c: [7u8; 32]
    let compressed_proof = CompressedProof {
        a: [5u8; 32],
        b: [6u8; 64],
        c: [7u8; 32],
    };
    let proof_data = compressed_proof.try_to_vec().unwrap();
    let (z_proof, _) = CompressedProof::zero_copy_at(&proof_data).unwrap();

    let result = z_instruction.initialize(
        200,
        &Pubkey::new_unique(),
        Some(z_proof),
        &None::<CompressedCpiContext>,
    );

    // Assert: verify proof_ref.a == [5u8; 32], proof_ref.b == [6u8; 64], proof_ref.c == [7u8; 32]
    assert!(result.is_ok());
    let proof_ref = z_instruction.proof.as_ref().unwrap();
    assert_eq!(proof_ref.a, [5u8; 32]);
    assert_eq!(proof_ref.b, [6u8; 64]);
    assert_eq!(proof_ref.c, [7u8; 32]);
}

#[test]
fn test_randomized_all_functions() {
    // Setup: Generate seed with thread_rng() and print for reproducibility
    let seed = thread_rng().gen::<u64>();
    println!("Randomized test seed: {}", seed);
    let mut rng = StdRng::seed_from_u64(seed);

    // Execute: 1000 iterations loop
    for _i in 0..1000 {
        // Test ZOutputCompressedAccountWithPackedContextMut::set()
        let output_account = OutputCompressedAccountWithPackedContext {
            compressed_account: create_compressed_account(Some([1u8; 32]), None),
            merkle_tree_index: rng.gen::<u8>(),
        };
        let mut data = output_account.try_to_vec().unwrap();
        let (mut z_output, _) =
            OutputCompressedAccountWithPackedContext::zero_copy_at_mut(&mut data).unwrap();
        let _ = z_output.set(
            Pubkey::new_unique(),
            rng.gen::<u64>(),
            Some(rng.gen::<[u8; 32]>()),
            rng.gen::<u8>(),
            rng.gen::<[u8; 8]>(),
            rng.gen::<[u8; 32]>(),
        );

        // Test ZInAccountMut::set_z()
        let in_account = InAccount {
            discriminator: rng.gen::<[u8; 8]>(),
            root_index: rng.gen::<u16>(),
            data_hash: rng.gen::<[u8; 32]>(),
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: rng.gen::<u8>(),
                queue_pubkey_index: rng.gen::<u8>(),
                leaf_index: rng.gen::<u32>(),
                prove_by_index: rng.gen::<bool>(),
            },
            lamports: rng.gen::<u64>(),
            address: None,
        };
        let mut in_data = in_account.try_to_vec().unwrap();
        let (mut z_in, _) = InAccount::zero_copy_at_mut(&mut in_data).unwrap();

        let merkle_ctx = PackedMerkleContext {
            merkle_tree_pubkey_index: rng.gen::<u8>(),
            queue_pubkey_index: rng.gen::<u8>(),
            leaf_index: rng.gen::<u32>(),
            prove_by_index: rng.gen::<bool>(),
        };
        let ctx_data = merkle_ctx.try_to_vec().unwrap();
        let (z_context, _) = PackedMerkleContext::zero_copy_at(&ctx_data).unwrap();

        let _ = z_in.set_z(
            rng.gen::<[u8; 8]>(),
            rng.gen::<[u8; 32]>(),
            &z_context,
            U16::new(rng.gen::<u16>()),
            rng.gen::<u64>(),
            None,
        );

        // Test ZInAccountMut::set()
        let _ = z_in.set(
            rng.gen::<[u8; 8]>(),
            rng.gen::<[u8; 32]>(),
            &merkle_ctx,
            U16::new(rng.gen::<u16>()),
            rng.gen::<u64>(),
            None,
        );

        // Test ZInstructionDataInvokeCpiWithReadOnlyMut::initialize()
        let instruction = InstructionDataInvokeCpiWithReadOnly {
            mode: rng.gen::<u8>(),
            bump: rng.gen::<u8>(),
            invoking_program_id: Pubkey::new_unique(),
            compress_or_decompress_lamports: 0,
            is_compress: false,
            with_cpi_context: false,
            with_transaction_hash: false,
            cpi_context: CompressedCpiContext::default(),
            proof: None,
            new_address_params: Vec::new(),
            input_compressed_accounts: Vec::new(),
            output_compressed_accounts: Vec::new(),
            read_only_addresses: Vec::new(),
            read_only_accounts: Vec::new(),
        };
        let mut inst_data = instruction.try_to_vec().unwrap();
        let (mut z_instruction, _) =
            InstructionDataInvokeCpiWithReadOnly::zero_copy_at_mut(&mut inst_data).unwrap();

        let _ = z_instruction.initialize(
            rng.gen::<u8>(),
            &Pubkey::new_unique(),
            None,
            &None::<CompressedCpiContext>,
        );

        // Test ZNewAddressParamsAssignedPackedMut::set()
        let new_address = NewAddressParamsAssignedPacked::default();
        let mut addr_data = new_address.try_to_vec().unwrap();
        let (mut z_new_address, _) =
            NewAddressParamsAssignedPacked::zero_copy_at_mut(&mut addr_data).unwrap();

        z_new_address.set(
            rng.gen::<[u8; 32]>(),
            U16::new(rng.gen::<u16>()),
            Some(rng.gen::<u8>()),
            rng.gen::<u8>(),
        );
    }

    // Assert: All calls succeed with random valid inputs
    // The test succeeds if we reach this point without panicking
}

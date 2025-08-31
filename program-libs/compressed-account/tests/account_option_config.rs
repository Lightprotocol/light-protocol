#![cfg(all(test, feature = "new-unique"))]

use borsh::BorshSerialize;
use light_compressed_account::{
    instruction_data::{
        cpi_context::CompressedCpiContext,
        invoke_cpi::InstructionDataInvokeCpi,
        traits::{AccountOptions, InstructionData},
        with_account_info::InstructionDataInvokeCpiWithAccountInfo,
        with_readonly::InstructionDataInvokeCpiWithReadOnly,
        zero_copy::ZInstructionDataInvokeCpi,
    },
    pubkey::Pubkey,
};
use light_zero_copy::traits::ZeroCopyAt;
use rand::{rngs::StdRng, Rng, SeedableRng};

/// Tests for account_option_config() implementations
/// Structs tested:
/// 1. ZInstructionDataInvokeCpi - 4 tests for lamports/compress/context combinations
/// 2. ZInstructionDataInvokeCpiWithReadOnly - 4 tests for write_to_cpi_context logic
/// 3. ZInstructionDataInvokeCpiWithAccountInfo - 2 tests for full config scenarios
/// 4. AccountOptions::get_num_expected_accounts - 6 tests for account counting
/// 5. Randomized property tests - 3 tests with 1k iterations each
/// 6. Cross-implementation consistency - 1 test

// =============================================================================
// ZInstructionDataInvokeCpi Tests
// =============================================================================

#[test]
fn test_invoke_cpi_no_lamports_no_context() {
    // Setup: No lamports, no CPI context
    let instruction = InstructionDataInvokeCpi {
        proof: None,
        new_address_params: vec![],
        input_compressed_accounts_with_merkle_context: vec![],
        output_compressed_accounts: vec![],
        relay_fee: None,
        is_compress: false,
        compress_or_decompress_lamports: None,
        cpi_context: None,
    };

    // Execute - serialize with borsh then deserialize as zero-copy
    let mut z_instruction_bytes = Vec::new();
    instruction.serialize(&mut z_instruction_bytes).unwrap();
    let (z_instruction, _) = ZInstructionDataInvokeCpi::zero_copy_at(&z_instruction_bytes).unwrap();
    let options = z_instruction.account_option_config().unwrap();

    // Assert: All flags should be false
    let expected = AccountOptions {
        sol_pool_pda: false,
        decompression_recipient: false,
        cpi_context_account: false,
        write_to_cpi_context: false,
    };
    assert_eq!(options, expected);
}

#[test]
fn test_invoke_cpi_compress_with_lamports() {
    // Setup: Compress mode with lamports
    let instruction = InstructionDataInvokeCpi {
        proof: None,
        new_address_params: vec![],
        input_compressed_accounts_with_merkle_context: vec![],
        output_compressed_accounts: vec![],
        relay_fee: None,
        is_compress: true,
        compress_or_decompress_lamports: Some(100),
        cpi_context: None,
    };

    // Execute - serialize with borsh then deserialize as zero-copy
    let mut z_instruction_bytes = Vec::new();
    instruction.serialize(&mut z_instruction_bytes).unwrap();
    let (z_instruction, _) = ZInstructionDataInvokeCpi::zero_copy_at(&z_instruction_bytes).unwrap();
    let options = z_instruction.account_option_config().unwrap();

    // Assert: sol_pool_pda=true, decompression_recipient=false (because is_compress=true)
    let expected = AccountOptions {
        sol_pool_pda: true,
        decompression_recipient: false,
        cpi_context_account: false,
        write_to_cpi_context: false,
    };
    assert_eq!(options, expected);
}

#[test]
fn test_invoke_cpi_decompress_with_lamports() {
    // Setup: Decompress mode with lamports
    let instruction = InstructionDataInvokeCpi {
        proof: None,
        new_address_params: vec![],
        input_compressed_accounts_with_merkle_context: vec![],
        output_compressed_accounts: vec![],
        relay_fee: None,
        is_compress: false,
        compress_or_decompress_lamports: Some(100),
        cpi_context: None,
    };

    // Execute - serialize with borsh then deserialize as zero-copy
    let mut z_instruction_bytes = Vec::new();
    instruction.serialize(&mut z_instruction_bytes).unwrap();
    let (z_instruction, _) = ZInstructionDataInvokeCpi::zero_copy_at(&z_instruction_bytes).unwrap();
    let options = z_instruction.account_option_config().unwrap();

    // Assert: Both sol_pool_pda and decompression_recipient should be true
    let expected = AccountOptions {
        sol_pool_pda: true,
        decompression_recipient: true,
        cpi_context_account: false,
        write_to_cpi_context: false,
    };
    assert_eq!(options, expected);
}

#[test]
fn test_invoke_cpi_with_context() {
    // Setup: With CPI context
    let instruction = InstructionDataInvokeCpi {
        proof: None,
        new_address_params: vec![],
        input_compressed_accounts_with_merkle_context: vec![],
        output_compressed_accounts: vec![],
        relay_fee: None,
        is_compress: false,
        compress_or_decompress_lamports: None,
        cpi_context: Some(CompressedCpiContext {
            set_context: true,
            first_set_context: true,
            cpi_context_account_index: 0,
        }),
    };

    // Execute - serialize with borsh then deserialize as zero-copy
    let mut z_instruction_bytes = Vec::new();
    instruction.serialize(&mut z_instruction_bytes).unwrap();
    let (z_instruction, _) = ZInstructionDataInvokeCpi::zero_copy_at(&z_instruction_bytes).unwrap();
    let options = z_instruction.account_option_config().unwrap();

    // Assert: cpi_context_account=true, write_to_cpi_context=false (always for InvokeCpi)
    let expected = AccountOptions {
        sol_pool_pda: false,
        decompression_recipient: false,
        cpi_context_account: true,
        write_to_cpi_context: false, // Always false for ZInstructionDataInvokeCpi
    };
    assert_eq!(options, expected);
}

// =============================================================================
// ZInstructionDataInvokeCpiWithReadOnly Tests
// =============================================================================

#[test]
fn test_readonly_write_context_first_set() {
    // Setup: first_set_context=true, set_context=false
    let instruction = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: 0,
        invoking_program_id: Pubkey::default(),
        compress_or_decompress_lamports: 0,
        is_compress: false,
        with_cpi_context: true,
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext {
            set_context: false,
            first_set_context: true,
            cpi_context_account_index: 0,
        },
        proof: None,
        new_address_params: vec![],
        input_compressed_accounts: vec![],
        output_compressed_accounts: vec![],
        read_only_addresses: vec![],
        read_only_accounts: vec![],
    };

    // Execute - serialize with borsh then deserialize as zero-copy
    let mut z_instruction_bytes = Vec::new();
    instruction.serialize(&mut z_instruction_bytes).unwrap();
    let (z_instruction, _) =
        InstructionDataInvokeCpiWithReadOnly::zero_copy_at(&z_instruction_bytes).unwrap();
    let options = z_instruction.account_option_config().unwrap();

    // Assert: write_to_cpi_context should be true
    assert!(options.write_to_cpi_context);
}

#[test]
fn test_readonly_write_context_set() {
    // Setup: first_set_context=false, set_context=true
    let instruction = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: 0,
        invoking_program_id: Pubkey::default(),
        compress_or_decompress_lamports: 0,
        is_compress: false,
        with_cpi_context: true,
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext {
            set_context: true,
            first_set_context: false,
            cpi_context_account_index: 0,
        },
        proof: None,
        new_address_params: vec![],
        input_compressed_accounts: vec![],
        output_compressed_accounts: vec![],
        read_only_addresses: vec![],
        read_only_accounts: vec![],
    };

    // Execute - serialize with borsh then deserialize as zero-copy
    let mut z_instruction_bytes = Vec::new();
    instruction.serialize(&mut z_instruction_bytes).unwrap();
    let (z_instruction, _) =
        InstructionDataInvokeCpiWithReadOnly::zero_copy_at(&z_instruction_bytes).unwrap();
    let options = z_instruction.account_option_config().unwrap();

    // Assert: write_to_cpi_context should be true
    assert!(options.write_to_cpi_context);
}

#[test]
fn test_readonly_write_context_both() {
    // Setup: first_set_context=true, set_context=true
    let instruction = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: 0,
        invoking_program_id: Pubkey::default(),
        compress_or_decompress_lamports: 0,
        is_compress: false,
        with_cpi_context: true,
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext {
            set_context: true,
            first_set_context: true,
            cpi_context_account_index: 0,
        },
        proof: None,
        new_address_params: vec![],
        input_compressed_accounts: vec![],
        output_compressed_accounts: vec![],
        read_only_addresses: vec![],
        read_only_accounts: vec![],
    };

    // Execute - serialize with borsh then deserialize as zero-copy
    let mut z_instruction_bytes = Vec::new();
    instruction.serialize(&mut z_instruction_bytes).unwrap();
    let (z_instruction, _) =
        InstructionDataInvokeCpiWithReadOnly::zero_copy_at(&z_instruction_bytes).unwrap();
    let options = z_instruction.account_option_config().unwrap();

    // Assert: write_to_cpi_context should be true (OR logic)
    assert!(options.write_to_cpi_context);
}

#[test]
fn test_readonly_write_context_neither() {
    // Setup: first_set_context=false, set_context=false
    let instruction = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: 0,
        invoking_program_id: Pubkey::default(),
        compress_or_decompress_lamports: 0,
        is_compress: false,
        with_cpi_context: true,
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext {
            set_context: false,
            first_set_context: false,
            cpi_context_account_index: 0,
        },
        proof: None,
        new_address_params: vec![],
        input_compressed_accounts: vec![],
        output_compressed_accounts: vec![],
        read_only_addresses: vec![],
        read_only_accounts: vec![],
    };

    // Execute - serialize with borsh then deserialize as zero-copy
    let mut z_instruction_bytes = Vec::new();
    instruction.serialize(&mut z_instruction_bytes).unwrap();
    let (z_instruction, _) =
        InstructionDataInvokeCpiWithReadOnly::zero_copy_at(&z_instruction_bytes).unwrap();
    let options = z_instruction.account_option_config().unwrap();

    // Assert: write_to_cpi_context should be false
    assert!(!options.write_to_cpi_context);
}

// =============================================================================
// ZInstructionDataInvokeCpiWithAccountInfo Tests
// =============================================================================

#[test]
fn test_account_info_full_config() {
    // Setup: Decompress with lamports and CPI context with both flags
    let instruction = InstructionDataInvokeCpiWithAccountInfo {
        mode: 0,
        bump: 0,
        invoking_program_id: Pubkey::default(),
        compress_or_decompress_lamports: 100,
        is_compress: false,
        with_cpi_context: true,
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext {
            set_context: true,
            first_set_context: true,
            cpi_context_account_index: 0,
        },
        proof: None,
        new_address_params: vec![],
        account_infos: vec![],
        read_only_addresses: vec![],
        read_only_accounts: vec![],
    };

    // Execute - serialize with borsh then deserialize as zero-copy
    let mut z_instruction_bytes = Vec::new();
    instruction.serialize(&mut z_instruction_bytes).unwrap();
    let (z_instruction, _) =
        InstructionDataInvokeCpiWithAccountInfo::zero_copy_at(&z_instruction_bytes).unwrap();
    let options = z_instruction.account_option_config().unwrap();

    // Assert: All flags should be true
    let expected = AccountOptions {
        sol_pool_pda: true,            // has lamports
        decompression_recipient: true, // has lamports && !is_compress
        cpi_context_account: true,     // has cpi_context
        write_to_cpi_context: true,    // first_set_context || set_context
    };
    assert_eq!(options, expected);
}

#[test]
fn test_account_info_compress_mode() {
    // Setup: Compress mode with lamports
    let instruction = InstructionDataInvokeCpiWithAccountInfo {
        mode: 0,
        bump: 0,
        invoking_program_id: Pubkey::default(),
        compress_or_decompress_lamports: 100,
        is_compress: true,
        with_cpi_context: true,
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext {
            set_context: false,
            first_set_context: false,
            cpi_context_account_index: 0,
        },
        proof: None,
        new_address_params: vec![],
        account_infos: vec![],
        read_only_addresses: vec![],
        read_only_accounts: vec![],
    };

    // Execute - serialize with borsh then deserialize as zero-copy
    let mut z_instruction_bytes = Vec::new();
    instruction.serialize(&mut z_instruction_bytes).unwrap();
    let (z_instruction, _) =
        InstructionDataInvokeCpiWithAccountInfo::zero_copy_at(&z_instruction_bytes).unwrap();
    let options = z_instruction.account_option_config().unwrap();

    // Assert: decompression_recipient should be false when is_compress=true
    let expected = AccountOptions {
        sol_pool_pda: true,
        decompression_recipient: false, // is_compress=true
        cpi_context_account: true,
        write_to_cpi_context: false,
    };
    assert_eq!(options, expected);
}

// =============================================================================
// Randomized Property Tests
// =============================================================================

#[test]
fn test_randomized_invoke_cpi_config() {
    let mut rng = StdRng::seed_from_u64(42);

    for iteration in 0..1000 {
        // Generate random parameters
        let has_lamports = rng.gen_bool(0.5);
        let lamports = if has_lamports {
            Some(rng.gen_range(1..=1000000))
        } else {
            None
        };
        let is_compress = rng.gen_bool(0.5);
        let has_context = rng.gen_bool(0.5);
        let cpi_context = if has_context {
            Some(CompressedCpiContext {
                set_context: rng.gen_bool(0.5),
                first_set_context: rng.gen_bool(0.5),
                cpi_context_account_index: rng.gen_range(0..=10),
            })
        } else {
            None
        };

        // Create instruction
        let instruction = InstructionDataInvokeCpi {
            proof: None,
            new_address_params: vec![],
            input_compressed_accounts_with_merkle_context: vec![],
            output_compressed_accounts: vec![],
            relay_fee: None,
            is_compress,
            compress_or_decompress_lamports: lamports,
            cpi_context,
        };

        // Convert to zero-copy and get options
        let mut z_instruction_bytes = Vec::new();
        instruction.serialize(&mut z_instruction_bytes).unwrap();
        let (z_instruction, _) =
            ZInstructionDataInvokeCpi::zero_copy_at(&z_instruction_bytes).unwrap();
        let options = z_instruction.account_option_config().unwrap();

        // Verify invariants
        // sol_pool_pda must be true iff lamports.is_some()
        assert_eq!(
            options.sol_pool_pda,
            lamports.is_some(),
            "Iteration {}: sol_pool_pda mismatch",
            iteration
        );

        // decompression_recipient must be true iff lamports.is_some() && !is_compress
        assert_eq!(
            options.decompression_recipient,
            lamports.is_some() && !is_compress,
            "Iteration {}: decompression_recipient mismatch",
            iteration
        );

        // cpi_context_account must be true iff cpi_context.is_some()
        assert_eq!(
            options.cpi_context_account,
            cpi_context.is_some(),
            "Iteration {}: cpi_context_account mismatch",
            iteration
        );

        // write_to_cpi_context must always be false for InvokeCpi
        assert!(
            !options.write_to_cpi_context,
            "Iteration {}: write_to_cpi_context should always be false",
            iteration
        );
    }
}

#[test]
fn test_randomized_readonly_config() {
    let mut rng = StdRng::seed_from_u64(43);

    for iteration in 0..1000 {
        // Generate random parameters
        let has_lamports = rng.gen_bool(0.5);
        let lamports = if has_lamports {
            rng.gen_range(1..=1000000)
        } else {
            0
        };
        let is_compress = rng.gen_bool(0.5);
        let set_context = rng.gen_bool(0.5);
        let first_set_context = rng.gen_bool(0.5);

        // Create instruction
        let instruction = InstructionDataInvokeCpiWithReadOnly {
            mode: 0,
            bump: 0,
            invoking_program_id: Pubkey::default(),
            compress_or_decompress_lamports: lamports,
            is_compress,
            with_cpi_context: true,
            with_transaction_hash: false,
            cpi_context: CompressedCpiContext {
                set_context,
                first_set_context,
                cpi_context_account_index: 0,
            },
            proof: None,
            new_address_params: vec![],
            input_compressed_accounts: vec![],
            output_compressed_accounts: vec![],
            read_only_addresses: vec![],
            read_only_accounts: vec![],
        };

        // Convert to zero-copy and get options
        let mut z_instruction_bytes = Vec::new();
        instruction.serialize(&mut z_instruction_bytes).unwrap();
        let (z_instruction, _) =
            InstructionDataInvokeCpiWithReadOnly::zero_copy_at(&z_instruction_bytes).unwrap();
        let options = z_instruction.account_option_config().unwrap();

        // Verify invariants
        assert_eq!(
            options.sol_pool_pda, has_lamports,
            "Iteration {}: sol_pool_pda mismatch",
            iteration
        );

        assert_eq!(
            options.decompression_recipient,
            has_lamports && !is_compress,
            "Iteration {}: decompression_recipient mismatch",
            iteration
        );

        // CPI context always exists for WithReadOnly
        assert!(
            options.cpi_context_account,
            "Iteration {}: cpi_context_account should always be true",
            iteration
        );

        // write_to_cpi_context = first_set_context || set_context
        assert_eq!(
            options.write_to_cpi_context,
            first_set_context || set_context,
            "Iteration {}: write_to_cpi_context mismatch",
            iteration
        );
    }
}

#[test]
fn test_randomized_account_info_config() {
    let mut rng = StdRng::seed_from_u64(44);

    for iteration in 0..1000 {
        // Generate random parameters
        let has_lamports = rng.gen_bool(0.5);
        let lamports = if has_lamports {
            rng.gen_range(1..=1000000)
        } else {
            0
        };
        let is_compress = rng.gen_bool(0.5);
        let set_context = rng.gen_bool(0.5);
        let first_set_context = rng.gen_bool(0.5);
        let with_cpi_context = rng.gen_bool(0.5);

        // Create instruction
        let instruction = InstructionDataInvokeCpiWithAccountInfo {
            mode: 0,
            bump: 0,
            invoking_program_id: Pubkey::default(),
            compress_or_decompress_lamports: lamports,
            is_compress,
            with_cpi_context,
            with_transaction_hash: false,
            cpi_context: CompressedCpiContext {
                set_context,
                first_set_context,
                cpi_context_account_index: 0,
            },
            proof: None,
            new_address_params: vec![],
            account_infos: vec![],
            read_only_addresses: vec![],
            read_only_accounts: vec![],
        };

        // Convert to zero-copy and get options
        let mut z_instruction_bytes = Vec::new();
        instruction.serialize(&mut z_instruction_bytes).unwrap();
        let (z_instruction, _) =
            InstructionDataInvokeCpiWithAccountInfo::zero_copy_at(&z_instruction_bytes).unwrap();

        // Check if this combination should produce an error
        let wants_to_write = first_set_context || set_context;
        let should_error = wants_to_write && !with_cpi_context;

        if should_error {
            // Verify we get the expected error
            let result = z_instruction.account_option_config();
            assert!(
                result.is_err(),
                "Iteration {}: Expected error for invalid state",
                iteration
            );
            assert_eq!(
                result.unwrap_err(),
                light_compressed_account::CompressedAccountError::InvalidCpiContext,
                "Iteration {}: Wrong error type",
                iteration
            );
        } else {
            // Valid configuration, verify the options
            let options = z_instruction.account_option_config().unwrap();

            assert_eq!(
                options.sol_pool_pda, has_lamports,
                "Iteration {}: sol_pool_pda mismatch",
                iteration
            );

            assert_eq!(
                options.decompression_recipient,
                has_lamports && !is_compress,
                "Iteration {}: decompression_recipient mismatch",
                iteration
            );

            assert_eq!(
                options.cpi_context_account, with_cpi_context,
                "Iteration {}: cpi_context_account mismatch",
                iteration
            );

            // write_to_cpi_context only true if both conditions met
            assert_eq!(
                options.write_to_cpi_context,
                wants_to_write && with_cpi_context,
                "Iteration {}: write_to_cpi_context mismatch",
                iteration
            );
        }
    }
}

// =============================================================================
// Error Case Tests
// =============================================================================

#[test]
fn test_cpi_context_present_but_no_write() {
    // Test valid state: Has CPI context but doesn't want to write (both flags false)
    // This should succeed with write_to_cpi_context = false

    let instruction = InstructionDataInvokeCpiWithAccountInfo {
        mode: 0,
        bump: 0,
        invoking_program_id: Pubkey::default(),
        compress_or_decompress_lamports: 100, // Has lamports for other flags
        is_compress: false,
        with_cpi_context: true, // Has CPI context
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext {
            set_context: false,       // But doesn't want to write
            first_set_context: false, // Both write flags false
            cpi_context_account_index: 0,
        },
        proof: None,
        new_address_params: vec![],
        account_infos: vec![],
        read_only_addresses: vec![],
        read_only_accounts: vec![],
    };

    // Convert to zero-copy
    let mut z_instruction_bytes = Vec::new();
    instruction.serialize(&mut z_instruction_bytes).unwrap();
    let (z_instruction, _) =
        InstructionDataInvokeCpiWithAccountInfo::zero_copy_at(&z_instruction_bytes).unwrap();

    // Should succeed - valid configuration
    let options = z_instruction.account_option_config().unwrap();

    // Verify the expected state
    assert!(
        options.sol_pool_pda,
        "Should have sol_pool_pda due to lamports"
    );
    assert!(
        options.decompression_recipient,
        "Should have decompression_recipient (lamports && !compress)"
    );
    assert!(
        options.cpi_context_account,
        "Should have cpi_context_account"
    );
    assert!(
        !options.write_to_cpi_context,
        "Should NOT write to context (both flags false)"
    );
}

#[test]
fn test_readonly_cpi_context_present_but_no_write() {
    // Test valid state for ReadOnly: Has CPI context but doesn't want to write
    // This should succeed with write_to_cpi_context = false

    let instruction = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: 0,
        invoking_program_id: Pubkey::default(),
        compress_or_decompress_lamports: 50,
        is_compress: true,      // Compress mode for variety
        with_cpi_context: true, // Has CPI context
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext {
            set_context: false,       // But doesn't want to write
            first_set_context: false, // Both write flags false
            cpi_context_account_index: 0,
        },
        proof: None,
        new_address_params: vec![],
        input_compressed_accounts: vec![],
        output_compressed_accounts: vec![],
        read_only_addresses: vec![],
        read_only_accounts: vec![],
    };

    // Convert to zero-copy
    let mut z_instruction_bytes = Vec::new();
    instruction.serialize(&mut z_instruction_bytes).unwrap();
    let (z_instruction, _) =
        InstructionDataInvokeCpiWithReadOnly::zero_copy_at(&z_instruction_bytes).unwrap();

    // Should succeed - valid configuration
    let options = z_instruction.account_option_config().unwrap();

    // Verify the expected state
    assert!(
        options.sol_pool_pda,
        "Should have sol_pool_pda due to lamports"
    );
    assert!(
        !options.decompression_recipient,
        "Should NOT have decompression_recipient (is_compress=true)"
    );
    assert!(
        options.cpi_context_account,
        "Should have cpi_context_account"
    );
    assert!(
        !options.write_to_cpi_context,
        "Should NOT write to context (both flags false)"
    );
}

#[test]
fn test_invalid_cpi_context_error() {
    // Test that we get an error when write_to_cpi_context is true but cpi_context is None

    // Create instruction with mismatched flags for WithAccountInfo
    let instruction = InstructionDataInvokeCpiWithAccountInfo {
        mode: 0,
        bump: 0,
        invoking_program_id: Pubkey::default(),
        compress_or_decompress_lamports: 0,
        is_compress: false,
        with_cpi_context: false, // No CPI context
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext {
            set_context: true, // But we want to write
            first_set_context: false,
            cpi_context_account_index: 0,
        },
        proof: None,
        new_address_params: vec![],
        account_infos: vec![],
        read_only_addresses: vec![],
        read_only_accounts: vec![],
    };

    // Convert to zero-copy
    let mut z_instruction_bytes = Vec::new();
    instruction.serialize(&mut z_instruction_bytes).unwrap();
    let (z_instruction, _) =
        InstructionDataInvokeCpiWithAccountInfo::zero_copy_at(&z_instruction_bytes).unwrap();

    // Should get InvalidCpiContext error
    let result = z_instruction.account_option_config();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        light_compressed_account::CompressedAccountError::InvalidCpiContext
    );
}

// =============================================================================
// Cross-Implementation Consistency Tests
// =============================================================================

#[test]
fn test_consistency_readonly_vs_account_info() {
    // Test that ReadOnly and AccountInfo produce same results for equivalent inputs
    let test_cases = vec![
        (100, false, true, false), // Decompress with first_set
        (100, true, false, true),  // Compress with set_context
        (0, false, false, false),  // No lamports, no context
        (500, false, true, true),  // Decompress with both context flags
    ];

    for (lamports, is_compress, first_set, set_ctx) in test_cases {
        // Create ReadOnly instruction
        let readonly_instruction = InstructionDataInvokeCpiWithReadOnly {
            mode: 0,
            bump: 0,
            invoking_program_id: Pubkey::default(),
            compress_or_decompress_lamports: lamports,
            is_compress,
            with_cpi_context: true,
            with_transaction_hash: false,
            cpi_context: CompressedCpiContext {
                set_context: set_ctx,
                first_set_context: first_set,
                cpi_context_account_index: 0,
            },
            proof: None,
            new_address_params: vec![],
            input_compressed_accounts: vec![],
            output_compressed_accounts: vec![],
            read_only_addresses: vec![],
            read_only_accounts: vec![],
        };

        // Create AccountInfo instruction with equivalent config
        let account_info_instruction = InstructionDataInvokeCpiWithAccountInfo {
            mode: 0,
            bump: 0,
            invoking_program_id: Pubkey::default(),
            compress_or_decompress_lamports: lamports,
            is_compress,
            with_cpi_context: true,
            with_transaction_hash: false,
            cpi_context: CompressedCpiContext {
                set_context: set_ctx,
                first_set_context: first_set,
                cpi_context_account_index: 0,
            },
            proof: None,
            new_address_params: vec![],
            account_infos: vec![],
            read_only_addresses: vec![],
            read_only_accounts: vec![],
        };

        // Convert both to zero-copy
        let mut z_readonly_bytes = Vec::new();
        readonly_instruction
            .serialize(&mut z_readonly_bytes)
            .unwrap();
        let (z_readonly, _) =
            InstructionDataInvokeCpiWithReadOnly::zero_copy_at(&z_readonly_bytes).unwrap();
        let readonly_options = z_readonly.account_option_config().unwrap();

        let mut z_account_info_bytes = Vec::new();
        account_info_instruction
            .serialize(&mut z_account_info_bytes)
            .unwrap();
        let (z_account_info, _) =
            InstructionDataInvokeCpiWithAccountInfo::zero_copy_at(&z_account_info_bytes).unwrap();
        let account_info_options = z_account_info.account_option_config().unwrap();

        // Assert that the core flags match
        assert_eq!(
            readonly_options.sol_pool_pda, account_info_options.sol_pool_pda,
            "sol_pool_pda mismatch for lamports={}, compress={}",
            lamports, is_compress
        );

        assert_eq!(
            readonly_options.decompression_recipient, account_info_options.decompression_recipient,
            "decompression_recipient mismatch for lamports={}, compress={}",
            lamports, is_compress
        );

        // Both should have cpi_context_account=true
        assert!(readonly_options.cpi_context_account);
        assert!(account_info_options.cpi_context_account);

        // write_to_cpi_context should match
        assert_eq!(
            readonly_options.write_to_cpi_context, account_info_options.write_to_cpi_context,
            "write_to_cpi_context mismatch for first_set={}, set_ctx={}",
            first_set, set_ctx
        );
    }
}

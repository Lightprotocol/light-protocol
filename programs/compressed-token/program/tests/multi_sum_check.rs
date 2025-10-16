use std::collections::HashMap;

use anchor_compressed_token::ErrorCode;
use anchor_lang::AnchorSerialize;
use light_account_checks::{
    account_info::test_account_info::pinocchio::get_account_info,
    packed_accounts::ProgramPackedAccounts,
};
use light_compressed_token::transfer2::sum_check::{
    sum_check_multi_mint, validate_mint_uniqueness,
};
use light_ctoken_types::instructions::transfer2::{
    Compression, CompressionMode, MultiInputTokenDataWithContext, MultiTokenTransferOutputData,
};
use light_zero_copy::traits::ZeroCopyAt;

type Result<T> = std::result::Result<T, ErrorCode>;

#[test]
fn test_multi_sum_check() {
    // SUCCEED: no relay fee, compression
    multi_sum_check_test(&[100, 50], &[150], None, CompressionMode::Decompress).unwrap();
    multi_sum_check_test(
        &[75, 25, 25],
        &[25, 25, 25, 25, 12, 13],
        None,
        CompressionMode::Decompress,
    )
    .unwrap();

    // FAIL: no relay fee, compression
    multi_sum_check_test(&[100, 50], &[150 + 1], None, CompressionMode::Decompress).unwrap_err();
    multi_sum_check_test(&[100, 50], &[150 - 1], None, CompressionMode::Decompress).unwrap_err();
    multi_sum_check_test(&[100, 50], &[], None, CompressionMode::Decompress).unwrap_err();
    multi_sum_check_test(&[], &[100, 50], None, CompressionMode::Decompress).unwrap_err();

    // SUCCEED: empty
    multi_sum_check_test(&[], &[], None, CompressionMode::Compress).unwrap();
    multi_sum_check_test(&[], &[], None, CompressionMode::Decompress).unwrap();
    // FAIL: empty
    multi_sum_check_test(&[], &[], Some(1), CompressionMode::Decompress).unwrap_err();
    multi_sum_check_test(&[], &[], Some(1), CompressionMode::Compress).unwrap_err();

    // SUCCEED: with compress
    multi_sum_check_test(&[100], &[123], Some(23), CompressionMode::Compress).unwrap();
    multi_sum_check_test(&[], &[150], Some(150), CompressionMode::Compress).unwrap();
    // FAIL: compress
    multi_sum_check_test(&[], &[150], Some(150 - 1), CompressionMode::Compress).unwrap_err();
    multi_sum_check_test(&[], &[150], Some(150 + 1), CompressionMode::Compress).unwrap_err();

    // SUCCEED: with decompress
    multi_sum_check_test(&[100, 50], &[100], Some(50), CompressionMode::Decompress).unwrap();
    multi_sum_check_test(&[100, 50], &[], Some(150), CompressionMode::Decompress).unwrap();
    // FAIL: decompress
    multi_sum_check_test(&[100, 50], &[], Some(150 - 1), CompressionMode::Decompress).unwrap_err();
    multi_sum_check_test(&[100, 50], &[], Some(150 + 1), CompressionMode::Decompress).unwrap_err();
}

fn multi_sum_check_test(
    input_amounts: &[u64],
    output_amounts: &[u64],
    compress_or_decompress_amount: Option<u64>,
    compression_mode: CompressionMode,
) -> Result<()> {
    // Create normal types
    let inputs: Vec<_> = input_amounts
        .iter()
        .map(|&amount| MultiInputTokenDataWithContext {
            amount,
            ..Default::default()
        })
        .collect();

    let outputs: Vec<_> = output_amounts
        .iter()
        .map(|&amount| MultiTokenTransferOutputData {
            amount,
            ..Default::default()
        })
        .collect();

    let compressions = compress_or_decompress_amount.map(|amount| {
        vec![Compression {
            amount,
            mode: compression_mode,
            mint: 0, // Same mint
            source_or_recipient: 0,
            authority: 0,
            pool_account_index: 0,
            pool_index: 0,
            bump: 255,
        }]
    });

    // Serialize to bytes using borsh
    let input_bytes = inputs.try_to_vec().unwrap();
    let output_bytes = outputs.try_to_vec().unwrap();
    let compression_bytes = compressions.as_ref().map(|c| c.try_to_vec().unwrap());

    // Deserialize as zero-copy
    let (inputs_zc, _) = Vec::<MultiInputTokenDataWithContext>::zero_copy_at(&input_bytes).unwrap();
    let (outputs_zc, _) = Vec::<MultiTokenTransferOutputData>::zero_copy_at(&output_bytes).unwrap();
    let compressions_zc = if let Some(ref bytes) = compression_bytes {
        let (comp, _) = Vec::<Compression>::zero_copy_at(bytes).unwrap();
        Some(comp)
    } else {
        None
    };

    // Call our sum check function - now returns mint_map
    sum_check_multi_mint(&inputs_zc, &outputs_zc, compressions_zc.as_deref()).map(|_| ())
}

#[test]
fn test_simple_multi_mint_cases() {
    // First test a simple known case
    test_simple_multi_mint().unwrap();
}

#[test]
fn test_multi_mint_randomized() {
    // Test multiple scenarios with different mint combinations
    for scenario in 0..3000 {
        println!("Testing scenario {}", scenario);

        // Create test case with multiple mints
        let seed = scenario as u64;
        test_randomized_scenario(seed).unwrap();
    }
}
#[test]
fn test_failing_multi_mint_cases() {
    // Test specific failure cases
    test_failing_cases().unwrap();
}
fn test_simple_multi_mint() -> Result<()> {
    // Simple test: mint 0: input 100, output 100; mint 1: input 200, output 200
    let inputs = vec![(0, 100), (1, 200)];
    let outputs = vec![(0, 100), (1, 200)];
    let compressions = vec![];

    test_multi_mint_scenario(&inputs, &outputs, &compressions)?;

    // Test with compression: mint 0: input 100 + compress 50 = output 150
    let inputs = vec![(0, 100)];
    let outputs = vec![(0, 150)];
    let compressions = vec![(0, 50, CompressionMode::Compress)];

    test_multi_mint_scenario(&inputs, &outputs, &compressions)?;

    // Test with decompression: mint 0: input 200 - decompress 50 = output 150
    let inputs = vec![(0, 200)];
    let outputs = vec![(0, 150)];
    let compressions = vec![(0, 50, CompressionMode::Decompress)];

    test_multi_mint_scenario(&inputs, &outputs, &compressions)
}

fn test_randomized_scenario(seed: u64) -> Result<()> {
    let mut rng_state = seed;

    // Simple LCG for deterministic randomness
    let mut next_rand = || {
        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        rng_state
    };

    // Generate 2-4 mints
    let num_mints = 2 + (next_rand() % 3) as usize;
    let mint_ids: Vec<u8> = (0..num_mints as u8).collect();

    // Track balances per mint
    let mut mint_balances: HashMap<u8, i128> = HashMap::new();

    // Generate inputs (1-6 inputs)
    let num_inputs = 1 + (next_rand() % 6) as usize;
    let mut inputs = Vec::new();

    for _ in 0..num_inputs {
        let mint = mint_ids[(next_rand() % num_mints as u64) as usize];
        let amount = 100 + (next_rand() % 1000);

        inputs.push((mint, amount));
        *mint_balances.entry(mint).or_insert(0) += amount as i128;
    }

    // Generate compressions (0-3 compressions)
    let num_compressions = (next_rand() % 4) as usize;
    let mut compressions = Vec::new();

    for _ in 0..num_compressions {
        let mint = mint_ids[(next_rand() % num_mints as u64) as usize];
        let amount = 50 + (next_rand() % 500);
        let compression_mode = if (next_rand() % 2) == 0 {
            CompressionMode::Compress
        } else {
            CompressionMode::Decompress
        };

        compressions.push((mint, amount, compression_mode));

        if matches!(compression_mode, CompressionMode::Compress) {
            *mint_balances.entry(mint).or_insert(0) += amount as i128;
        } else {
            // Only allow decompress if the mint has sufficient balance
            let current_balance = *mint_balances.entry(mint).or_insert(0);
            if current_balance >= amount as i128 {
                *mint_balances.entry(mint).or_insert(0) -= amount as i128;
            } else {
                // Convert to compress instead to avoid negative balance
                compressions.last_mut().unwrap().2 = CompressionMode::Compress;
                *mint_balances.entry(mint).or_insert(0) += amount as i128;
            }
        }
    }

    // Ensure all balances are non-negative (adjust decompressions if needed)
    for (&mint, balance) in mint_balances.iter_mut() {
        if *balance < 0 {
            // Add compression to make balance positive
            let needed = (-*balance) as u64;
            compressions.push((mint, needed, CompressionMode::Compress));
            *balance += needed as i128;
        }
    }

    // Generate outputs that exactly match the remaining balances
    let mut outputs = Vec::new();
    for (&mint, &balance) in mint_balances.iter() {
        if balance > 0 {
            // Split the balance into 1-3 outputs
            let num_outputs = 1 + (next_rand() % 3) as usize;
            let mut remaining = balance as u64;

            for i in 0..num_outputs {
                let amount = if i == num_outputs - 1 {
                    // Last output gets the remainder
                    remaining
                } else if remaining <= 1 {
                    break; // Don't create zero-amount outputs
                } else {
                    let max_amount = remaining / (num_outputs - i) as u64;
                    if max_amount == 0 {
                        break;
                    } else {
                        1 + (next_rand() % max_amount.max(1))
                    }
                };

                if amount > 0 && remaining >= amount {
                    outputs.push((mint, amount));
                    remaining -= amount;
                } else {
                    break;
                }
            }

            // Add any remaining amount as final output
            if remaining > 0 {
                outputs.push((mint, remaining));
            }
        }
    }

    // Debug print for first scenario only
    if seed == 0 {
        println!(
            "Debug scenario {}: inputs={:?}, compressions={:?}, outputs={:?}",
            seed, inputs, compressions, outputs
        );
        println!("Balances: {:?}", mint_balances);
    }

    // Sort inputs by mint for order validation
    inputs.sort_by_key(|(mint, _)| *mint);
    // Sort outputs by mint for order validation
    outputs.sort_by_key(|(mint, _)| *mint);

    // Test the sum check
    test_multi_mint_scenario(&inputs, &outputs, &compressions)
}

fn test_failing_cases() -> Result<()> {
    // Test case 1: Wrong output amount
    let inputs = vec![(0, 100), (1, 200)];
    let outputs = vec![(0, 100), (1, 201)]; // Wrong amount
    let compressions = vec![];

    match test_multi_mint_scenario(&inputs, &outputs, &compressions) {
        Err(ErrorCode::ComputeOutputSumFailed) => {} // Expected
        Err(e) => panic!("Expected ComputeOutputSumFailed, got: {:?}", e),
        Ok(_) => panic!("Expected ComputeOutputSumFailed, but transaction succeeded"),
    }

    // Test case 2: Output for non-existent mint
    let inputs = vec![(0, 100)];
    let outputs = vec![(0, 50), (1, 50)]; // Mint 1 not in inputs
    let compressions = vec![];

    match test_multi_mint_scenario(&inputs, &outputs, &compressions) {
        Err(ErrorCode::ComputeOutputSumFailed) => {} // Expected
        _ => panic!("Should have failed with SumCheckFailed"),
    }

    // Test case 3: Too many mints (>5)
    let inputs = vec![(0, 10), (1, 10), (2, 10), (3, 10), (4, 10), (5, 10)];
    let outputs = vec![(0, 10), (1, 10), (2, 10), (3, 10), (4, 10), (5, 10)];
    let compressions = vec![];

    match test_multi_mint_scenario(&inputs, &outputs, &compressions) {
        Err(ErrorCode::TooManyMints) => {} // Expected
        _ => panic!("Should have failed with TooManyMints"),
    }

    Ok(())
}

fn test_multi_mint_scenario(
    inputs: &[(u8, u64)],                        // (mint, amount)
    outputs: &[(u8, u64)],                       // (mint, amount)
    compressions: &[(u8, u64, CompressionMode)], // (mint, amount, compression_mode)
) -> Result<()> {
    // Create input structures
    let input_structs: Vec<_> = inputs
        .iter()
        .map(|&(mint, amount)| MultiInputTokenDataWithContext {
            amount,
            mint,
            ..Default::default()
        })
        .collect();

    // Create output structures
    let output_structs: Vec<_> = outputs
        .iter()
        .map(|&(mint, amount)| MultiTokenTransferOutputData {
            amount,
            mint,
            ..Default::default()
        })
        .collect();

    // Create compression structures

    let compression_structs: Vec<_> = compressions
        .iter()
        .map(|&(mint, amount, mode)| Compression {
            amount,
            mode,
            mint,
            source_or_recipient: 0,
            authority: 0,
            pool_account_index: 0,
            pool_index: 0,
            bump: 255,
        })
        .collect();

    // Serialize to bytes
    let input_bytes = input_structs.try_to_vec().unwrap();
    let output_bytes = output_structs.try_to_vec().unwrap();
    let compression_bytes = if compression_structs.is_empty() {
        None
    } else {
        Some(compression_structs.try_to_vec().unwrap())
    };

    // Deserialize as zero-copy
    let (inputs_zc, _) = Vec::<MultiInputTokenDataWithContext>::zero_copy_at(&input_bytes).unwrap();
    let (outputs_zc, _) = Vec::<MultiTokenTransferOutputData>::zero_copy_at(&output_bytes).unwrap();
    let compressions_zc = if let Some(ref bytes) = compression_bytes {
        let (comp, _) = Vec::<Compression>::zero_copy_at(bytes).unwrap();
        Some(comp)
    } else {
        None
    };

    // Call sum check - now returns mint_map
    sum_check_multi_mint(&inputs_zc, &outputs_zc, compressions_zc.as_deref()).map(|_| ())
}

#[test]
fn test_duplicate_mint_indices() {
    // Test case 1: Same mint index used in two inputs
    // This tests that the ArrayMap correctly tracks and allows same mint index (which is valid)
    let inputs = vec![
        (0, 100), // mint index 0
        (0, 50),  // same mint index 0 - should be allowed and summed
    ];
    let outputs = vec![(0, 150)]; // Should balance
    let compressions = vec![];

    // This should SUCCEED because same mint index is allowed
    test_multi_mint_scenario(&inputs, &outputs, &compressions).unwrap();

    // Test case 2: Mint index 0 in inputs and compressions
    let inputs = vec![(0, 100)];
    let outputs = vec![(0, 150)];
    let compressions = vec![(0, 50, CompressionMode::Compress)]; // same mint index 0

    // Should SUCCEED - same mint can appear in inputs and compressions
    test_multi_mint_scenario(&inputs, &outputs, &compressions).unwrap();

    // Test case 3: Multiple compressions with same mint
    let inputs = vec![(0, 100)];
    let outputs = vec![(0, 200)];
    let compressions = vec![
        (0, 50, CompressionMode::Compress),
        (0, 50, CompressionMode::Compress),
    ];

    // Should SUCCEED - same mint in multiple compressions
    test_multi_mint_scenario(&inputs, &outputs, &compressions).unwrap();

    // Test case 4: Ensure different mints still work
    let inputs = vec![(0, 100), (1, 200)];
    let outputs = vec![(0, 100), (1, 200)];
    let compressions = vec![];

    test_multi_mint_scenario(&inputs, &outputs, &compressions).unwrap();
}

#[test]
fn test_duplicate_mint_pubkey_detection() {
    // Test that the same mint pubkey cannot be referenced by different indices
    // Setup: Create inputs/outputs with two different mint indices (0 and 1)
    // but both pointing to the same mint pubkey

    let inputs = [(0, 100), (1, 100)]; // Two different indices
    let outputs = [(0, 100), (1, 100)]; // Amounts balance correctly
    let _compressions: Vec<(u8, u64, CompressionMode)> = vec![];

    // First, verify that sum_check passes (it only checks amounts)
    let input_structs: Vec<_> = inputs
        .iter()
        .map(|&(mint, amount)| MultiInputTokenDataWithContext {
            amount,
            mint,
            ..Default::default()
        })
        .collect();

    let output_structs: Vec<_> = outputs
        .iter()
        .map(|&(mint, amount)| MultiTokenTransferOutputData {
            amount,
            mint,
            ..Default::default()
        })
        .collect();

    let input_bytes = input_structs.try_to_vec().unwrap();
    let output_bytes = output_structs.try_to_vec().unwrap();

    let (inputs_zc, _) = Vec::<MultiInputTokenDataWithContext>::zero_copy_at(&input_bytes).unwrap();
    let (outputs_zc, _) = Vec::<MultiTokenTransferOutputData>::zero_copy_at(&output_bytes).unwrap();

    // Sum check should pass (amounts are correct)
    let mint_map = sum_check_multi_mint(&inputs_zc, &outputs_zc, None).unwrap();

    // Create test accounts where indices 0 and 1 point to the SAME mint pubkey
    let same_mint_pubkey = [1u8; 32]; // Same pubkey for both
    let owner = [0u8; 32];

    let mint_account_0 =
        get_account_info(same_mint_pubkey, owner, false, false, false, vec![0u8; 82]);
    let mint_account_1 = get_account_info(
        same_mint_pubkey, // Same pubkey!
        owner,
        false,
        false,
        false,
        vec![0u8; 82],
    );

    let accounts = vec![mint_account_0, mint_account_1];
    let packed_accounts = ProgramPackedAccounts {
        accounts: &accounts,
    };

    // Now validate_mint_uniqueness should detect the duplicate and fail
    let result = validate_mint_uniqueness(&mint_map, &packed_accounts);

    match result {
        Err(ErrorCode::DuplicateMint) => {
            // Expected: duplicate mint detected
        }
        Err(e) => panic!("Expected DuplicateMint error, got: {:?}", e),
        Ok(_) => panic!("Expected DuplicateMint error, but validation passed!"),
    }
}

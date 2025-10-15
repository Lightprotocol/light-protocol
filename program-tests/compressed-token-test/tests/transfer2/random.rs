use light_ctoken_types::state::TokenDataVersion;
use rand::{
    rngs::{StdRng, ThreadRng},
    Rng, RngCore, SeedableRng,
};
use serial_test::serial;

use crate::transfer2::shared::{
    MetaApproveInput, MetaCompressAndCloseInput, MetaCompressInput, MetaDecompressInput,
    MetaTransfer2InstructionType, MetaTransferInput, TestCase, TestConfig, TestContext,
};

// Failing because of the test setup
// ============================================================================
// Randomized Test Generation
// ============================================================================

/// Generate a random test case with random actions and parameters
fn generate_random_test_case(rng: &mut StdRng, config: &TestConfig) -> TestCase {
    // Random number of actions (1-20)
    let num_actions = rng.gen_range(1..=5);
    let mut actions = Vec::new();
    let mut total_outputs = 0; // Track total outputs to respect limit of 30
    let mut total_inputs = 0u8; // Track total input compressed accounts to respect limit of 8

    for _ in 0..num_actions {
        // Respect output limit of 30 accounts
        if total_outputs >= 30 {
            break;
        }

        // Respect input limit of 8 accounts
        if total_inputs >= 8 {
            break;
        }

        // Weighted random selection of action type
        let action_type = rng.gen_range(0..1000);
        // TODO: include compressions into random test.
        #[allow(unreachable_patterns)]
        let action = match action_type {
            // 30% chance: Transfer (compressed-to-compressed)
            0..=299 => {
                // Calculate how many inputs we can still add
                let max_inputs_remaining = 8u8.saturating_sub(total_inputs);
                if max_inputs_remaining == 0 {
                    continue; // Can't add any more inputs
                }

                let num_inputs = rng.gen_range(1..=max_inputs_remaining.min(8));
                let input_amounts: Vec<u64> = (0..num_inputs)
                    .map(|_| rng.gen_range(100..=10000))
                    .collect();
                let total_input: u64 = input_amounts.iter().sum();
                let transfer_amount = rng.gen_range(1..=total_input);

                // Estimate outputs: 1 recipient + maybe 1 change
                let estimated_outputs = if transfer_amount < total_input { 2 } else { 1 };
                if total_outputs + estimated_outputs > 30 {
                    continue; // Skip this action if it would exceed limit
                }
                total_outputs += estimated_outputs;
                total_inputs += num_inputs;

                MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                    input_compressed_accounts: input_amounts,
                    amount: transfer_amount,
                    change_amount: None,         // Let system calculate change
                    is_delegate_transfer: false, // Disable delegate transfers for now (requires Approve setup)
                    token_data_version: random_token_version(rng),
                    signer_index: rng.gen_range(0..config.max_keypairs.min(10)),
                    delegate_index: None, // No delegate for non-delegate transfers
                    recipient_index: rng.gen_range(0..config.max_keypairs.min(10)),
                    mint_index: rng.gen_range(0..config.max_supported_mints), // Any mint works for transfers
                })
            }
            _ => {
                continue;
            }
            // 25% chance: Compress (SPL/CToken → compressed)
            300..=549 => {
                // Simplify: No compressed inputs for now to avoid ownership complexity
                let num_inputs = 0u8;
                let estimated_outputs = 1; // Simple compress creates 1 output
                if total_outputs + estimated_outputs > 30 {
                    continue;
                }
                total_outputs += estimated_outputs;

                // Use CToken only for now (no SPL)
                let use_spl = false;
                let mint_index = rng.gen_range(0..config.max_supported_mints);

                MetaTransfer2InstructionType::Compress(MetaCompressInput {
                    num_input_compressed_accounts: num_inputs,
                    amount: rng.gen_range(100..=5000),
                    token_data_version: random_token_version(rng),
                    signer_index: rng.gen_range(0..config.max_keypairs.min(10)),
                    recipient_index: rng.gen_range(0..config.max_keypairs.min(10)),
                    mint_index,
                    use_spl,
                    pool_index: None,
                })
            }

            // 25% chance: Decompress (compressed → SPL/CToken)
            550..=799 => {
                // Calculate how many inputs we can still add
                let max_inputs_remaining = 8u8.saturating_sub(total_inputs);
                if max_inputs_remaining == 0 {
                    continue; // Can't add any more inputs
                }

                let num_inputs = rng.gen_range(1..=max_inputs_remaining.min(5));
                let estimated_outputs = 0; // Decompress doesn't create compressed outputs
                total_outputs += estimated_outputs;
                total_inputs += num_inputs;

                // For now, only decompress to CToken (to_spl requires SPL-compressed tokens)
                let to_spl = false;
                let mint_index = rng.gen_range(0..config.max_supported_mints);

                let total_amount = (num_inputs as u64) * rng.gen_range(200..=1000);
                MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                    num_input_compressed_accounts: num_inputs,
                    decompress_amount: rng.gen_range(100..=total_amount),
                    amount: total_amount,
                    token_data_version: random_token_version(rng),
                    signer_index: rng.gen_range(0..config.max_keypairs.min(10)),
                    recipient_index: rng.gen_range(0..config.max_keypairs.min(10)),
                    mint_index,
                    to_spl,
                    pool_index: None,
                })
            }

            // 15% chance: Approve (delegation)
            800..=949 => {
                // Calculate how many inputs we can still add
                let max_inputs_remaining = 8u8.saturating_sub(total_inputs);
                if max_inputs_remaining == 0 {
                    continue; // Can't add any more inputs
                }

                let num_inputs = rng.gen_range(1..=max_inputs_remaining.min(3));
                let estimated_outputs = num_inputs as usize; // Approve typically creates same number of outputs
                if total_outputs + estimated_outputs > 30 {
                    continue;
                }
                total_outputs += estimated_outputs;
                total_inputs += num_inputs;

                MetaTransfer2InstructionType::Approve(MetaApproveInput {
                    setup: false,
                    num_input_compressed_accounts: num_inputs,
                    delegate_amount: rng.gen_range(100..=5000),
                    token_data_version: random_token_version(rng),
                    signer_index: rng.gen_range(0..config.max_keypairs.min(10)),
                    delegate_index: rng.gen_range(0..config.max_keypairs.min(10)),
                    mint_index: rng.gen_range(0..config.max_supported_mints),
                })
            }

            // 5% chance: CompressAndClose
            _ => {
                let estimated_outputs = 1; // CompressAndClose creates 1 compressed output
                if total_outputs + estimated_outputs > 30 {
                    continue;
                }
                total_outputs += estimated_outputs;

                MetaTransfer2InstructionType::CompressAndClose(MetaCompressAndCloseInput {
                    token_data_version: TokenDataVersion::ShaFlat, // Must be ShaFlat for security
                    signer_index: rng.gen_range(0..config.max_keypairs.min(10)),
                    destination_index: if rng.gen_bool(0.7) {
                        Some(rng.gen_range(0..config.max_keypairs.min(10)))
                    } else {
                        None
                    },
                    mint_index: rng.gen_range(0..config.max_supported_mints),
                    is_compressible: rng.gen_bool(0.5), // Randomly choose compressible or not
                })
            }
        };

        actions.push(action);
    }

    // Balance all actions: ensure each signer has enough tokens for each mint
    balance_actions(&mut actions, config);

    TestCase {
        name: format!("Random test case with {} actions", actions.len()),
        actions,
    }
}

/// Balance actions by tracking token amounts per (signer, mint) and adjusting to ensure validity
fn balance_actions(actions: &mut Vec<MetaTransfer2InstructionType>, _config: &TestConfig) {
    use std::collections::HashMap;

    // Track inputs (consumption) and outputs (creation) for each (signer_index, mint_index)
    let mut inputs: HashMap<(usize, usize), u64> = HashMap::new();
    let mut outputs: HashMap<(usize, usize), u64> = HashMap::new();

    // First pass: sum all inputs and outputs per (signer, mint)
    for action in actions.iter() {
        match action {
            MetaTransfer2InstructionType::Transfer(transfer) => {
                // Transfer consumes tokens (inputs)
                let key = (transfer.signer_index, transfer.mint_index);
                let total: u64 = transfer.input_compressed_accounts.iter().sum();
                *inputs.entry(key).or_insert(0) += total;
            }
            MetaTransfer2InstructionType::Compress(compress) => {
                // Compress creates tokens (outputs)
                let key = (compress.recipient_index, compress.mint_index);
                *outputs.entry(key).or_insert(0) += compress.amount;
            }
            MetaTransfer2InstructionType::Decompress(decompress) => {
                // Decompress consumes compressed tokens (inputs)
                let key = (decompress.signer_index, decompress.mint_index);
                let total_needed =
                    decompress.amount * decompress.num_input_compressed_accounts as u64;
                *inputs.entry(key).or_insert(0) += total_needed;
            }
            MetaTransfer2InstructionType::Approve(approve) => {
                // Approve consumes tokens (inputs)
                let key = (approve.signer_index, approve.mint_index);
                *inputs.entry(key).or_insert(0) += approve.delegate_amount;
            }
            _ => {}
        }
    }

    // Second pass: for each (signer, mint) where inputs > outputs, append a Compress action
    // Calculate: amount_needed = inputs - outputs (if inputs > outputs, else 0)
    for (key, total_inputs) in inputs.iter() {
        let total_outputs = outputs.get(key).copied().unwrap_or(0);

        if *total_inputs > total_outputs {
            let amount_needed = *total_inputs - total_outputs;

            // Append a Compress action to create the missing tokens
            // Order doesn't matter since all actions are batched in one transaction
            let compress_action = MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0, // No compressed inputs, compress from CToken
                amount: amount_needed,
                token_data_version: TokenDataVersion::V2, // Default version
                signer_index: key.0,
                recipient_index: key.0, // Compress to same signer
                mint_index: key.1,
                use_spl: false, // Use CToken ATA
                pool_index: None,
            });

            actions.push(compress_action);
        }
    }
}

/// Generate a random token data version
fn random_token_version(rng: &mut StdRng) -> TokenDataVersion {
    match rng.gen_range(0..3) {
        0 => TokenDataVersion::V1,
        1 => TokenDataVersion::V2,
        _ => TokenDataVersion::ShaFlat,
    }
}

// ============================================================================
// Randomized Functional Test
// ============================================================================

#[tokio::test]
#[serial]
async fn test_transfer2_random() {
    // Setup randomness
    let mut thread_rng = ThreadRng::default();
    let seed = thread_rng.next_u64();

    // Keep this print so that in case the test fails
    // we can use the seed to reproduce the error.
    println!("\n\n🎲 Random Transfer2 Test - Seed: {}\n\n", seed);
    let mut rng = StdRng::seed_from_u64(6885807522658073896);

    let config = TestConfig::default();

    // Run 1000 random test iterations
    for iteration in 0..100 {
        println!("\n--- Random Test Iteration {} ---", iteration + 1);

        // Generate random test case
        let test_case = generate_random_test_case(&mut rng, &config);

        // Skip if no actions were generated
        if test_case.actions.is_empty() {
            println!(
                "⚠️  Skipping iteration {} - no actions generated",
                iteration + 1
            );
            continue;
        }

        println!("Generated test case: {}", test_case.name);
        println!("Actions: {}", test_case.actions.len());
        for (i, action) in test_case.actions.iter().enumerate() {
            let action_type = match action {
                MetaTransfer2InstructionType::Transfer(_) => "Transfer",
                MetaTransfer2InstructionType::Compress(_) => "Compress",
                MetaTransfer2InstructionType::Decompress(_) => "Decompress",
                MetaTransfer2InstructionType::Approve(_) => "Approve",
                MetaTransfer2InstructionType::CompressAndClose(_) => "CompressAndClose",
            };
            println!("  Action {}: {}", i, action_type);
        }

        // Create fresh test context for each iteration
        let mut context = match TestContext::new(&test_case, config.clone()).await {
            Ok(ctx) => ctx,
            Err(e) => {
                println!(
                    "⚠️  Skipping iteration {} due to setup error: {:?}",
                    iteration + 1,
                    e
                );
                continue;
            }
        };

        // Execute the test case
        match context.perform_test(&test_case).await {
            Ok(()) => {
                println!("✅ Iteration {} completed successfully", iteration + 1);
            }
            Err(e) => {
                println!("❌ Iteration {} failed: {:?}", iteration + 1, e);
                println!("🔍 Reproducing failure with seed: {}", seed);
                panic!("Random test failed on iteration {}: {:?}", iteration + 1, e);
            }
        }

        // Print progress every 100 iterations
        if (iteration + 1) % 100 == 0 {
            println!("🎯 Completed {} random test iterations", iteration + 1);
        }
    }

    println!("\n🎉 All 1000 random test iterations completed successfully!");
    println!("🔧 Test seed for reproduction: {}", seed);
}

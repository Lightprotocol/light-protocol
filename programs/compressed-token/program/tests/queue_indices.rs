use anchor_compressed_token::ErrorCode;
use anchor_lang::AnchorSerialize;
use light_compressed_token::mint_action::queue_indices::QueueIndices;
use light_ctoken_types::instructions::mint_action::CpiContext;
use light_zero_copy::traits::ZeroCopyAt;

#[derive(Debug)]
struct QueueIndicesTestCase {
    name: &'static str,
    input: QueueIndicesTestInput,
    expected: QueueIndices,
}

#[derive(Debug)]
struct QueueIndicesTestInput {
    cpi_context: Option<CpiContext>,
    create_mint: bool,
    tokens_out_queue_exists: bool,
    queue_keys_match: bool,
}

fn create_zero_copy_cpi_context(cpi_context: &CpiContext) -> Vec<u8> {
    cpi_context.try_to_vec().unwrap()
}

#[test]
fn test_queue_indices_comprehensive() {
    let test_cases = vec![
        // === CPI CONTEXT CASES ===
        // When CPI context exists, all values come from context regardless of other params
        QueueIndicesTestCase {
            name: "CPI context + create_mint=true",
            input: QueueIndicesTestInput {
                cpi_context: Some(CpiContext {
                    set_context: false,
                    first_set_context: false,
                    in_tree_index: 1, // Must be 1 for execute mode with create_mint (address tree at index 1 in tree_pubkeys)
                    in_queue_index: 6,
                    out_queue_index: 7,
                    token_out_queue_index: 8,
                    assigned_account_index: 0,
                    ..Default::default()
                }),
                create_mint: true,
                tokens_out_queue_exists: false, // Ignored when CPI context exists
                queue_keys_match: false,        // Ignored when CPI context exists
            },
            expected: QueueIndices {
                in_tree_index: 0,             // 0 when create_mint=true
                address_merkle_tree_index: 1, // cpi.in_tree_index when create_mint=true (must be 1 in execute mode)
                in_queue_index: 6,            // cpi.in_queue_index
                out_token_queue_index: 8,     // cpi.token_out_queue_index
                output_queue_index: 7,        // cpi.out_queue_index
                deduplicated: false,          // Not used in CPI context path
            },
        },
        QueueIndicesTestCase {
            name: "CPI context + create_mint=false",
            input: QueueIndicesTestInput {
                cpi_context: Some(CpiContext {
                    set_context: false,
                    first_set_context: false,
                    in_tree_index: 5,
                    in_queue_index: 6,
                    out_queue_index: 7,
                    token_out_queue_index: 8,
                    assigned_account_index: 0,
                    ..Default::default()
                }),
                create_mint: false,
                tokens_out_queue_exists: true, // Ignored when CPI context exists
                queue_keys_match: true,        // Ignored when CPI context exists
            },
            expected: QueueIndices {
                in_tree_index: 5,             // cpi.in_tree_index when create_mint=false
                address_merkle_tree_index: 0, // 0 when create_mint=false
                in_queue_index: 6,            // cpi.in_queue_index
                out_token_queue_index: 8,     // cpi.token_out_queue_index
                output_queue_index: 7,        // cpi.out_queue_index
                deduplicated: false,          // Not used in CPI context path
            },
        },
        QueueIndicesTestCase {
            name: "CPI context + deduplicated=true case",
            input: QueueIndicesTestInput {
                cpi_context: Some(CpiContext {
                    set_context: false,
                    first_set_context: false,
                    in_tree_index: 5,
                    in_queue_index: 6,
                    out_queue_index: 8,       // Same as token_out_queue_index
                    token_out_queue_index: 8, // Same as out_queue_index
                    assigned_account_index: 0,
                    ..Default::default()
                }),
                create_mint: false,
                tokens_out_queue_exists: true,
                queue_keys_match: true,
            },
            expected: QueueIndices {
                in_tree_index: 5,
                address_merkle_tree_index: 0,
                in_queue_index: 6,
                out_token_queue_index: 8,
                output_queue_index: 8,
                deduplicated: false, // Not used in CPI context path
            },
        },
        QueueIndicesTestCase {
            name: "CPI context + deduplicated=false case",
            input: QueueIndicesTestInput {
                cpi_context: Some(CpiContext {
                    set_context: false,
                    first_set_context: false,
                    in_tree_index: 5,
                    in_queue_index: 6,
                    out_queue_index: 7, // Different from token_out_queue_index
                    token_out_queue_index: 8, // Different from out_queue_index
                    assigned_account_index: 0,
                    ..Default::default()
                }),
                create_mint: false,
                tokens_out_queue_exists: true,
                queue_keys_match: true,
            },
            expected: QueueIndices {
                in_tree_index: 5,
                address_merkle_tree_index: 0,
                in_queue_index: 6,
                out_token_queue_index: 8,
                output_queue_index: 7,
                deduplicated: false, // Not used in CPI context path
            },
        },
        // === NO CPI CONTEXT CASES ===
        // When no CPI context, use defaults and queue logic
        QueueIndicesTestCase {
            name: "No CPI + create_mint=true + no tokens_queue",
            input: QueueIndicesTestInput {
                cpi_context: None,
                create_mint: true,
                tokens_out_queue_exists: false,
                queue_keys_match: false, // Irrelevant when tokens_out_queue_exists=false
            },
            expected: QueueIndices {
                in_tree_index: 0,             // 0 when create_mint=true
                address_merkle_tree_index: 1, // Default when no CPI context
                in_queue_index: 2,            // Default when no CPI context
                out_token_queue_index: 0,     // No tokens queue
                output_queue_index: 0,        // Default when no CPI context
                deduplicated: false,          // tokens_out_queue_exists=false
            },
        },
        QueueIndicesTestCase {
            name: "No CPI + create_mint=false + no tokens_queue",
            input: QueueIndicesTestInput {
                cpi_context: None,
                create_mint: false,
                tokens_out_queue_exists: false,
                queue_keys_match: false, // Irrelevant when tokens_out_queue_exists=false
            },
            expected: QueueIndices {
                in_tree_index: 1,             // Default when no CPI context
                address_merkle_tree_index: 0, // 0 when create_mint=false
                in_queue_index: 2,            // Default when no CPI context
                out_token_queue_index: 0,     // No tokens queue
                output_queue_index: 0,        // Default when no CPI context
                deduplicated: false,          // tokens_out_queue_exists=false
            },
        },
        QueueIndicesTestCase {
            name: "No CPI + create_mint=true + tokens_queue + keys_match",
            input: QueueIndicesTestInput {
                cpi_context: None,
                create_mint: true,
                tokens_out_queue_exists: true,
                queue_keys_match: true,
            },
            expected: QueueIndices {
                in_tree_index: 0,             // 0 when create_mint=true
                address_merkle_tree_index: 1, // Default when no CPI context
                in_queue_index: 2,            // Default when no CPI context
                out_token_queue_index: 0,     // Queue keys match -> use same index
                output_queue_index: 0,        // Default when no CPI context
                deduplicated: true,           // tokens_out_queue_exists=true && 0==0
            },
        },
        QueueIndicesTestCase {
            name: "No CPI + create_mint=true + tokens_queue + keys_dont_match",
            input: QueueIndicesTestInput {
                cpi_context: None,
                create_mint: true,
                tokens_out_queue_exists: true,
                queue_keys_match: false,
            },
            expected: QueueIndices {
                in_tree_index: 0,             // 0 when create_mint=true
                address_merkle_tree_index: 1, // Default when no CPI context
                in_queue_index: 2,            // Default when no CPI context
                out_token_queue_index: 3,     // Queue keys don't match -> use different index
                output_queue_index: 0,        // Default when no CPI context
                deduplicated: false,          // tokens_out_queue_exists=true && 3!=0
            },
        },
        QueueIndicesTestCase {
            name: "No CPI + create_mint=false + tokens_queue + keys_match",
            input: QueueIndicesTestInput {
                cpi_context: None,
                create_mint: false,
                tokens_out_queue_exists: true,
                queue_keys_match: true,
            },
            expected: QueueIndices {
                in_tree_index: 1,             // Default when no CPI context
                address_merkle_tree_index: 0, // 0 when create_mint=false
                in_queue_index: 2,            // Default when no CPI context
                out_token_queue_index: 0,     // Queue keys match -> use same index
                output_queue_index: 0,        // Default when no CPI context
                deduplicated: true,           // tokens_out_queue_exists=true && 0==0
            },
        },
        QueueIndicesTestCase {
            name: "No CPI + create_mint=false + tokens_queue + keys_dont_match",
            input: QueueIndicesTestInput {
                cpi_context: None,
                create_mint: false,
                tokens_out_queue_exists: true,
                queue_keys_match: false,
            },
            expected: QueueIndices {
                in_tree_index: 1,             // Default when no CPI context
                address_merkle_tree_index: 0, // 0 when create_mint=false
                in_queue_index: 2,            // Default when no CPI context
                out_token_queue_index: 3,     // Queue keys don't match -> use different index
                output_queue_index: 0,        // Default when no CPI context
                deduplicated: false,          // tokens_out_queue_exists=true && 3!=0
            },
        },
    ];

    println!("\n=== QueueIndices Comprehensive Test Results ===");
    println!("Testing {} combinations\n", test_cases.len());

    for (i, test_case) in test_cases.iter().enumerate() {
        println!("Test {}: {}", i + 1, test_case.name);

        let result = if let Some(cpi_context) = &test_case.input.cpi_context {
            let serialized = create_zero_copy_cpi_context(cpi_context);
            let (zero_copy_context, _) = CpiContext::zero_copy_at(&serialized).unwrap();

            QueueIndices::new(
                Some(&zero_copy_context),
                test_case.input.create_mint,
                test_case.input.tokens_out_queue_exists,
                test_case.input.queue_keys_match,
                cpi_context.first_set_context || cpi_context.set_context,
            )
        } else {
            QueueIndices::new(
                None,
                test_case.input.create_mint,
                test_case.input.tokens_out_queue_exists,
                test_case.input.queue_keys_match,
                false,
            )
        };

        match result {
            Ok(actual) => {
                assert_eq!(actual, test_case.expected);
            }
            Err(e) => {
                println!("  ❌ ERROR: {:?}", e);
                panic!("Test case errored: {}", test_case.name);
            }
        }
    }
}

#[test]
fn test_queue_indices_invalid_address_tree_index() {
    println!("\n=== Testing Invalid Address Tree Index in Execute Mode ===");

    // Test case: Execute mode (not write_to_cpi_context) with create_mint=true
    // and in_tree_index != 1 should fail with MintActionInvalidCpiContextForCreateMint
    let cpi_context = CpiContext {
        set_context: false,
        first_set_context: false, // Execute mode
        in_tree_index: 5,         // Invalid! Must be 1 in execute mode with create_mint
        in_queue_index: 6,
        out_queue_index: 7,
        token_out_queue_index: 8,
        assigned_account_index: 0,
        ..Default::default()
    };

    let serialized = create_zero_copy_cpi_context(&cpi_context);
    let (zero_copy_context, _) = CpiContext::zero_copy_at(&serialized).unwrap();

    let result = QueueIndices::new(
        Some(&zero_copy_context),
        true,  // create_mint=true
        false, // tokens_out_queue_exists
        false, // queue_keys_match
        false, // write_to_cpi_context (execute mode)
    );

    match result {
        Ok(_) => {
            panic!("Expected MintActionInvalidCpiContextForCreateMint error, but got Ok");
        }
        Err(e) => {
            // Compare error codes by their discriminant values
            assert!(
                matches!(e, ErrorCode::MintActionInvalidCpiContextForCreateMint),
                "Expected MintActionInvalidCpiContextForCreateMint, got {:?}",
                e
            );
            println!("   Error: {:?}", e);
        }
    }

    // Test that in_tree_index=1 works correctly (positive case)
    let valid_cpi_context = CpiContext {
        set_context: false,
        first_set_context: false,
        in_tree_index: 1, // Valid!
        in_queue_index: 6,
        out_queue_index: 7,
        token_out_queue_index: 8,
        assigned_account_index: 0,
        ..Default::default()
    };

    let serialized = create_zero_copy_cpi_context(&valid_cpi_context);
    let (zero_copy_context, _) = CpiContext::zero_copy_at(&serialized).unwrap();

    let result = QueueIndices::new(
        Some(&zero_copy_context),
        true,  // create_mint=true
        false, // tokens_out_queue_exists
        false, // queue_keys_match
        false, // write_to_cpi_context (execute mode)
    );

    assert!(
        result.is_ok(),
        "Expected Ok with in_tree_index=1, got error: {:?}",
        result.err()
    );
}

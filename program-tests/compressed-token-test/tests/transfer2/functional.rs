use light_ctoken_types::state::TokenDataVersion;

use serial_test::serial;

use crate::transfer2::shared::{
    MetaCompressInput, MetaDecompressInput, MetaTransfer2InstructionType, MetaTransferInput,
    TestCase, TestConfig, TestContext,
};

// Basic Transfer Operations

//  1. 1 in 2 out Version::V1
//  2. 1 in 2 out Version::V2
//  3. 1 in 2 out Version::ShaFlat
//  4. 2 in 2 out Version::ShaFlat
//  5. 3 in 2 out Version::ShaFlat
//  6. 4 in 2 out Version::ShaFlat
//  7. 5 in 2 out Version::ShaFlat
//  8. 6 in 2 out Version::ShaFlat
//  9. 7 in 2 out Version::ShaFlat
//  10. 8 in 2 out Version::ShaFlat (maximum inputs)
//  11. Single input to multiple outputs (1→N split)
//  12. Multiple inputs to single output (N→1 merge)
//  13. Multiple inputs to multiple outputs (N→M complex)
//  14. Transfer with 0 explicit outputs (change account only)

//  Output Account Limits

//  15. 1 output compressed account
//  16. 10 output compressed accounts
//  17. 20 output compressed accounts
//  18. 35 output compressed accounts (maximum)

//  Amount Edge Cases

//  19. Transfer 0 tokens (valid operation)
//  20. Transfer 1 token (minimum non-zero)
//  21. Transfer full balance (no change account created)
//  22. Transfer partial balance (change account created)
//  23. Transfer u64::MAX tokens
//  24. Multiple partial transfers creating multiple change accounts

//  Token Data Versions

//  25. All V1 (Poseidon with pubkey hashing)
//  26. All V2 (Poseidon with pubkey hashing)
//  27. All V3/ShaFlat (SHA256)
//  28. Mixed V1 and V2 in same transaction
//  29. Mixed V1 and V3 in same transaction
//  30. Mixed V2 and V3 in same transaction
//  31. All three versions in same transaction

//  Multi-Mint Operations

//  32. Single mint operations
//  33. 2 different mints in same transaction
//  34. 3 different mints in same transaction
//  35. 4 different mints in same transaction
//  36. 5 different mints in same transaction (maximum)
//  37. Multiple operations per mint (e.g., 2 transfers of mint A, 3 of mint B)

//  Compression Operations (Path A - no compressed accounts)

//  38. Compress from SPL token only
//  39. Compress from CToken only
//  40. Decompress to SPL token only
//  41. Decompress to CToken only
//  42. Multiple compress operations only
//  43. Multiple decompress operations only
//  44. Compress and decompress same amount (must balance)

//  Mixed Compression + Transfer (Path B)

//  45. Transfer + compress SPL in same transaction
//  46. Transfer + decompress to SPL in same transaction
//  47. Transfer + compress CToken in same transaction
//  48. Transfer + decompress to CToken in same transaction
//  49. Transfer + multiple compressions
//  50. Transfer + multiple decompressions
//  51. Transfer + compress + decompress (all must balance)

//  CompressAndClose Operations

//  52. CompressAndClose as owner (no validation needed)
//  53. CompressAndClose as rent authority (requires compressible account)
//  54. Multiple CompressAndClose in single transaction
//  55. CompressAndClose + regular transfer in same transaction
//  56. CompressAndClose with full balance
//  57. CompressAndClose creating specific output (rent authority case)

//  Delegate Operations

//  58. Approve creating delegated account + change
//  59. Transfer using delegate authority (full delegated amount)
//  60. Transfer using delegate authority (partial amount)
//  61. Revoke delegation (merges all accounts)
//  62. Multiple delegates in same transaction
//  63. Delegate transfer with change account

//  Token Pool Operations

//  64. Compress to pool index 0
//  65. Compress to pool index 1
//  66. Compress to pool index 4 (max is 5)
//  67. Decompress from pool index 0
//  68. Decompress from different pool indices
//  69. Multiple pools for same mint in transaction

//  Change Account Behavior

//  70. Single change account from partial transfer
//  71. Multiple change accounts from multiple partial transfers
//  72. No change account when full amount transferred
//  73. Change account preserving delegate
//  74. Change account with different token version
//  75. Zero-amount change accounts (SDK behavior)

//  Sum Check Validation

//  76. Perfect balance single mint (inputs = outputs)
//  77. Perfect balance 2 mints
//  78. Perfect balance 5 mints (max)
//  79. Compress 1000, decompress 1000 (must balance)
//  80. Multiple compress = multiple decompress
//  81. Complex multi-mint balancing

//  Merkle Tree/Queue Targeting

//  82. All outputs to same merkle tree
//  83. Outputs to different merkle trees
//  84. Outputs to queue vs tree
//  85. Multiple trees and queues in same transaction

//  Account Reuse Patterns

//  86. Same owner multiple inputs
//  87. Same recipient multiple outputs
//  88. Circular transfer A→B, B→A in same transaction
//  89. Self-transfer (same account input and output)
//  90. Multiple operations on same mint

//  Proof Modes

//  91. Proof by index (no ZK proof)
//  92. With ZK proof
//  93. Mixed proof modes in same transaction
//  94. with_transaction_hash = true

//  Transfer Deduplication

//  95. Multiple transfers to same recipient (should deduplicate)
//  96. Up to 40 compression transfers (maximum)
//  97. Deduplication across different mints

//  Cross-Type Implicit Transfers

//  98. SPL to CToken without compressed intermediary
//  99. CToken to SPL without compressed intermediary
//  100. Mixed SPL and CToken operations

//  Complex Scenarios

//  101. Maximum complexity: 8 inputs, 35 outputs, 5 mints
//  102. All operations: transfer + compress + decompress + CompressAndClose
//  103. Circular transfers with multiple participants: A→B→C→A

#[tokio::test]
#[serial]
async fn test_transfer2_functional() {
    let config = TestConfig::default();
    let test_cases = vec![
        // Basic input account tests
        test1_basic_transfer_poseidon_v1(),
        test1_basic_transfer_poseidon_v2(),
        test1_basic_transfer_sha_flat(),
        test1_basic_transfer_sha_flat_8(),
        test1_basic_transfer_sha_flat_2_inputs(),
        test1_basic_transfer_sha_flat_3_inputs(),
        test1_basic_transfer_sha_flat_4_inputs(),
        test1_basic_transfer_sha_flat_5_inputs(),
        test1_basic_transfer_sha_flat_6_inputs(),
        test1_basic_transfer_sha_flat_7_inputs(),
        test1_basic_transfer_sha_flat_8_inputs(),
        // New complex transfer pattern tests
        test2_single_input_multiple_outputs(),
        test3_multiple_inputs_single_output(),
        test4_multiple_inputs_multiple_outputs(),
        test5_change_account_only(),
        // Output account limit tests
        test6_single_output_account(),
        test7_ten_output_accounts(),
        test8_twenty_output_accounts(),
        test9_maximum_output_accounts(),
        // Amount edge case tests
        test10_transfer_zero_tokens(),
        test11_transfer_one_token(),
        test12_transfer_full_balance(),
        test13_transfer_partial_balance(),
        test14_transfer_max_tokens(),
        test15_multiple_partial_transfers(),
        test16_all_v1_poseidon(),
        test17_all_v2_poseidon(),
        test18_all_sha_flat(),
        test19_mixed_v1_v2(),
        test20_mixed_v1_sha_flat(),
        test21_mixed_v2_sha_flat(),
        test22_all_three_versions(),
        // Multi-mint operation tests
        test23_single_mint_operations(),
        test24_two_different_mints(),
        test25_three_different_mints(),
        test26_four_different_mints(),
        test27_five_different_mints_maximum(),
        test28_multiple_operations_per_mint(),
        // Compression operations tests
        test38_compress_from_spl_only(),    // SPL compression
        test39_compress_from_ctoken_only(), // CToken compression
        test40_decompress_to_ctoken_only(),
        test41_multiple_compress_operations(),
        test42_multiple_decompress_operations(),
        test43_compress_decompress_balance(),
        test44_decompress_to_spl(),                   // SPL decompression
        test45_compress_spl_with_compressed_inputs(), // SPL compress with compressed inputs
        test46_mixed_spl_ctoken_operations(),         // Mixed SPL and CToken operations
    ];

    for (i, test_case) in test_cases.iter().enumerate() {
        println!("\n========================================");
        println!("Test #{}: {}", i + 1, test_case.name);
        println!("========================================");

        // Create test context with all initialization
        let mut ctx = TestContext::new(test_case, config.clone()).await.unwrap();

        // Execute the test
        ctx.perform_test(test_case).await.unwrap();
    }

    println!("\n========================================");
    println!("All tests completed successfully!");
    println!("========================================");
}

// ============================================================================
// Test Case Builders
// ============================================================================

fn test1_basic_transfer_poseidon_v1() -> TestCase {
    TestCase {
        name: "Basic compressed-to-compressed transfer".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![300], // One account with 300 tokens
            amount: 300,
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::V1,
            signer_index: 0,      // Owner (keypair[0]) signs the transfer
            delegate_index: None, // Not a delegate transfer
            recipient_index: 1,   // Transfer to keypair[1]
            change_amount: None,
            mint_index: 0,
        })],
    }
}

fn test1_basic_transfer_poseidon_v2() -> TestCase {
    TestCase {
        name: "Basic compressed-to-compressed transfer".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![300], // One account with 300 tokens
            amount: 300,
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::V2,
            signer_index: 0,      // Owner (keypair[0]) signs the transfer
            delegate_index: None, // Not a delegate transfer
            recipient_index: 1,   // Transfer to keypair[1]
            change_amount: None,
            mint_index: 0,
        })],
    }
}

fn test1_basic_transfer_sha_flat() -> TestCase {
    TestCase {
        name: "Basic compressed-to-compressed transfer".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![300], // One account with 300 tokens
            amount: 300,
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,      // Owner (keypair[0]) signs the transfer
            delegate_index: None, // Not a delegate transfer
            recipient_index: 1,   // Transfer to keypair[1]
            change_amount: None,
            mint_index: 0,
        })],
    }
}

fn test1_basic_transfer_sha_flat_8() -> TestCase {
    TestCase {
        name: "8 transfers from different signers using ShaFlat (max input limit)".to_string(),
        actions: (0..8) // MAX_INPUT_ACCOUNTS is 8
            .map(|i| {
                MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                    input_compressed_accounts: vec![300], // One account with 300 tokens
                    amount: 100, // Partial transfer to avoid 0-amount change accounts
                    is_delegate_transfer: false,
                    token_data_version: TokenDataVersion::ShaFlat,
                    signer_index: i,        // Each transfer from keypair 0-7
                    delegate_index: None,   // Not a delegate transfer
                    recipient_index: i + 8, // Transfer to keypair 8-15 (no overlap with signers)
                    change_amount: None,
                    mint_index: 0,
                })
            })
            .collect(),
    }
}

fn test1_basic_transfer_sha_flat_2_inputs() -> TestCase {
    TestCase {
        name: "2 transfers from different signers using ShaFlat".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![300, 300], // Two accounts with 300 tokens each
            amount: 600,
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,      // Owner (keypair[0]) signs the transfer
            delegate_index: None, // Not a delegate transfer
            recipient_index: 1,   // Transfer to keypair[1]
            change_amount: None,
            mint_index: 0,
        })],
    }
}

fn test1_basic_transfer_sha_flat_3_inputs() -> TestCase {
    TestCase {
        name: "3 transfers from different signers using ShaFlat".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![300, 300, 300], // Three accounts with 300 tokens each
            amount: 900,
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,      // Owner (keypair[0]) signs the transfer
            delegate_index: None, // Not a delegate transfer
            recipient_index: 1,   // Transfer to keypair[1]
            change_amount: None,
            mint_index: 0,
        })],
    }
}

fn test1_basic_transfer_sha_flat_4_inputs() -> TestCase {
    TestCase {
        name: "4 transfers from different signers using ShaFlat".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![300, 300, 300, 300], // Four accounts
            amount: 1200,
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,      // Owner (keypair[0]) signs the transfer
            delegate_index: None, // Not a delegate transfer
            recipient_index: 1,   // Transfer to keypair[1]
            change_amount: None,
            mint_index: 0,
        })],
    }
}

fn test1_basic_transfer_sha_flat_5_inputs() -> TestCase {
    TestCase {
        name: "5 transfers from different signers using ShaFlat".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![300, 300, 300, 300, 300], // Five accounts
            amount: 1500,
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,      // Owner (keypair[0]) signs the transfer
            delegate_index: None, // Not a delegate transfer
            recipient_index: 1,   // Transfer to keypair[1]
            change_amount: None,
            mint_index: 0,
        })],
    }
}

fn test1_basic_transfer_sha_flat_6_inputs() -> TestCase {
    TestCase {
        name: "6 transfers from different signers using ShaFlat".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![300, 300, 300, 300, 300, 300], // Six accounts
            amount: 1800,
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,      // Owner (keypair[0]) signs the transfer
            delegate_index: None, // Not a delegate transfer
            recipient_index: 1,   // Transfer to keypair[1]
            change_amount: None,
            mint_index: 0,
        })],
    }
}

fn test1_basic_transfer_sha_flat_7_inputs() -> TestCase {
    TestCase {
        name: "7 transfers from different signers using ShaFlat".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![300, 300, 300, 300, 300, 300, 300], // Seven accounts
            amount: 2100,
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,      // Owner (keypair[0]) signs the transfer
            delegate_index: None, // Not a delegate transfer
            recipient_index: 1,   // Transfer to keypair[1]
            change_amount: None,
            mint_index: 0,
        })],
    }
}

fn test1_basic_transfer_sha_flat_8_inputs() -> TestCase {
    TestCase {
        name: "8 transfers from different signers using ShaFlat (max input limit)".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![300, 300, 300, 300, 300, 300, 300, 300], // Eight accounts
            amount: 2400,
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,      // Owner (keypair[0]) signs the transfer
            delegate_index: None, // Not a delegate transfer
            recipient_index: 1,   // Transfer to keypair[1]
            change_amount: None,
            mint_index: 0,
        })],
    }
}

// Test 1: Single input to multiple outputs (1→N split)
fn test2_single_input_multiple_outputs() -> TestCase {
    TestCase {
        name: "Single input to multiple outputs (1→N split)".to_string(),
        actions: vec![
            // Transfer 100 tokens from keypair[0] to keypair[1]
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![900], // Create account with 700 tokens
                amount: 100,                          // Transfer 100
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 10,
                change_amount: Some(900 - 100 - 150 - 50),
                mint_index: 0,
            }),
            // Transfer 150 tokens from keypair[0] to keypair[2]
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![], // Uses existing input from first transfer
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 12,
                change_amount: Some(0),
                mint_index: 0,
            }),
            // Transfer 50 tokens from keypair[0] to keypair[3]
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![], // Uses existing input from first transfer
                amount: 50,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                delegate_index: None,
                recipient_index: 13,
                change_amount: Some(0),
                mint_index: 0,
            }),
        ],
    }
}

// Test 2: Multiple inputs to single output (N→1 merge)
fn test3_multiple_inputs_single_output() -> TestCase {
    TestCase {
        name: "Multiple inputs to single output (N→1 merge)".to_string(),
        actions: vec![
            // Transfer from keypair[0] to keypair[5]
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![200, 200],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 5,
                change_amount: None,
                mint_index: 0,
            }),
            // Transfer from keypair[1] to keypair[5] (same recipient)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![150],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 5,
                change_amount: None,
                mint_index: 0,
            }),
            // Transfer from keypair[2] to keypair[5] (same recipient)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![100],
                amount: 100,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                delegate_index: None,
                recipient_index: 5,
                change_amount: None,
                mint_index: 0,
            }),
        ],
    }
}

// Test 3: Multiple inputs to multiple outputs (N→M complex)
fn test4_multiple_inputs_multiple_outputs() -> TestCase {
    TestCase {
        name: "Multiple inputs to multiple outputs (N→M complex)".to_string(),
        actions: vec![
            // Transfer from keypair[0] - split to multiple recipients
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![100, 100],
                amount: 100,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 3,
                change_amount: Some(50), // Keep 100 as change for next transfer
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![], // Reuse input
                amount: 50,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 4,
                change_amount: Some(0), // Use 50 from change, keep 50 remaining
                mint_index: 0,
            }),
            // Transfer from keypair[1] - split to multiple recipients
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![100, 100],
                amount: 75,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 3,     // Same recipient as above
                change_amount: Some(0), // Keep 125 as change for next transfer
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![], // Reuse input
                amount: 125,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 5,
                change_amount: Some(0), // Use all 125 from change
                mint_index: 0,
            }),
            // Transfer from keypair[2] to multiple recipients
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![80],
                amount: 80,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                delegate_index: None,
                recipient_index: 4,     // Same recipient as above
                change_amount: Some(0), // Exact amount, no change
                mint_index: 0,
            }),
        ],
    }
}

// Test 4: Transfer with 0 explicit outputs (change account only)
fn test5_change_account_only() -> TestCase {
    TestCase {
        name: "Transfer with change account only (partial transfer to self)".to_string(),
        actions: vec![
            // Transfer partial amount to self - creates only a change account
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150, // Partial amount, leaving 150 as change
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 0, // Transfer to self
                change_amount: None,
                mint_index: 0,
            }),
        ],
    }
}

// ============================================================================
// Output Account Limit Tests (12-15)
// ============================================================================

// Test 6: Single output compressed account (minimum)
fn test6_single_output_account() -> TestCase {
    TestCase {
        name: "Single output compressed account".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![100], // One input account
            amount: 100,                          // Transfer full amount
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,
            delegate_index: None,
            recipient_index: 1,     // Single output
            change_amount: Some(0), // No change (full amount transfer)
            mint_index: 0,
        })],
    }
}

// Test 7: 10 output compressed accounts
fn test7_ten_output_accounts() -> TestCase {
    TestCase {
        name: "10 output compressed accounts".to_string(),
        actions: {
            let mut actions = vec![];
            // Create one large input account to split into 10 outputs
            let total_amount = 1000u64;
            let amount_per_output = 100u64;

            // First transfer with input account, creates change for subsequent transfers
            actions.push(MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![total_amount],
                amount: amount_per_output,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: Some(0), // Keep remaining as change
                mint_index: 0,
            }));

            // 9 more transfers using the change from the first transfer
            for i in 1..10 {
                let remaining_change = total_amount - (amount_per_output * (i as u64 + 1));
                actions.push(MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                    input_compressed_accounts: vec![], // Use change from previous
                    amount: amount_per_output,
                    is_delegate_transfer: false,
                    token_data_version: TokenDataVersion::ShaFlat,
                    signer_index: 0,
                    delegate_index: None,
                    recipient_index: i + 1, // Recipients 2-10
                    change_amount: Some(0),
                    mint_index: 0,
                }));
            }

            actions
        },
    }
}

// Test 8: 20 output compressed accounts
fn test8_twenty_output_accounts() -> TestCase {
    TestCase {
        name: "20 output compressed accounts".to_string(),
        actions: {
            let mut actions = vec![];
            let total_amount = 2000u64;
            let amount_per_output = 100u64;

            // First transfer with input account
            actions.push(MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![total_amount],
                amount: amount_per_output,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: Some(0),
                mint_index: 0,
            }));

            // 19 more transfers using the change
            for i in 1..20 {
                let remaining_change = total_amount - (amount_per_output * (i as u64 + 1));
                actions.push(MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                    input_compressed_accounts: vec![],
                    amount: amount_per_output,
                    is_delegate_transfer: false,
                    token_data_version: TokenDataVersion::ShaFlat,
                    signer_index: 0,
                    delegate_index: None,
                    recipient_index: i + 1, // Recipients 2-20
                    change_amount: Some(0),
                    mint_index: 0,
                }));
            }

            actions
        },
    }
}

// Test 9: 35 output compressed accounts (maximum per instruction)
fn test9_maximum_output_accounts() -> TestCase {
    TestCase {
        name: "35 output compressed accounts (maximum)".to_string(),
        actions: {
            let mut actions = vec![];
            let total_amount = 2900u64; // 35 * 100
            let amount_per_output = 100u64;

            // First transfer with input account
            actions.push(MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![total_amount],
                amount: amount_per_output,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: Some(0),
                mint_index: 0,
            }));

            // 34 more transfers to reach the maximum of 35 outputs
            for i in 1..29 {
                let remaining_change = total_amount - (amount_per_output * (i as u64 + 1));
                actions.push(MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                    input_compressed_accounts: vec![],
                    amount: amount_per_output,
                    is_delegate_transfer: false,
                    token_data_version: TokenDataVersion::ShaFlat,
                    signer_index: 0,
                    delegate_index: None,
                    recipient_index: i + 1, // Recipients 2-35
                    change_amount: Some(0),
                    mint_index: 0,
                }));
            }

            actions
        },
    }
}

// ============================================================================
// Amount Edge Case Tests (16-21)
// ============================================================================

// Test 10: Transfer 0 tokens (valid operation)
fn test10_transfer_zero_tokens() -> TestCase {
    TestCase {
        name: "Transfer 0 tokens (valid operation)".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![1000],
            amount: 0, // Transfer 0 tokens
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,
            delegate_index: None,
            recipient_index: 1,
            change_amount: None, // Keep all 1000 as change
            mint_index: 0,
        })],
    }
}

// Test 11: Transfer 1 token (minimum non-zero)
fn test11_transfer_one_token() -> TestCase {
    TestCase {
        name: "Transfer 1 token (minimum non-zero)".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![1000],
            amount: 1, // Transfer 1 token
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,
            delegate_index: None,
            recipient_index: 1,
            change_amount: None, // Keep 999 as change
            mint_index: 0,
        })],
    }
}

// Test 12: Transfer full balance (no change account created)
fn test12_transfer_full_balance() -> TestCase {
    TestCase {
        name: "Transfer full balance (no change account created)".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![1000],
            amount: 1000, // Transfer full amount
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,
            delegate_index: None,
            recipient_index: 1,
            change_amount: Some(0), // No change account
            mint_index: 0,
        })],
    }
}

// Test 13: Transfer partial balance (change account created)
fn test13_transfer_partial_balance() -> TestCase {
    TestCase {
        name: "Transfer partial balance (change account created)".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![1000],
            amount: 750, // Partial transfer
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,
            delegate_index: None,
            recipient_index: 1,
            change_amount: None, // Keep 250 as change
            mint_index: 0,
        })],
    }
}

// Test 14: Transfer u64::MAX tokens (maximum possible)
fn test14_transfer_max_tokens() -> TestCase {
    TestCase {
        name: "Transfer u64::MAX tokens (maximum possible)".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![u64::MAX],
            amount: u64::MAX, // Maximum amount
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,
            delegate_index: None,
            recipient_index: 1,
            change_amount: Some(0), // No change account
            mint_index: 0,
        })],
    }
}

// Test 15: Multiple partial transfers creating multiple change accounts
fn test15_multiple_partial_transfers() -> TestCase {
    TestCase {
        name: "Multiple partial transfers creating multiple change accounts".to_string(),
        actions: vec![
            // First partial transfer
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![1000],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None, // Keep 800 as change
                mint_index: 0,
            }),
            // Second partial transfer from different account
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 2,
                change_amount: None, // Keep 350 as change
                mint_index: 0,
            }),
            // Third partial transfer from another account
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![800],
                amount: 300,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                delegate_index: None,
                recipient_index: 3,
                change_amount: None, // Keep 500 as change
                mint_index: 0,
            }),
        ],
    }
}
// ============================================================================
// Token Data Version Tests (22-28)
// ============================================================================

// Test 16: All V1 (Poseidon with pubkey hashing)
fn test16_all_v1_poseidon() -> TestCase {
    TestCase {
        name: "All V1 (Poseidon with pubkey hashing)".to_string(),
        actions: vec![
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::V1,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::V1,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 2,
                change_amount: None,
                mint_index: 0,
            }),
        ],
    }
}

// Test 17: All V2 (Poseidon with pubkey hashing)
fn test17_all_v2_poseidon() -> TestCase {
    TestCase {
        name: "All V2 (Poseidon with pubkey hashing)".to_string(),
        actions: vec![
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::V2,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::V2,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 2,
                change_amount: None,
                mint_index: 0,
            }),
        ],
    }
}

// Test 18: All V3/ShaFlat (SHA256)
fn test18_all_sha_flat() -> TestCase {
    TestCase {
        name: "All V3/ShaFlat (SHA256)".to_string(),
        actions: vec![
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 2,
                change_amount: None,
                mint_index: 0,
            }),
        ],
    }
}

// Test 19: Mixed V1 and V2 in same transaction
fn test19_mixed_v1_v2() -> TestCase {
    TestCase {
        name: "Mixed V1 and V2 in same transaction".to_string(),
        actions: vec![
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::V1, // V1 transfer
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::V2, // V2 transfer
                signer_index: 1,
                delegate_index: None,
                recipient_index: 2,
                change_amount: None,
                mint_index: 0,
            }),
        ],
    }
}

// Test 20: Mixed V1 and V3 in same transaction
fn test20_mixed_v1_sha_flat() -> TestCase {
    TestCase {
        name: "Mixed V1 and V3 in same transaction".to_string(),
        actions: vec![
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::V1, // V1 transfer
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat, // ShaFlat transfer
                signer_index: 1,
                delegate_index: None,
                recipient_index: 2,
                change_amount: None,
                mint_index: 0,
            }),
        ],
    }
}

// Test 21: Mixed V2 and V3 in same transaction
fn test21_mixed_v2_sha_flat() -> TestCase {
    TestCase {
        name: "Mixed V2 and V3 in same transaction".to_string(),
        actions: vec![
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::V2, // V2 transfer
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat, // ShaFlat transfer
                signer_index: 1,
                delegate_index: None,
                recipient_index: 2,
                change_amount: None,
                mint_index: 0,
            }),
        ],
    }
}

// Test 22: All three versions in same transaction
fn test22_all_three_versions() -> TestCase {
    TestCase {
        name: "All three versions in same transaction".to_string(),
        actions: vec![
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::V1, // V1 transfer
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::V2, // V2 transfer
                signer_index: 1,
                delegate_index: None,
                recipient_index: 2,
                change_amount: None,
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![400],
                amount: 100,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat, // ShaFlat transfer
                signer_index: 2,
                delegate_index: None,
                recipient_index: 3,
                change_amount: None,
                mint_index: 0,
            }),
        ],
    }
}

// ============================================================================
// Multi-Mint Operation Tests (29-34)
// ============================================================================

// Test 23: Single mint operations
fn test23_single_mint_operations() -> TestCase {
    TestCase {
        name: "Single mint operations".to_string(),
        actions: vec![
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 2,
                change_amount: None,
                mint_index: 0,
            }),
        ],
    }
}

// Test 24: 2 different mints in same transaction
fn test24_two_different_mints() -> TestCase {
    TestCase {
        name: "2 different mints in same transaction".to_string(),
        actions: vec![
            // Transfer from mint A
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0,
            }),
            // Transfer from mint B (different mint)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1, // Different signer implies different mint
                delegate_index: None,
                recipient_index: 2,
                change_amount: None,
                mint_index: 1,
            }),
        ],
    }
}

// Test 25: 3 different mints in same transaction
fn test25_three_different_mints() -> TestCase {
    TestCase {
        name: "3 different mints in same transaction".to_string(),
        actions: vec![
            // Transfer from mint A
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 3,
                change_amount: None,
                mint_index: 0,
            }),
            // Transfer from mint B
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 4,
                change_amount: None,
                mint_index: 1,
            }),
            // Transfer from mint C
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![400],
                amount: 100,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                delegate_index: None,
                recipient_index: 5,
                change_amount: None,
                mint_index: 2,
            }),
        ],
    }
}

// Test 26: 4 different mints in same transaction
fn test26_four_different_mints() -> TestCase {
    TestCase {
        name: "4 different mints in same transaction".to_string(),
        actions: vec![
            // Transfer from mint A
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 4,
                change_amount: None,
                mint_index: 0,
            }),
            // Transfer from mint B
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 5,
                change_amount: None,
                mint_index: 1,
            }),
            // Transfer from mint C
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![400],
                amount: 100,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                delegate_index: None,
                recipient_index: 6,
                change_amount: None,
                mint_index: 2,
            }),
            // Transfer from mint D
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![600],
                amount: 250,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 3,
                delegate_index: None,
                recipient_index: 7,
                change_amount: None,
                mint_index: 3,
            }),
        ],
    }
}

// Test 27: 5 different mints in same transaction (maximum)
fn test27_five_different_mints_maximum() -> TestCase {
    TestCase {
        name: "5 different mints in same transaction (maximum)".to_string(),
        actions: vec![
            // Transfer from mint A
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 5,
                change_amount: None,
                mint_index: 0,
            }),
            // Transfer from mint B
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 6,
                change_amount: None,
                mint_index: 1,
            }),
            // Transfer from mint C
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![400],
                amount: 100,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                delegate_index: None,
                recipient_index: 7,
                change_amount: None,
                mint_index: 2,
            }),
            // Transfer from mint D
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![600],
                amount: 250,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 3,
                delegate_index: None,
                recipient_index: 8,
                change_amount: None,
                mint_index: 3,
            }),
            // Transfer from mint E
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![700],
                amount: 300,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 4,
                delegate_index: None,
                recipient_index: 9,
                change_amount: None,
                mint_index: 4,
            }),
        ],
    }
}

// Test 28: Multiple operations per mint (2 transfers of mint A, 3 of mint B)
fn test28_multiple_operations_per_mint() -> TestCase {
    TestCase {
        name: "Multiple operations per mint (2 transfers of mint A, 3 of mint B)".to_string(),
        actions: vec![
            // First transfer from mint A (signer 0)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0, // Mint A, signer 0
                delegate_index: None,
                recipient_index: 10,
                change_amount: None,
                mint_index: 0,
            }),
            // Second transfer from mint A (different signer to avoid double spend)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2, // Mint A, different signer (2)
                delegate_index: None,
                recipient_index: 11,
                change_amount: None,
                mint_index: 0,
            }),
            // First transfer from mint B (signer 1)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![400],
                amount: 100,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1, // Mint B, signer 1
                delegate_index: None,
                recipient_index: 12,
                change_amount: None,
                mint_index: 1,
            }),
            // Second transfer from mint B (different signer to avoid double spend)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![600],
                amount: 250,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 3, // Mint B, different signer (3)
                delegate_index: None,
                recipient_index: 13,
                change_amount: None,
                mint_index: 1,
            }),
            // Third transfer from mint B (another different signer to avoid double spend)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![350],
                amount: 175,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 4, // Mint B, different signer (4)
                delegate_index: None,
                recipient_index: 14,
                change_amount: None,
                mint_index: 1,
            }),
        ],
    }
}

// ============================================================================
// Compression Operations Tests (39-44)
// ============================================================================

// Test 38: Compress from SPL token only
fn test38_compress_from_spl_only() -> TestCase {
    TestCase {
        name: "Compress from SPL token only".to_string(),
        actions: vec![MetaTransfer2InstructionType::Compress(MetaCompressInput {
            num_input_compressed_accounts: 0, // No compressed inputs
            amount: 1000,                     // Amount to compress from SPL token account
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,    // Owner of the SPL token account
            recipient_index: 0, // Compress to same owner
            mint_index: 0,
            use_spl: true, // Use SPL token account
        })],
    }
}

// Test 39: Compress from CToken only
fn test39_compress_from_ctoken_only() -> TestCase {
    TestCase {
        name: "Compress from CToken only".to_string(),
        actions: vec![MetaTransfer2InstructionType::Compress(MetaCompressInput {
            num_input_compressed_accounts: 0, // No compressed inputs
            amount: 1000,                     // Amount to compress from CToken ATA
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,    // Owner of the CToken ATA
            recipient_index: 0, // Compress to same owner
            mint_index: 0,
            use_spl: false, // Use CToken ATA
        })],
    }
}

// Test 40: Decompress to CToken only
fn test40_decompress_to_ctoken_only() -> TestCase {
    TestCase {
        name: "Decompress to CToken only".to_string(),
        actions: vec![MetaTransfer2InstructionType::Decompress(
            MetaDecompressInput {
                num_input_compressed_accounts: 1, // One compressed account as input
                decompress_amount: 800,
                amount: 800,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,    // Owner of compressed tokens
                recipient_index: 1, // Decompress to different recipient
                mint_index: 0,
                to_spl: false, // Decompress to CToken ATA
            },
        )],
    }
}

// Test 41: Multiple compress operations only
fn test41_multiple_compress_operations() -> TestCase {
    TestCase {
        name: "Multiple compress operations only".to_string(),
        actions: vec![
            // First compress from signer 0
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 500,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                recipient_index: 0,
                mint_index: 0,
                use_spl: false, // Use CToken ATA
            }),
            // Second compress from signer 1
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 750,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                recipient_index: 1,
                mint_index: 0,
                use_spl: false, // Use CToken ATA
            }),
            // Third compress from signer 2
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 250,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                recipient_index: 2,
                mint_index: 0,
                use_spl: false, // Use CToken ATA
            }),
        ],
    }
}

// Test 42: Multiple decompress operations only
fn test42_multiple_decompress_operations() -> TestCase {
    TestCase {
        name: "Multiple decompress operations only".to_string(),
        actions: vec![
            // First decompress to recipient 0
            MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                num_input_compressed_accounts: 1,
                decompress_amount: 400,
                amount: 400,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                recipient_index: 3, // Different recipient
                mint_index: 0,
                to_spl: false, // Decompress to CToken ATA
            }),
            // Second decompress to recipient 1
            MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                num_input_compressed_accounts: 1,
                decompress_amount: 300,
                amount: 300,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                recipient_index: 4, // Different recipient
                mint_index: 0,
                to_spl: false, // Decompress to CToken ATA
            }),
            // Third decompress to recipient 2
            MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                num_input_compressed_accounts: 1,
                decompress_amount: 200,
                amount: 200,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                recipient_index: 5, // Different recipient
                mint_index: 0,
                to_spl: false, // Decompress to CToken ATA
            }),
        ],
    }
}

// Test 43: Compress and decompress same amount (must balance)
fn test43_compress_decompress_balance() -> TestCase {
    TestCase {
        name: "Compress and decompress same amount (must balance)".to_string(),
        actions: vec![
            // Compress 1000 tokens from CToken
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 1000,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                recipient_index: 0,
                mint_index: 0,
                use_spl: false, // Use CToken ATA
            }),
            // Decompress 1000 tokens to different CToken
            MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                num_input_compressed_accounts: 1,
                decompress_amount: 1000,
                amount: 1000,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                recipient_index: 2, // Different recipient
                mint_index: 0,
                to_spl: false, // Decompress to CToken ATA
            }),
        ],
    }
}

// Test 44: Decompress to SPL token account
fn test44_decompress_to_spl() -> TestCase {
    TestCase {
        name: "Decompress to SPL token account".to_string(),
        actions: vec![MetaTransfer2InstructionType::Decompress(
            MetaDecompressInput {
                num_input_compressed_accounts: 1, // One compressed account as input
                decompress_amount: 600,
                amount: 600,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,    // Owner of compressed tokens
                recipient_index: 1, // Decompress to different recipient
                mint_index: 0,
                to_spl: true, // Decompress to SPL token account
            },
        )],
    }
}

// Test 45: Compress SPL with multiple compressed account inputs
fn test45_compress_spl_with_compressed_inputs() -> TestCase {
    TestCase {
        name: "Compress SPL with compressed inputs".to_string(),
        actions: vec![MetaTransfer2InstructionType::Compress(MetaCompressInput {
            num_input_compressed_accounts: 2, // Use 2 compressed accounts plus SPL account
            amount: 1500,                     // Total to compress (from both compressed + SPL)
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,
            recipient_index: 0,
            mint_index: 0,
            use_spl: true, // Use SPL token account
        })],
    }
}

// Test 46: Mixed SPL and CToken operations
fn test46_mixed_spl_ctoken_operations() -> TestCase {
    TestCase {
        name: "Mixed SPL and CToken operations".to_string(),
        actions: vec![
            // Compress from SPL
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 500,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                recipient_index: 0,
                mint_index: 0,
                use_spl: true, // SPL source
            }),
            // Compress from CToken
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 300,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                recipient_index: 1,
                mint_index: 1,
                use_spl: false, // CToken source
            }),
            // Decompress to CToken
            MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                num_input_compressed_accounts: 1,
                decompress_amount: 400,
                amount: 400,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                recipient_index: 3, // Different recipient
                mint_index: 0,
                to_spl: false, // Decompress to CToken ATA
            }),
        ],
    }
}

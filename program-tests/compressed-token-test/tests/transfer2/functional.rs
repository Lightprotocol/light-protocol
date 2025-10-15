use light_ctoken_types::state::TokenDataVersion;
use serial_test::serial;

use crate::transfer2::shared::{
    MetaApproveInput, MetaCompressAndCloseInput, MetaCompressInput, MetaDecompressInput,
    MetaTransfer2InstructionType, MetaTransferInput, TestCase, TestConfig, TestContext,
};

// Basic Transfer Operations

//  1. 1 in 1 out Version::V1
//  2. 1 in 1 out Version::V2
//  3. 1 in 1 out Version::ShaFlat
//  4. 8 transfers from different signers using ShaFlat (max concurrent signers)
//  5. 2 in 1 out Version::ShaFlat
//  6. 3 in 1 out Version::ShaFlat
//  7. 4 in 1 out Version::ShaFlat
//  8. 5 in 1 out Version::ShaFlat
//  9. 6 in 1 out Version::ShaFlat
//  10. 7 in 1 out Version::ShaFlat
//  11. 8 in 1 out Version::ShaFlat (maximum inputs)
//  12. Single input to multiple outputs (1→N split)
//  13. Multiple inputs to single output (N→1 merge)
//  14. Multiple inputs to multiple outputs (N→M complex)
//  15. Transfer with 0 explicit outputs (change account only)

//  Output Account Limits

//  16. 1 output compressed account
//  17. 10 output compressed accounts
//  18. 20 output compressed accounts
//  19. 35 output compressed accounts (maximum)

//  Amount Edge Cases

//  20. Transfer 0 tokens (valid operation)
//  21. Transfer 1 token (minimum non-zero)
//  22. Transfer full balance (no change account created)
//  23. Transfer partial balance (change account created)
//  24. Transfer u64::MAX tokens
//  25. Multiple partial transfers creating multiple change accounts

//  Token Data Versions

//  26. All V1 (Poseidon with pubkey hashing)
//  27. All V2 (Poseidon with pubkey hashing)
//  28. All V3/ShaFlat (SHA256)
//  29. Mixed V1 and V2 in same transaction
//  30. Mixed V1 and V3 in same transaction
//  31. Mixed V2 and V3 in same transaction
//  32. All three versions in same transaction

//  Multi-Mint Operations

//  33. Single mint operations
//  34. 2 different mints in same transaction
//  35. 3 different mints in same transaction
//  36. 4 different mints in same transaction
//  37. 5 different mints in same transaction (maximum)
//  38. Multiple operations per mint (e.g., 2 transfers of mint A, 3 of mint B)

//  Compression Operations (Path A - no compressed accounts)

//  39. Compress from SPL token only
//  40. Compress from CToken only
//  41. Decompress to CToken only
//  42. Multiple compress operations only
//  43. Multiple decompress operations only
//  44. Compress and decompress same amount (must balance)
//  45. Decompress to SPL token only
//  46. Compress SPL with multiple compressed account inputs
//  47. Mixed SPL and CToken operations

//  Mixed Compression + Transfer (Path B) - NOT YET IMPLEMENTED

//  48. Transfer + compress SPL in same transaction
//  49. Transfer + decompress to SPL in same transaction
//  50. Transfer + compress CToken in same transaction
//  51. Transfer + decompress to CToken in same transaction
//  52. Transfer + multiple compressions
//  53. Transfer + multiple decompressions
//  54. Transfer + compress + decompress (all must balance)

//  CompressAndClose Operations

//  55. CompressAndClose as owner (both non-compressible and compressible versions)
//  56. CompressAndClose with destination (compressible, rent to specific recipient)
//  57. Multiple CompressAndClose in single transaction (compressible)
//  58. CompressAndClose + regular transfer in same transaction (compressible)
//  59. CompressAndClose with full balance (compressible)
//  60. CompressAndClose creating specific output (compressible, rent authority case)

//  Delegate Operations

//  61. Approve creating delegated account + change
//  62. Transfer using delegate authority (full delegated amount)
//  63. Transfer using delegate authority (partial amount)
//  64. Revoke delegation (merges all accounts)
//  65. Multiple delegates in same transaction
//  66. Delegate transfer with change account

//  Token Pool Operations

//  67. Compress to pool index 0
//  68. Compress to pool index 1
//  69. Compress to pool index 4 (max is 5)
//  70. Decompress from pool index 0
//  71. Decompress from different pool indices
//  72. Multiple pools for same mint in transaction

#[tokio::test]
#[serial]
async fn test_transfer2_functional() {
    let config = TestConfig::default();
    let test_cases = vec![
        // Basic Transfer Operations (1-15)
        test1_basic_transfer_poseidon_v1(),
        test2_basic_transfer_poseidon_v2(),
        test3_basic_transfer_sha_flat(),
        test4_basic_transfer_sha_flat_8(),
        test5_basic_transfer_sha_flat_2_inputs(),
        test6_basic_transfer_sha_flat_3_inputs(),
        test7_basic_transfer_sha_flat_4_inputs(),
        test8_basic_transfer_sha_flat_5_inputs(),
        test9_basic_transfer_sha_flat_6_inputs(),
        test10_basic_transfer_sha_flat_7_inputs(),
        test11_basic_transfer_sha_flat_8_inputs(),
        test12_single_input_multiple_outputs(),
        test13_multiple_inputs_single_output(),
        test14_multiple_inputs_multiple_outputs(),
        test15_change_account_only(),
        // Output Account Limits (16-19)
        test16_single_output_account(),
        test17_ten_output_accounts(),
        test18_twenty_output_accounts(),
        test19_maximum_output_accounts(),
        // Amount Edge Cases (20-25)
        test20_transfer_zero_tokens(),
        test21_transfer_one_token(),
        test22_transfer_full_balance(),
        test23_transfer_partial_balance(),
        test24_transfer_max_tokens(),
        test25_multiple_partial_transfers(),
        // Token Data Versions (26-32)
        test26_all_v1_poseidon(),
        test27_all_v2_poseidon(),
        test28_all_sha_flat(),
        test29_mixed_v1_v2(),
        test30_mixed_v1_sha_flat(),
        test31_mixed_v2_sha_flat(),
        test32_all_three_versions(),
        // Multi-Mint Operations (33-38)
        test33_single_mint_operations(),
        test34_two_different_mints(),
        test35_three_different_mints(),
        test36_four_different_mints(),
        test37_five_different_mints_maximum(),
        test38_multiple_operations_per_mint(),
        // Compression Operations (39-47)
        test39_compress_from_spl_only(),
        test40_compress_from_ctoken_only(),
        test41_decompress_to_ctoken_only(),
        test42_multiple_compress_operations(),
        test43_multiple_decompress_operations(),
        test44_compress_decompress_balance(),
        test45_decompress_to_spl(),
        test46_compress_spl_with_compressed_inputs(),
        test47_mixed_spl_ctoken_operations(),
        test48_transfer_compress_spl(),
        test49_transfer_decompress_spl(),
        test50_transfer_compress_ctoken(),
        test51_transfer_decompress_ctoken(),
        test52_transfer_multiple_compressions(),
        test53_transfer_multiple_decompressions(),
        test54_transfer_compress_decompress_balanced(),
        test55_compress_and_close_as_owner(),
        test55_compress_and_close_as_owner_compressible(),
        test56_compress_and_close_with_destination(),
        test57_multiple_compress_and_close(),
        test58_compress_and_close_with_transfer(),
        test59_compress_and_close_full_balance(),
        test60_compress_and_close_specific_output(),
        // Delegate Operations (61-66)
        test61_approve_with_change(),
        test62_delegate_transfer_single_input(),
        test63_delegate_transfer_partial_amount(),
        test64_revoke_delegation(),
        test65_multiple_delegates(),
        test66_delegate_transfer_with_change(),
        // Token Pool Operations (67-72)
        test67_compress_to_pool_index_0(),
        test68_compress_to_pool_index_1(),
        test69_compress_to_pool_index_4(),
        test70_decompress_from_pool_index_0(),
        test71_decompress_from_different_pools(),
        test72_multiple_pools_same_mint(),
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

fn test2_basic_transfer_poseidon_v2() -> TestCase {
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

fn test3_basic_transfer_sha_flat() -> TestCase {
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

fn test4_basic_transfer_sha_flat_8() -> TestCase {
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

fn test5_basic_transfer_sha_flat_2_inputs() -> TestCase {
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

fn test6_basic_transfer_sha_flat_3_inputs() -> TestCase {
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

fn test7_basic_transfer_sha_flat_4_inputs() -> TestCase {
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

fn test8_basic_transfer_sha_flat_5_inputs() -> TestCase {
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

fn test9_basic_transfer_sha_flat_6_inputs() -> TestCase {
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

fn test10_basic_transfer_sha_flat_7_inputs() -> TestCase {
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

fn test11_basic_transfer_sha_flat_8_inputs() -> TestCase {
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

// Test 12: Single input to multiple outputs (1→N split)
fn test12_single_input_multiple_outputs() -> TestCase {
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

// Test 13: Multiple inputs to single output (N→1 merge)
fn test13_multiple_inputs_single_output() -> TestCase {
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

// Test 14: Multiple inputs to multiple outputs (N→M complex)
fn test14_multiple_inputs_multiple_outputs() -> TestCase {
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

// Test 15: Transfer with 0 explicit outputs (change account only)
fn test15_change_account_only() -> TestCase {
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
// Output Account Limit Tests (16-19)
// ============================================================================

// Test 16: Single output compressed account (minimum)
fn test16_single_output_account() -> TestCase {
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

// Test 17: 10 output compressed accounts
fn test17_ten_output_accounts() -> TestCase {
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

// Test 18: 20 output compressed accounts
fn test18_twenty_output_accounts() -> TestCase {
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

// Test 19: 35 output compressed accounts (maximum per instruction)
fn test19_maximum_output_accounts() -> TestCase {
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
// Amount Edge Case Tests (20-25)
// ============================================================================

// Test 20: Transfer 0 tokens (valid operation)
fn test20_transfer_zero_tokens() -> TestCase {
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

// Test 21: Transfer 1 token (minimum non-zero)
fn test21_transfer_one_token() -> TestCase {
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

// Test 22: Transfer full balance (no change account created)
fn test22_transfer_full_balance() -> TestCase {
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

// Test 23: Transfer partial balance (change account created)
fn test23_transfer_partial_balance() -> TestCase {
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

// Test 24: Transfer u64::MAX tokens (maximum possible)
fn test24_transfer_max_tokens() -> TestCase {
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

// Test 25: Multiple partial transfers creating multiple change accounts
fn test25_multiple_partial_transfers() -> TestCase {
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
// Token Data Version Tests (26-32)
// ============================================================================

// Test 26: All V1 (Poseidon with pubkey hashing)
fn test26_all_v1_poseidon() -> TestCase {
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

// Test 27: All V2 (Poseidon with pubkey hashing)
fn test27_all_v2_poseidon() -> TestCase {
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

// Test 28: All V3/ShaFlat (SHA256)
fn test28_all_sha_flat() -> TestCase {
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

// Test 29: Mixed V1 and V2 in same transaction
fn test29_mixed_v1_v2() -> TestCase {
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

// Test 30: Mixed V1 and V3 in same transaction
fn test30_mixed_v1_sha_flat() -> TestCase {
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

// Test 31: Mixed V2 and V3 in same transaction
fn test31_mixed_v2_sha_flat() -> TestCase {
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

// Test 32: All three versions in same transaction
fn test32_all_three_versions() -> TestCase {
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
// Multi-Mint Operation Tests (33-38)
// ============================================================================

// Test 33: Single mint operations
fn test33_single_mint_operations() -> TestCase {
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

// Test 34: 2 different mints in same transaction
fn test34_two_different_mints() -> TestCase {
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

// Test 35: 3 different mints in same transaction
fn test35_three_different_mints() -> TestCase {
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

// Test 36: 4 different mints in same transaction
fn test36_four_different_mints() -> TestCase {
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

// Test 37: 5 different mints in same transaction (maximum)
fn test37_five_different_mints_maximum() -> TestCase {
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

// Test 38: Multiple operations per mint (2 transfers of mint A, 3 of mint B)
fn test38_multiple_operations_per_mint() -> TestCase {
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
// Compression Operations Tests (39-47)
// ============================================================================

// Test 39: Compress from SPL token only
fn test39_compress_from_spl_only() -> TestCase {
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
            pool_index: None,
        })],
    }
}

// Test 40: Compress from CToken only
fn test40_compress_from_ctoken_only() -> TestCase {
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
            pool_index: None,
        })],
    }
}

// Test 41: Decompress to CToken only
fn test41_decompress_to_ctoken_only() -> TestCase {
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
                pool_index: None,
            },
        )],
    }
}

// Test 42: Multiple compress operations only
fn test42_multiple_compress_operations() -> TestCase {
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
                pool_index: None,
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
                pool_index: None,
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
                pool_index: None,
            }),
        ],
    }
}

// Test 43: Multiple decompress operations only
fn test43_multiple_decompress_operations() -> TestCase {
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
                pool_index: None,
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
                pool_index: None,
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
                pool_index: None,
            }),
        ],
    }
}

// Test 44: Compress and decompress same amount (must balance)
fn test44_compress_decompress_balance() -> TestCase {
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
                pool_index: None,
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
                pool_index: None,
            }),
        ],
    }
}

// Test 45: Decompress to SPL token account
fn test45_decompress_to_spl() -> TestCase {
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
                pool_index: None,
            },
        )],
    }
}

// Test 46: Compress SPL with multiple compressed account inputs
fn test46_compress_spl_with_compressed_inputs() -> TestCase {
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
            pool_index: None,
        })],
    }
}

// Test 47: Mixed SPL and CToken operations
fn test47_mixed_spl_ctoken_operations() -> TestCase {
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
                pool_index: None,
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
                pool_index: None,
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
                pool_index: None,
            }),
        ],
    }
}

// ============================================================================
// Mixed Compression + Transfer Tests (48-54)
// ============================================================================

// Test 48: Transfer + compress SPL in same transaction
fn test48_transfer_compress_spl() -> TestCase {
    TestCase {
        name: "Transfer + compress SPL in same transaction".to_string(),
        actions: vec![
            // First: Regular compressed-to-compressed transfer (uses compressed mint 0)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500], // One account with 500 tokens
                amount: 300,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,  // Transfer to keypair[1]
                change_amount: None, // Keep 200 as change
                mint_index: 0,       // Compressed mint
            }),
            // Second: Compress from SPL token account (uses SPL mint 1)
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0, // No compressed inputs
                amount: 1000,                     // Amount to compress from SPL
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,    // Different signer (keypair[2])
                recipient_index: 2, // Compress to same owner
                mint_index: 1,      // SPL mint (different from transfer)
                use_spl: true,      // Use SPL token account
                pool_index: None,
            }),
        ],
    }
}

// Test 49: Transfer + decompress to SPL in same transaction
fn test49_transfer_decompress_spl() -> TestCase {
    TestCase {
        name: "Transfer + decompress to SPL in same transaction".to_string(),
        actions: vec![
            // First: Regular compressed-to-compressed transfer (uses compressed mint 0)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500], // One account with 500 tokens
                amount: 300,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,  // Transfer to keypair[1]
                change_amount: None, // Keep 200 as change
                mint_index: 0,       // Compressed mint
            }),
            // Second: Decompress to SPL token account (uses SPL mint 1)
            MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                num_input_compressed_accounts: 1, // One compressed account as input
                decompress_amount: 600,
                amount: 600,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,    // Different signer (keypair[2])
                recipient_index: 3, // Decompress to different recipient (keypair[3])
                mint_index: 1,      // SPL mint (different from transfer)
                to_spl: true,       // Decompress to SPL token account
                pool_index: None,
            }),
        ],
    }
}

// Test 50: Transfer + compress CToken in same transaction
fn test50_transfer_compress_ctoken() -> TestCase {
    TestCase {
        name: "Transfer + compress CToken in same transaction".to_string(),
        actions: vec![
            // First: Regular compressed-to-compressed transfer (uses compressed mint 0)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 300,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0, // Compressed mint
            }),
            // Second: Compress from CToken ATA (uses compressed mint 1)
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 1000,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                recipient_index: 2,
                mint_index: 1,  // Different compressed mint
                use_spl: false, // Use CToken ATA
                pool_index: None,
            }),
        ],
    }
}

// Test 51: Transfer + decompress to CToken in same transaction
fn test51_transfer_decompress_ctoken() -> TestCase {
    TestCase {
        name: "Transfer + decompress to CToken in same transaction".to_string(),
        actions: vec![
            // First: Regular compressed-to-compressed transfer (uses compressed mint 0)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 300,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0, // Compressed mint
            }),
            // Second: Decompress to CToken ATA (uses compressed mint 1)
            MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                num_input_compressed_accounts: 1,
                decompress_amount: 600,
                amount: 600,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                recipient_index: 3,
                mint_index: 1, // Different compressed mint
                to_spl: false, // Decompress to CToken ATA
                pool_index: None,
            }),
        ],
    }
}

// Test 52: Transfer + multiple compressions
fn test52_transfer_multiple_compressions() -> TestCase {
    TestCase {
        name: "Transfer + multiple compressions in same transaction".to_string(),
        actions: vec![
            // First: Regular compressed-to-compressed transfer (uses compressed mint 0)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 300,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0, // Compressed mint
            }),
            // Second: Compress from SPL (mint 1)
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 800,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                recipient_index: 2,
                mint_index: 1, // SPL mint
                use_spl: true,
                pool_index: None,
            }),
            // Third: Compress from CToken (mint 2)
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 600,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 3,
                recipient_index: 3,
                mint_index: 2, // Compressed mint
                use_spl: false,
                pool_index: None,
            }),
            // Fourth: Another compress from SPL (mint 1, different signer)
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 400,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 4,
                recipient_index: 4,
                mint_index: 1, // SPL mint (same as second action)
                use_spl: true,
                pool_index: None,
            }),
        ],
    }
}

// Test 53: Transfer + multiple decompressions
fn test53_transfer_multiple_decompressions() -> TestCase {
    TestCase {
        name: "Transfer + multiple decompressions in same transaction".to_string(),
        actions: vec![
            // First: Regular compressed-to-compressed transfer (uses compressed mint 0)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 300,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0, // Compressed mint
            }),
            // Second: Decompress to SPL (mint 1)
            MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                num_input_compressed_accounts: 1,
                decompress_amount: 400,
                amount: 400,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                recipient_index: 3,
                mint_index: 1, // SPL mint
                to_spl: true,
                pool_index: None,
            }),
            // Third: Decompress to CToken (mint 2)
            MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                num_input_compressed_accounts: 1,
                decompress_amount: 500,
                amount: 500,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 4,
                recipient_index: 5,
                mint_index: 2, // Compressed mint
                to_spl: false,
                pool_index: None,
            }),
            // Fourth: Another decompress to SPL (mint 1, different signer)
            MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                num_input_compressed_accounts: 1,
                decompress_amount: 300,
                amount: 300,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 6,
                recipient_index: 7,
                mint_index: 1, // SPL mint (same as second action)
                to_spl: true,
                pool_index: None,
            }),
        ],
    }
}

// Test 54: Transfer + compress + decompress (complex balanced operations)
fn test54_transfer_compress_decompress_balanced() -> TestCase {
    TestCase {
        name: "Transfer + compress + decompress (all must balance)".to_string(),
        actions: vec![
            // First: Regular compressed-to-compressed transfer (mint 0)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 300,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0, // Compressed mint
            }),
            // Second: Compress from SPL (mint 1)
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 800,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                recipient_index: 2,
                mint_index: 1, // SPL mint
                use_spl: true,
                pool_index: None,
            }),
            // Third: Decompress to SPL (mint 1, different signer)
            MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                num_input_compressed_accounts: 1,
                decompress_amount: 400,
                amount: 400,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 3,
                recipient_index: 4,
                mint_index: 1, // SPL mint (same as compress)
                to_spl: true,
                pool_index: None,
            }),
            // Fourth: Compress from CToken (mint 2)
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 600,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 5,
                recipient_index: 5,
                mint_index: 2, // Compressed mint
                use_spl: false,
                pool_index: None,
            }),
            // Fifth: Decompress to CToken (mint 2, different signer)
            MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                num_input_compressed_accounts: 1,
                decompress_amount: 500,
                amount: 500,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 6,
                recipient_index: 7,
                mint_index: 2, // Compressed mint (same as compress)
                to_spl: false,
                pool_index: None,
            }),
        ],
    }
}

// ============================================================================
// Token Pool Operation Tests (67-72)
// ============================================================================

// Test 67: Compress to pool index 0 (default pool)
fn test67_compress_to_pool_index_0() -> TestCase {
    TestCase {
        name: "Compress to pool index 0 (default pool)".to_string(),
        actions: vec![MetaTransfer2InstructionType::Compress(MetaCompressInput {
            num_input_compressed_accounts: 0, // No compressed inputs
            amount: 1000,                     // Amount to compress from SPL token account
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,    // Owner of the SPL token account
            recipient_index: 0, // Compress to same owner
            mint_index: 0,
            use_spl: true,       // Use SPL token account
            pool_index: Some(0), // Explicitly use pool 0
        })],
    }
}

// Test 68: Compress to pool index 1
fn test68_compress_to_pool_index_1() -> TestCase {
    TestCase {
        name: "Compress to pool index 1".to_string(),
        actions: vec![MetaTransfer2InstructionType::Compress(MetaCompressInput {
            num_input_compressed_accounts: 0,
            amount: 1500,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,
            recipient_index: 0,
            mint_index: 0,
            use_spl: true,       // SPL only - CToken doesn't use pools
            pool_index: Some(1), // Use pool 1 (will be created by test setup)
        })],
    }
}

// Test 69: Compress to pool index 4 (max valid index, max is 5 pools: 0-4)
fn test69_compress_to_pool_index_4() -> TestCase {
    TestCase {
        name: "Compress to pool index 4 (maximum)".to_string(),
        actions: vec![MetaTransfer2InstructionType::Compress(MetaCompressInput {
            num_input_compressed_accounts: 0,
            amount: 2000,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,
            recipient_index: 0,
            mint_index: 0,
            use_spl: true,
            pool_index: Some(4), // Maximum pool index (will be created by test setup)
        })],
    }
}

// Test 70: Decompress from pool index 0 (default pool)
fn test70_decompress_from_pool_index_0() -> TestCase {
    TestCase {
        name: "Decompress from pool index 0 (default pool)".to_string(),
        actions: vec![MetaTransfer2InstructionType::Decompress(
            MetaDecompressInput {
                num_input_compressed_accounts: 1, // One compressed account as input
                decompress_amount: 800,
                amount: 800,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,    // Owner of compressed tokens
                recipient_index: 1, // Decompress to different recipient
                mint_index: 0,
                to_spl: true,        // Decompress to SPL token account
                pool_index: Some(0), // Explicitly use pool 0
            },
        )],
    }
}

// Test 71: Decompress from different pool indices in same transaction
fn test71_decompress_from_different_pools() -> TestCase {
    TestCase {
        name: "Decompress from different pool indices".to_string(),
        actions: vec![
            // First compress to pool 0
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 500,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                recipient_index: 0,
                mint_index: 0,
                use_spl: true,
                pool_index: Some(0),
            }),
            // Compress to pool 1
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 600,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                recipient_index: 1,
                mint_index: 0,
                use_spl: true,
                pool_index: Some(1),
            }),
            // Compress to pool 2
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 400,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                recipient_index: 2,
                mint_index: 0,
                use_spl: true,
                pool_index: Some(2),
            }),
            // Now decompress from pool 0
            MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                num_input_compressed_accounts: 1,
                decompress_amount: 500,
                amount: 500,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                recipient_index: 1,
                mint_index: 0,
                to_spl: true,
                pool_index: Some(0),
            }),
            // Decompress from pool 1
            MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                num_input_compressed_accounts: 1,
                decompress_amount: 600,
                amount: 600,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                recipient_index: 2,
                mint_index: 0,
                to_spl: true,
                pool_index: Some(1),
            }),
            // Decompress from pool 2
            MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                num_input_compressed_accounts: 1,
                decompress_amount: 400,
                amount: 400,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                recipient_index: 3,
                mint_index: 0,
                to_spl: true,
                pool_index: Some(2),
            }),
        ],
    }
}

// Test 72: Multiple pools for same mint in transaction
fn test72_multiple_pools_same_mint() -> TestCase {
    TestCase {
        name: "Multiple pools for same mint in transaction".to_string(),
        actions: vec![
            // Compress to pool 0
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 1000,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                recipient_index: 0,
                mint_index: 0,
                use_spl: true,
                pool_index: Some(0),
            }),
            // Compress to pool 1 (same mint)
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 1500,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                recipient_index: 1,
                mint_index: 0, // Same mint as above
                use_spl: true,
                pool_index: Some(1),
            }),
            // Decompress from pool 0
            MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                num_input_compressed_accounts: 1,
                decompress_amount: 700,
                amount: 700,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                recipient_index: 3,
                mint_index: 0, // Same mint
                to_spl: true,
                pool_index: Some(0),
            }),
        ],
    }
}

// ============================================================================
// CompressAndClose Operation Tests (55-60)
// ============================================================================

// Test 55: CompressAndClose as owner (not compressible)
fn test55_compress_and_close_as_owner() -> TestCase {
    TestCase {
        name: "CompressAndClose as owner (no validation needed)".to_string(),
        actions: vec![MetaTransfer2InstructionType::CompressAndClose(
            MetaCompressAndCloseInput {
                token_data_version: TokenDataVersion::ShaFlat, // Must be ShaFlat for security
                signer_index: 0,         // Owner who signs and owns the CToken ATA
                destination_index: None, // No destination = authority receives rent
                mint_index: 0,           // Use first mint
                is_compressible: false,  // Regular CToken ATA, no extensions
            },
        )],
    }
}
// Test 55: CompressAndClose as owner (compressible)
fn test55_compress_and_close_as_owner_compressible() -> TestCase {
    TestCase {
        name: "CompressAndClose as owner (no validation needed)".to_string(),
        actions: vec![MetaTransfer2InstructionType::CompressAndClose(
            MetaCompressAndCloseInput {
                token_data_version: TokenDataVersion::ShaFlat, // Must be ShaFlat for security
                signer_index: 0,         // Owner who signs and owns the CToken ATA
                destination_index: None, // No destination = authority receives rent
                mint_index: 0,           // Use first mint
                is_compressible: true,   // Regular CToken ATA, no extensions
            },
        )],
    }
}

// Test 56: CompressAndClose with destination
fn test56_compress_and_close_with_destination() -> TestCase {
    TestCase {
        name: "CompressAndClose with destination (rent to specific recipient)".to_string(),
        actions: vec![MetaTransfer2InstructionType::CompressAndClose(
            MetaCompressAndCloseInput {
                token_data_version: TokenDataVersion::ShaFlat, // Must be ShaFlat for security
                signer_index: 0,            // Owner who signs and owns the CToken ATA
                destination_index: Some(1), // Send rent lamports to keypair[1]
                mint_index: 0,              // Use first mint
                is_compressible: true,      // Compressible account with extensions
            },
        )],
    }
}

// Test 57: Multiple CompressAndClose in single transaction
fn test57_multiple_compress_and_close() -> TestCase {
    TestCase {
        name: "Multiple CompressAndClose in single transaction".to_string(),
        actions: vec![
            // Close first account from signer 0
            MetaTransfer2InstructionType::CompressAndClose(MetaCompressAndCloseInput {
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,         // First owner
                destination_index: None, // Rent back to authority
                mint_index: 0,
                is_compressible: true,
            }),
            // Close second account from signer 1
            MetaTransfer2InstructionType::CompressAndClose(MetaCompressAndCloseInput {
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,         // Second owner
                destination_index: None, // Rent back to authority
                mint_index: 0,
                is_compressible: true,
            }),
            // Close third account from signer 2
            MetaTransfer2InstructionType::CompressAndClose(MetaCompressAndCloseInput {
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,         // Third owner
                destination_index: None, // Rent back to authority
                mint_index: 0,
                is_compressible: true,
            }),
        ],
    }
}

// Test 58: CompressAndClose + regular transfer in same transaction
fn test58_compress_and_close_with_transfer() -> TestCase {
    TestCase {
        name: "CompressAndClose + regular transfer in same transaction".to_string(),
        actions: vec![
            // First: Close CToken account from signer 0
            MetaTransfer2InstructionType::CompressAndClose(MetaCompressAndCloseInput {
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,         // Owner who closes
                destination_index: None, // Rent back to authority
                mint_index: 0,
                is_compressible: true,
            }),
            // Second: Regular compressed transfer from signer 1 to signer 2
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500], // One account with 500 tokens
                amount: 300,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1, // Different signer than CompressAndClose
                delegate_index: None,
                recipient_index: 2,  // Transfer to keypair[2]
                change_amount: None, // Keep 200 as change
                mint_index: 0,
            }),
        ],
    }
}

// Test 59: CompressAndClose with full balance
fn test59_compress_and_close_full_balance() -> TestCase {
    TestCase {
        name: "CompressAndClose with full balance (compress all tokens before closing)".to_string(),
        actions: vec![MetaTransfer2InstructionType::CompressAndClose(
            MetaCompressAndCloseInput {
                token_data_version: TokenDataVersion::ShaFlat, // Must be ShaFlat for security
                signer_index: 0,         // Owner who signs and owns the CToken ATA
                destination_index: None, // Rent back to authority
                mint_index: 0,           // Use first mint
                is_compressible: true,   // Compressible account with extensions
            },
        )],
    }
}

// Test 60: CompressAndClose creating specific output (rent authority case)
fn test60_compress_and_close_specific_output() -> TestCase {
    TestCase {
        name: "CompressAndClose creating specific output (rent authority case)".to_string(),
        actions: vec![MetaTransfer2InstructionType::CompressAndClose(
            MetaCompressAndCloseInput {
                token_data_version: TokenDataVersion::ShaFlat, // Must be ShaFlat for security
                signer_index: 0,            // Owner who signs and owns the CToken ATA
                destination_index: Some(2), // Send rent lamports to specific recipient (keypair[2])
                mint_index: 0,              // Use first mint
                is_compressible: true,      // Compressible account with extensions
            },
        )],
    }
}

// ============================================================================
// Delegate Operation Tests (61-62)
// ============================================================================

// Test 61: Approve creating delegated account + change
fn test61_approve_with_change() -> TestCase {
    TestCase {
        name: "Approve creating delegated account + change".to_string(),
        actions: vec![
            // Approve delegate for partial amount, creating a delegated account and a change account
            MetaTransfer2InstructionType::Approve(MetaApproveInput {
                num_input_compressed_accounts: 1,
                delegate_amount: 200, // Approve only 200 out of 500 tokens
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,   // Owner (keypair[0]) approves
                delegate_index: 1, // Delegate is keypair[1]
                mint_index: 0,
                setup: false, // Execute in main test (not setup)
            }),
        ],
    }
}

// Test 62: Transfer using delegate authority with single input
fn test62_delegate_transfer_single_input() -> TestCase {
    TestCase {
        name: "Transfer using delegate authority (single input)".to_string(),
        actions: vec![
            // First, approve delegate to transfer tokens (executed in setup)
            MetaTransfer2InstructionType::Approve(MetaApproveInput {
                num_input_compressed_accounts: 1,
                delegate_amount: 300,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,   // Owner (keypair[0]) approves
                delegate_index: 1, // Delegate is keypair[1]
                mint_index: 0,
                setup: true, // Execute in setup phase
            }),
            // Transfer using delegate authority
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300], // One delegated account with 300 tokens
                amount: 300,
                is_delegate_transfer: true, // This is a delegate transfer
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,         // Owner index (for fetching accounts)
                delegate_index: Some(1), // Delegate (keypair[1]) signs the transfer
                recipient_index: 2,      // Transfer to keypair[2]
                change_amount: Some(0),  // Transfer full amount, no change
                mint_index: 0,
            }),
        ],
    }
}

// Test 63: Transfer using delegate authority (partial amount)
fn test63_delegate_transfer_partial_amount() -> TestCase {
    TestCase {
        name: "Transfer using delegate authority (partial amount)".to_string(),
        actions: vec![
            // First, approve delegate to transfer tokens (executed in setup)
            MetaTransfer2InstructionType::Approve(MetaApproveInput {
                num_input_compressed_accounts: 1,
                delegate_amount: 400, // Delegate can transfer up to 400 tokens
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,   // Owner (keypair[0]) approves
                delegate_index: 1, // Delegate is keypair[1]
                mint_index: 0,
                setup: true, // Execute in setup phase
            }),
            // Transfer partial amount using delegate authority
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![400], // One delegated account with 400 tokens
                amount: 250, // Transfer only 250 out of 400 delegated tokens
                is_delegate_transfer: true, // This is a delegate transfer
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,         // Owner index (for fetching accounts)
                delegate_index: Some(1), // Delegate (keypair[1]) signs the transfer
                recipient_index: 2,      // Transfer to keypair[2]
                change_amount: None, // Creates change account with remaining 150 delegated tokens
                mint_index: 0,
            }),
        ],
    }
}

// Test 64: Revoke delegation (merges all accounts)
fn test64_revoke_delegation() -> TestCase {
    TestCase {
        name: "Revoke delegation (merges all accounts)".to_string(),
        actions: vec![
            // First, approve delegate to transfer tokens (executed in setup)
            MetaTransfer2InstructionType::Approve(MetaApproveInput {
                num_input_compressed_accounts: 1,
                delegate_amount: 300, // Delegate can transfer up to 300 tokens
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,   // Owner (keypair[0]) approves
                delegate_index: 1, // Delegate is keypair[1]
                mint_index: 0,
                setup: true, // Execute in setup phase
            }),
            // Revoke delegation by doing a regular transfer to self (merges delegated account back to undelegated)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300], // One delegated account with 300 tokens
                amount: 300,
                is_delegate_transfer: false, // Regular transfer (NOT delegate transfer) - this revokes delegation
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,        // Owner (keypair[0]) signs
                delegate_index: None,   // No delegate for this transfer
                recipient_index: 0,     // Transfer to self (same owner)
                change_amount: Some(0), // Full amount transfer, no change
                mint_index: 0,
            }),
        ],
    }
}

// Test 65: Multiple delegates in same transaction
fn test65_multiple_delegates() -> TestCase {
    TestCase {
        name: "Multiple delegates in same transaction".to_string(),
        actions: vec![
            // Approve first delegate (keypair[1]) for 200 tokens
            MetaTransfer2InstructionType::Approve(MetaApproveInput {
                num_input_compressed_accounts: 1,
                delegate_amount: 200,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,   // Owner (keypair[0]) approves
                delegate_index: 1, // Delegate is keypair[1]
                mint_index: 0,
                setup: false, // Execute in main test
            }),
            // Approve second delegate (keypair[2]) for 150 tokens from a different account
            MetaTransfer2InstructionType::Approve(MetaApproveInput {
                num_input_compressed_accounts: 1,
                delegate_amount: 150,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 3,   // Different owner (keypair[3]) approves
                delegate_index: 2, // Delegate is keypair[2]
                mint_index: 0,
                setup: false, // Execute in main test
            }),
            // Approve third delegate (keypair[4]) for 100 tokens from another account
            MetaTransfer2InstructionType::Approve(MetaApproveInput {
                num_input_compressed_accounts: 1,
                delegate_amount: 100,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 5,   // Another owner (keypair[5]) approves
                delegate_index: 4, // Delegate is keypair[4]
                mint_index: 0,
                setup: false, // Execute in main test
            }),
        ],
    }
}

// Test 66: Delegate transfer with change account
fn test66_delegate_transfer_with_change() -> TestCase {
    TestCase {
        name: "Delegate transfer with change account".to_string(),
        actions: vec![
            // Approve delegate for 500 tokens (executed in setup)
            MetaTransfer2InstructionType::Approve(MetaApproveInput {
                num_input_compressed_accounts: 1,
                delegate_amount: 500,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,   // Owner (keypair[0]) approves
                delegate_index: 1, // Delegate is keypair[1]
                mint_index: 0,
                setup: true, // Execute in setup phase
            }),
            // Delegate transfers partial amount, creating delegated change account
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500], // One delegated account with 500 tokens
                amount: 200, // Transfer 200, leaving 300 as delegated change
                is_delegate_transfer: true, // This is a delegate transfer
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,         // Owner index (for fetching accounts)
                delegate_index: Some(1), // Delegate (keypair[1]) signs the transfer
                recipient_index: 2,      // Transfer to keypair[2]
                change_amount: None, // Creates change account with 300 tokens (still delegated to keypair[1])
                mint_index: 0,
            }),
        ],
    }
}

#![cfg(feature = "test-sbf")]

use anchor_lang::{InstructionData, ToAccountMetas};
use light_program_test::{program_test::LightProgramTest, Indexer, ProgramTestConfig, Rpc};
use light_sdk::instruction::{PackedAccounts, SystemAccountMetaConfig};
use serial_test::serial;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use system_cpi_test::{self, ID};

/// Test all three modes of cpi_context_indexing
/// Mode 0: None data
/// Mode 1: Default CompressedAccountData
/// Mode 2: Custom CompressedAccountData with specific values
#[serial]
#[tokio::test]
async fn test_cpi_context_indexing_mode_0() {
    test_cpi_context_indexing_with_mode(0).await;
}

#[serial]
#[tokio::test]
async fn test_cpi_context_indexing_mode_1() {
    test_cpi_context_indexing_with_mode(1).await;
}

#[serial]
#[tokio::test]
async fn test_cpi_context_indexing_mode_2() {
    test_cpi_context_indexing_with_mode(2).await;
}

async fn test_cpi_context_indexing_with_mode(mode: u8) {
    // Generate all permutations of [0, 1, 2]
    let permutations = vec![
        [0u8, 1, 2],
        [0u8, 2, 1],
        [1u8, 0, 2],
        [1u8, 2, 0],
        [2u8, 0, 1],
        [2u8, 1, 0],
    ];

    for leaf_indices in permutations {
        println!("\n============================================");
        println!(
            "Testing mode {} with leaf indices: {:?}",
            mode, leaf_indices
        );
        println!("============================================\n");

        // Setup test environment
        let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
            true,
            Some(vec![("system_cpi_test", ID)]),
        ))
        .await
        .expect("Failed to setup test programs with accounts");

        let payer = rpc.get_payer().insecure_clone();

        // Get initial compressed accounts count for the program
        let initial_accounts = if let Some(ref indexer) = rpc.indexer {
            indexer
                .get_compressed_accounts_by_owner(&ID, None, None)
                .await
                .unwrap()
                .value
                .items
        } else {
            vec![]
        };
        let initial_count = initial_accounts.len();
        println!("Initial accounts owned by program: {}", initial_count);

        // Build and execute the cpi_context_indexing instruction
        let instruction = build_cpi_context_indexing_instruction(&mut rpc, &payer, mode).await;

        let signature = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await
            .expect("Failed to execute cpi_context_indexing instruction");

        println!("Transaction signature: {}", signature);

        // Get compressed accounts created by the program
        let accounts = rpc
            .get_compressed_accounts_by_owner(&ID, None, None)
            .await
            .expect("Failed to get compressed accounts by owner")
            .value
            .items;

        let default_pubkey_accounts = rpc
            .get_compressed_accounts_by_owner(&Pubkey::default(), None, None)
            .await
            .expect("Failed to get compressed accounts by owner")
            .value
            .items;

        let program_owned_count = accounts.len() - initial_count;
        let default_owned_count = default_pubkey_accounts.len();

        println!("Accounts owned by program ID: {}", program_owned_count);
        println!("Accounts owned by default pubkey: {}", default_owned_count);

        // The function creates 4 accounts total
        assert_eq!(
            program_owned_count + default_owned_count,
            4,
            "Expected 4 total accounts for mode {}, but got {} program-owned + {} default-owned",
            mode,
            program_owned_count,
            default_owned_count
        );

        // Verify mode-specific account counts
        match mode {
            0 => {
                // Mode 0: 2 accounts with None data (default owner), 2 with data (program owner)
                assert_eq!(
                    program_owned_count, 2,
                    "Mode 0 should have exactly 2 program-owned accounts, got {}",
                    program_owned_count
                );
                assert_eq!(
                    default_owned_count, 2,
                    "Mode 0 should have exactly 2 default-owned accounts, got {}",
                    default_owned_count
                );
            }
            1 | 2 => {
                // Mode 1 & 2: All accounts have data and are program-owned
                assert_eq!(
                    program_owned_count, 4,
                    "Mode {} should have exactly 4 program-owned accounts, got {}",
                    mode, program_owned_count
                );
                assert_eq!(
                    default_owned_count, 0,
                    "Mode {} should have exactly 0 default-owned accounts, got {}",
                    mode, default_owned_count
                );
            }
            _ => panic!("Invalid mode {}", mode),
        }

        // Combine and verify all accounts based on mode
        verify_accounts_for_mode(&accounts[initial_count..], &default_pubkey_accounts, mode);

        // Now call the inputs instruction to consume the accounts we just created
        println!(
            "\nCalling inputs instruction with leaf indices {:?} to consume created accounts...",
            leaf_indices
        );
        let inputs_instruction =
            build_cpi_context_indexing_inputs_instruction(&mut rpc, &payer, mode, leaf_indices)
                .await;

        let inputs_signature = rpc
            .create_and_send_transaction(&[inputs_instruction], &payer.pubkey(), &[&payer])
            .await
            .expect("Failed to execute cpi_context_indexing_inputs instruction");

        println!("Inputs transaction signature: {}", inputs_signature);

        // Verify accounts after consumption based on mode
        let accounts_after_inputs = rpc
            .get_compressed_accounts_by_owner(&ID, None, None)
            .await
            .expect("Failed to get compressed accounts by owner")
            .value
            .items;

        let default_accounts_after_inputs = rpc
            .get_compressed_accounts_by_owner(&Pubkey::default(), None, None)
            .await
            .expect("Failed to get compressed accounts by owner")
            .value
            .items;

        match mode {
            0 => {
                // Mode 0: We can only consume program-owned accounts (2 accounts)
                // The 2 default-owned accounts should remain
                assert_eq!(
                    accounts_after_inputs.len(),
                    initial_count,
                    "Program-owned accounts should be consumed in mode 0"
                );
                assert_eq!(
                    default_accounts_after_inputs.len(),
                    2,
                    "Mode 0 should still have 2 default-owned accounts (they cannot be consumed)"
                );
                println!("✅ Mode 0 with leaf indices {:?}: Program-owned accounts consumed, default-owned accounts remain", leaf_indices);
            }
            1 | 2 => {
                // Modes 1-2: All 4 accounts are program-owned and should be consumed
                assert_eq!(
                    accounts_after_inputs.len(),
                    initial_count,
                    "All program-owned accounts should be consumed"
                );
                assert_eq!(
                    default_accounts_after_inputs.len(),
                    0,
                    "No default-owned accounts should exist in modes 1-2"
                );
                println!(
                    "✅ Mode {} with leaf indices {:?}: All accounts successfully consumed",
                    mode, leaf_indices
                );
            }
            _ => panic!("Invalid mode"),
        };
    }
}

async fn build_cpi_context_indexing_inputs_instruction(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    mode: u8,
    leaf_indices: [u8; 3],
) -> Instruction {
    // Create remaining accounts for CPI context (same as outputs)
    let mut remaining_accounts = PackedAccounts::default();
    let tree_info = rpc.test_accounts.v2_state_trees[0];
    let mut config = SystemAccountMetaConfig::new(ID);
    config.cpi_context = Some(tree_info.cpi_context);
    remaining_accounts
        .add_system_accounts(config)
        .expect("Failed to add system accounts");
    remaining_accounts.insert_or_get(tree_info.merkle_tree);

    remaining_accounts.insert_or_get(tree_info.output_queue);

    // Build instruction data for inputs
    let instruction_data =
        system_cpi_test::instruction::CpiContextIndexingInputs { mode, leaf_indices };

    let accounts = system_cpi_test::accounts::GenericAnchorAccounts {
        signer: payer.pubkey(),
    };

    let (remaining_accounts_metas, _, _) = remaining_accounts.to_account_metas();

    Instruction {
        program_id: ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            remaining_accounts_metas,
        ]
        .concat(),
        data: instruction_data.data(),
    }
}

async fn build_cpi_context_indexing_instruction(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    mode: u8,
) -> Instruction {
    // Create remaining accounts for CPI context
    let mut remaining_accounts = PackedAccounts::default();
    // Get tree info for output and pack it
    let tree_info = rpc.test_accounts.v2_state_trees[0];
    // Configure for CPI context mode
    let mut config = SystemAccountMetaConfig::new(ID);
    config.cpi_context = Some(tree_info.cpi_context);
    remaining_accounts
        .add_system_accounts(config)
        .expect("Failed to add system accounts");

    remaining_accounts.insert_or_get(tree_info.output_queue);
    // Build instruction data
    let instruction_data = system_cpi_test::instruction::CpiContextIndexing { mode };

    // Create accounts meta
    let accounts = system_cpi_test::accounts::GenericAnchorAccounts {
        signer: payer.pubkey(),
    };

    let (remaining_accounts_metas, _, _) = remaining_accounts.to_account_metas();

    Instruction {
        program_id: ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            remaining_accounts_metas,
        ]
        .concat(),
        data: instruction_data.data(),
    }
}

fn verify_accounts_for_mode(
    program_owned_accounts: &[light_client::indexer::CompressedAccount],
    default_owned_accounts: &[light_client::indexer::CompressedAccount],
    mode: u8,
) {
    // The function creates 4 accounts total via different CPI methods:
    // 1. OutputCompressedAccountWithPackedContext - uses mode-based data/owner
    // 2. CompressedAccountInfo - always creates data (owner is invoking_program_id)
    // 3. OutputCompressedAccountWithPackedContext - uses mode-based data/owner
    // 4. OutputCompressedAccountWithPackedContext - always [9u8; 8] discriminator with program owner
    //
    // Mode 0: 2 None data (default owner), 2 with data (program owner)
    // Mode 1-2: All 4 have data and are program-owned

    println!(
        "Mode {}: {} accounts owned by program, {} owned by default",
        mode,
        program_owned_accounts.len(),
        default_owned_accounts.len()
    );

    // All accounts owned by Pubkey::default() must have None data
    for (i, account) in default_owned_accounts.iter().enumerate() {
        assert_eq!(
            account.owner,
            Pubkey::default(),
            "Account {} should be owned by default pubkey",
            i
        );
        assert!(
            account.data.is_none(),
            "Account {} owned by default pubkey must have None data",
            i
        );
        assert_eq!(account.lamports, 0, "Account {} should have 0 lamports", i);
        let manual_hash =
            light_compressed_account::compressed_account::CompressedAccountWithMerkleContext::from(
                account.clone(),
            )
            .hash()
            .unwrap();
        assert_eq!(manual_hash, account.hash);
    }

    // All program-owned accounts must have data
    for (i, account) in program_owned_accounts.iter().enumerate() {
        println!("Verifying program-owned account {} for mode {}", i, mode);

        assert_eq!(
            account.owner, ID,
            "Account {} should be owned by program ID",
            i
        );
        assert_eq!(account.lamports, 0, "Account {} should have 0 lamports", i);

        // All program-owned accounts must have data
        assert!(
            account.data.is_some(),
            "Account {} owned by program must have data",
            i
        );
        let manual_hash =
            light_compressed_account::compressed_account::CompressedAccountWithMerkleContext::from(
                account.clone(),
            )
            .hash()
            .unwrap();
        assert_eq!(manual_hash, account.hash);

        if let Some(ref data) = account.data {
            // Check if this is the special account with [9u8; 8] discriminator
            // This account always has the same data regardless of mode
            if data.discriminator == [9u8; 8] {
                assert_eq!(
                    data.data,
                    Vec::<u8>::new(),
                    "Special account should have empty data"
                );
                assert_eq!(
                    data.data_hash, [9u8; 32],
                    "Special account should have [9u8; 32] data_hash"
                );
            } else {
                // Other accounts follow the mode-based pattern
                // Note: In Mode 0, the second account (from CompressedAccountInfo) always has data
                match mode {
                    0 | 1 => {
                        // Mode 0 & 1: Program-owned accounts should have default data
                        assert_eq!(
                            data.discriminator, [0u8; 8],
                            "Mode 0 or 1 account {} should have default discriminator",
                            i
                        );
                        assert_eq!(
                            data.data,
                            Vec::<u8>::new(),
                            "Mode 0 or 1 account {} should have empty data",
                            i
                        );
                        assert_eq!(
                            data.data_hash, [0u8; 32],
                            "Mode 0 or 1 account {} should have default data_hash",
                            i
                        );
                    }

                    2 => {
                        // Mode 2: Custom CompressedAccountData
                        assert_eq!(
                            data.discriminator, [1u8; 8],
                            "Mode 2 account {} should have [1u8; 8] discriminator",
                            i
                        );
                        assert_eq!(
                            data.data,
                            vec![2u8; 32],
                            "Mode 2 account {} should have vec![2u8; 32] data",
                            i
                        );
                        assert_eq!(
                            data.data_hash, [3u8; 32],
                            "Mode 2 account {} should have [3u8; 32] data_hash",
                            i
                        );
                    }
                    _ => panic!("Invalid mode {}", mode),
                }
            }
        } else {
            panic!("all accounts must have data.")
        }
    }
}

#![cfg(feature = "test-sbf")]

use anchor_lang::Discriminator;
use light_account_checks::account_info::test_account_info::pinocchio::get_account_info;
use light_batched_merkle_tree::constants::DEFAULT_CPI_CONTEXT_ACCOUNT_SIZE_V2;
const DEFAULT_CPI_CONTEXT_ACCOUNT_SIZE_V1: u64 = 20 * 1024 + 8;

use light_program_test::{
    program_test::{LightProgramTest, TestRpc},
    ProgramTestConfig,
};
use light_sdk::constants::{
    CPI_CONTEXT_ACCOUNT_1_DISCRIMINATOR, CPI_CONTEXT_ACCOUNT_2_DISCRIMINATOR,
};
use light_system_program_pinocchio::cpi_context::state::deserialize_cpi_context_account;
use light_test_utils::{legacy_cpi_context_account::get_legacy_cpi_context_account, Rpc};
use pinocchio::pubkey::Pubkey as PinocchioPubkey;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Signer,
    transaction::Transaction,
};

fn create_reinit_cpi_context_instruction(cpi_context_account: Pubkey) -> Instruction {
    Instruction {
        program_id: light_system_program::ID,
        accounts: vec![AccountMeta::new(cpi_context_account, false)],
        data: light_system_program::instruction::ReInitCpiContextAccount::DISCRIMINATOR.to_vec(),
    }
}

/// Test the reinitialization of legacy CPI context accounts
/// This test verifies:
/// 1. Successfully reinitialize a legacy CPI context account
/// 2. Check that discriminator is updated from legacy to new
/// 3. Verify that account data is properly zeroed out except for merkle tree
/// 4. Ensure reinit fails on already-migrated accounts
/// 5. Validate proper error handling for invalid accounts
#[tokio::test]
async fn test_re_init_cpi_account() {
    let config = ProgramTestConfig {
        skip_protocol_init: true,
        with_prover: false,
        ..Default::default()
    };
    let mut context = LightProgramTest::new(config).await.unwrap();

    // Generate a test CPI context account address
    let cpi_context_account = Pubkey::new_unique();

    // Test Case 1: Successfully reinitialize a legacy CPI context account
    {
        println!("Test Case 1: Successfully reinitialize legacy CPI context account");

        // Set up legacy CPI context account
        let legacy_account = get_legacy_cpi_context_account();
        context.set_account(cpi_context_account, legacy_account.clone());

        // Verify legacy discriminator is present
        let pre_account = context
            .get_account(cpi_context_account)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            &pre_account.data[0..8],
            &CPI_CONTEXT_ACCOUNT_1_DISCRIMINATOR,
            "Account should have legacy discriminator"
        );
        assert_eq!(
            pre_account.data.len(),
            DEFAULT_CPI_CONTEXT_ACCOUNT_SIZE_V1 as usize,
            "Legacy account should have V1 size"
        );

        // Create reinit instruction
        let instruction = create_reinit_cpi_context_instruction(cpi_context_account);

        let payer = context.get_payer().insecure_clone();
        let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
        let recent_blockhash = context.get_latest_blockhash().await.unwrap().0;
        transaction.sign(&[&payer], recent_blockhash);

        context.process_transaction(transaction).await.unwrap();

        // Verify account has been reinitialized with new discriminator
        let post_account = context
            .get_account(cpi_context_account)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            &post_account.data[0..8],
            &CPI_CONTEXT_ACCOUNT_2_DISCRIMINATOR,
            "Account should have new discriminator after reinit"
        );
        assert_eq!(
            post_account.data.len(),
            DEFAULT_CPI_CONTEXT_ACCOUNT_SIZE_V2 as usize,
            "Account should be resized to V2 size after reinit"
        );

        // Verify merkle tree is preserved
        // Legacy layout: discriminator (8) + fee_payer (32) + merkle_tree (32)
        // New layout: discriminator (8) + fee_payer (32) + merkle_tree (32)
        let legacy_merkle_tree = &legacy_account.data[40..72];
        let new_merkle_tree = &post_account.data[40..72];
        assert_eq!(
            legacy_merkle_tree, new_merkle_tree,
            "Associated merkle tree should be preserved"
        );

        // Verify fee payer is zeroed
        assert_eq!(
            &post_account.data[8..40],
            &[0u8; 32],
            "Fee payer should be zeroed"
        );

        // Create an AccountInfo using the test helper to deserialize the account
        let account_info = get_account_info(
            PinocchioPubkey::from(cpi_context_account.to_bytes()),
            PinocchioPubkey::from(light_system_program::ID.to_bytes()),
            false, // is_signer
            true,  // is_writable
            false, // is_executable
            post_account.data.clone(),
        );

        // Deserialize the account to verify vector capacities
        let deserialized = deserialize_cpi_context_account(&account_info).unwrap();
        assert_eq!(deserialized.remaining_capacity(), 6500);
        // Verify vector capacities match CpiContextAccountInitParams defaults
        assert_eq!(
            deserialized.new_addresses.capacity(),
            10,
            "new_addresses capacity should be 10"
        );
        assert_eq!(
            deserialized.new_addresses.len(),
            0,
            "new_addresses should be empty"
        );

        assert_eq!(
            deserialized.readonly_addresses.capacity(),
            10,
            "readonly_addresses capacity should be 10"
        );
        assert_eq!(
            deserialized.readonly_addresses.len(),
            0,
            "readonly_addresses should be empty"
        );

        assert_eq!(
            deserialized.readonly_accounts.capacity(),
            10,
            "readonly_accounts capacity should be 10"
        );
        assert_eq!(
            deserialized.readonly_accounts.len(),
            0,
            "readonly_accounts should be empty"
        );

        assert_eq!(
            deserialized.in_accounts.capacity(),
            20,
            "in_accounts capacity should be 20"
        );
        assert_eq!(
            deserialized.in_accounts.len(),
            0,
            "in_accounts should be empty"
        );

        assert_eq!(
            deserialized.out_accounts.capacity(),
            30,
            "out_accounts capacity should be 30"
        );
        assert_eq!(
            deserialized.out_accounts.len(),
            0,
            "out_accounts should be empty"
        );
    }

    // Test Case 2: Reinit should fail on already-migrated account
    {
        println!("Test Case 2: Reinit should fail on already-migrated account");

        // The account from Test Case 1 is already migrated
        let instruction = create_reinit_cpi_context_instruction(cpi_context_account);

        let payer = context.get_payer().insecure_clone();
        let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
        let recent_blockhash = context.get_latest_blockhash().await.unwrap().0;
        transaction.sign(&[&payer], recent_blockhash);

        let result = context.process_transaction(transaction).await;
        assert!(
            result.is_err(),
            "Reinit should fail on already-migrated account"
        );
    }

    // Test Case 3: Reinit should fail with wrong account owner
    {
        println!("Test Case 3: Reinit should fail with wrong account owner");

        // Create new context for clean state
        let config = ProgramTestConfig {
            skip_protocol_init: false,
            with_prover: false,
            ..Default::default()
        };
        let mut context = LightProgramTest::new(config).await.unwrap();

        let wrong_owner_account = Pubkey::new_unique();
        let mut legacy_account = get_legacy_cpi_context_account();
        legacy_account.owner = Pubkey::new_unique(); // Set wrong owner

        context.set_account(wrong_owner_account, legacy_account);

        let instruction = create_reinit_cpi_context_instruction(wrong_owner_account);

        let payer = context.get_payer().insecure_clone();
        let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
        let recent_blockhash = context.get_latest_blockhash().await.unwrap().0;
        transaction.sign(&[&payer], recent_blockhash);

        let result = context.process_transaction(transaction).await;
        assert!(
            result.is_err(),
            "Reinit should fail with wrong account owner"
        );
    }

    // Test Case 4: Reinit should fail with invalid discriminator
    {
        println!("Test Case 4: Reinit should fail with invalid discriminator");

        // Create new context for clean state
        let config = ProgramTestConfig {
            skip_protocol_init: false,
            with_prover: false,
            ..Default::default()
        };
        let mut context = LightProgramTest::new(config).await.unwrap();

        let invalid_discriminator_account = Pubkey::new_unique();
        let mut legacy_account = get_legacy_cpi_context_account();
        // Set invalid discriminator (not legacy)
        legacy_account.data[0..8].copy_from_slice(&[1u8; 8]);

        context.set_account(invalid_discriminator_account, legacy_account);

        let instruction = create_reinit_cpi_context_instruction(invalid_discriminator_account);

        let payer = context.get_payer().insecure_clone();
        let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
        let recent_blockhash = context.get_latest_blockhash().await.unwrap().0;
        transaction.sign(&[&payer], recent_blockhash);

        let result = context.process_transaction(transaction).await;
        assert!(
            result.is_err(),
            "Reinit should fail with invalid discriminator"
        );
    }
}

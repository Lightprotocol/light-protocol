use anchor_spl::token_2022::spl_token_2022;
use solana_sdk::{program_pack::Pack, signature::Keypair, signer::Signer};

use super::shared::*;

/// Test SPL token instruction compatibility with ctoken program
///
/// This test creates SPL token instructions using the official spl_token library,
/// then changes the program_id to the ctoken program to verify instruction format compatibility.
#[tokio::test]
#[allow(deprecated)] // We're testing SPL compatibility with the basic transfer instruction
async fn test_spl_instruction_compatibility() {
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();

    // Create two token accounts for testing
    let account1_keypair = Keypair::new();
    let account2_keypair = Keypair::new();

    println!("Creating first token account...");

    // Create first account using SPL token instruction format
    {
        // Step 1: Create account via system program with ctoken program as owner
        let rent = context
            .rpc
            .get_minimum_balance_for_rent_exemption(165)
            .await
            .unwrap();

        let create_account_ix = solana_sdk::system_instruction::create_account(
            &payer_pubkey,
            &account1_keypair.pubkey(),
            rent,
            165,
            &light_compressed_token::ID, // Use ctoken program as owner
        );

        context
            .rpc
            .create_and_send_transaction(
                &[create_account_ix],
                &payer_pubkey,
                &[&context.payer, &account1_keypair],
            )
            .await
            .unwrap();

        // Step 2: Initialize using SPL token initialize_account3 instruction
        // Note: initialize_account3 doesn't require account to be signer (SPL compatibility)
        let mut init_ix = spl_token_2022::instruction::initialize_account3(
            &spl_token_2022::ID,
            &account1_keypair.pubkey(),
            &context.mint_pubkey,
            &context.owner_keypair.pubkey(),
        )
        .unwrap();

        // Change program_id to ctoken program for compatibility test
        init_ix.program_id = light_compressed_token::ID;

        context
            .rpc
            .create_and_send_transaction(&[init_ix], &payer_pubkey, &[&context.payer])
            .await
            .unwrap();

        println!("First token account created successfully");
    }

    println!("Creating second token account...");

    // Create second account using SPL token instruction format
    {
        // Step 1: Create account via system program with ctoken program as owner
        let rent = context
            .rpc
            .get_minimum_balance_for_rent_exemption(165)
            .await
            .unwrap();

        let create_account_ix = solana_sdk::system_instruction::create_account(
            &payer_pubkey,
            &account2_keypair.pubkey(),
            rent,
            165,
            &light_compressed_token::ID, // Use ctoken program as owner
        );

        context
            .rpc
            .create_and_send_transaction(
                &[create_account_ix],
                &payer_pubkey,
                &[&context.payer, &account2_keypair],
            )
            .await
            .unwrap();

        // Step 2: Initialize using SPL token initialize_account3 instruction
        // Note: initialize_account3 doesn't require account to be signer (SPL compatibility)
        let mut init_ix = spl_token_2022::instruction::initialize_account3(
            &spl_token_2022::ID,
            &account2_keypair.pubkey(),
            &context.mint_pubkey,
            &context.owner_keypair.pubkey(),
        )
        .unwrap();

        // Change program_id to ctoken program for compatibility test
        init_ix.program_id = light_compressed_token::ID;

        context
            .rpc
            .create_and_send_transaction(&[init_ix], &payer_pubkey, &[&context.payer])
            .await
            .unwrap();

        println!("Second token account created successfully");
    }

    println!("Setting up account balances for transfer...");

    // Set balance on account1 so we can transfer
    {
        let mut account1 = context
            .rpc
            .get_account(account1_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();

        let mut spl_account =
            spl_token_2022::state::Account::unpack_unchecked(&account1.data[..165]).unwrap();
        spl_account.amount = 1000; // Set 1000 tokens

        spl_token_2022::state::Account::pack(spl_account, &mut account1.data[..165]).unwrap();
        context.rpc.set_account(account1_keypair.pubkey(), account1);

        println!("Account1 balance set to 1000 tokens");
    }

    println!("Performing transfer using SPL instruction format...");

    // Transfer tokens using SPL token instruction format
    {
        let mut transfer_ix = spl_token_2022::instruction::transfer(
            &spl_token_2022::ID,
            &account1_keypair.pubkey(),
            &account2_keypair.pubkey(),
            &context.owner_keypair.pubkey(),
            &[],
            500, // Transfer 500 tokens
        )
        .unwrap();

        // Change program_id to ctoken program for compatibility test
        transfer_ix.program_id = light_compressed_token::ID;

        context
            .rpc
            .create_and_send_transaction(
                &[transfer_ix],
                &payer_pubkey,
                &[&context.payer, &context.owner_keypair],
            )
            .await
            .unwrap();

        println!("Transfer completed successfully");

        // Verify balances
        let account1 = context
            .rpc
            .get_account(account1_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();
        let account1_data =
            spl_token_2022::state::Account::unpack_unchecked(&account1.data[..165]).unwrap();
        assert_eq!(account1_data.amount, 500, "Account1 should have 500 tokens");

        let account2 = context
            .rpc
            .get_account(account2_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();
        let account2_data =
            spl_token_2022::state::Account::unpack_unchecked(&account2.data[..165]).unwrap();
        assert_eq!(account2_data.amount, 500, "Account2 should have 500 tokens");

        println!("Balances verified: Account1=500, Account2=500");
    }

    println!("Closing first account using SPL instruction format...");

    // Close first account using SPL token instruction format
    {
        // First, transfer remaining balance to account2
        let mut transfer_ix = spl_token_2022::instruction::transfer(
            &spl_token_2022::ID,
            &account1_keypair.pubkey(),
            &account2_keypair.pubkey(),
            &context.owner_keypair.pubkey(),
            &[],
            500, // Transfer remaining 500 tokens
        )
        .unwrap();
        transfer_ix.program_id = light_compressed_token::ID;

        context
            .rpc
            .create_and_send_transaction(
                &[transfer_ix],
                &payer_pubkey,
                &[&context.payer, &context.owner_keypair],
            )
            .await
            .unwrap();

        // Now close the account
        let mut close_ix = spl_token_2022::instruction::close_account(
            &spl_token_2022::ID,
            &account1_keypair.pubkey(),
            &payer_pubkey, // Destination for lamports
            &context.owner_keypair.pubkey(),
            &[],
        )
        .unwrap();

        // Change program_id to ctoken program for compatibility test
        close_ix.program_id = light_compressed_token::ID;

        context
            .rpc
            .create_and_send_transaction(
                &[close_ix],
                &payer_pubkey,
                &[&context.payer, &context.owner_keypair],
            )
            .await
            .unwrap();

        println!("First account closed successfully");

        // Verify account is closed
        let account1_result = context.rpc.get_account(account1_keypair.pubkey()).await;
        assert!(
            account1_result.is_err() || account1_result.unwrap().is_none(),
            "Account1 should be closed"
        );
    }

    println!("Closing second account using SPL instruction format...");

    // Close second account using SPL token instruction format
    {
        // First, transfer all tokens out (to payer, doesn't matter where)
        // Actually, for closing we need zero balance, so let's just set it to zero directly
        let mut account2 = context
            .rpc
            .get_account(account2_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();

        let mut spl_account =
            spl_token_2022::state::Account::unpack_unchecked(&account2.data[..165]).unwrap();
        spl_account.amount = 0; // Set to zero for close

        spl_token_2022::state::Account::pack(spl_account, &mut account2.data[..165]).unwrap();
        context.rpc.set_account(account2_keypair.pubkey(), account2);

        // Now close the account
        let mut close_ix = spl_token_2022::instruction::close_account(
            &spl_token_2022::ID,
            &account2_keypair.pubkey(),
            &payer_pubkey, // Destination for lamports
            &context.owner_keypair.pubkey(),
            &[],
        )
        .unwrap();

        // Change program_id to ctoken program for compatibility test
        close_ix.program_id = light_compressed_token::ID;

        context
            .rpc
            .create_and_send_transaction(
                &[close_ix],
                &payer_pubkey,
                &[&context.payer, &context.owner_keypair],
            )
            .await
            .unwrap();

        println!("Second account closed successfully");

        // Verify account is closed
        let account2_result = context.rpc.get_account(account2_keypair.pubkey()).await;
        assert!(
            account2_result.is_err() || account2_result.unwrap().is_none(),
            "Account2 should be closed"
        );
    }

    println!("\nSPL instruction compatibility test passed!");
    println!("   - Created 2 accounts using SPL initialize_account3");
    println!("   - Transferred tokens using SPL transfer");
    println!("   - Closed both accounts using SPL close_account");
    println!("   - All SPL token instructions are compatible with ctoken program");
}

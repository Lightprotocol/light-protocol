use anchor_spl::token_2022::spl_token_2022;
use serial_test::serial;
use solana_sdk::{program_pack::Pack, signature::Keypair, signer::Signer};

use super::shared::*;

/// Test SPL token instruction compatibility with ctoken program
///
/// This test creates SPL token instructions using the official spl_token library,
/// then changes the program_id to the ctoken program to verify instruction format compatibility.
///
/// Non-compressible accounts (165 bytes) are fully SPL-compatible:
/// - CreateTokenAccount with 32 bytes of instruction data (owner only) works
/// - Transfer, TransferChecked, Approve, Revoke, Close all work with SPL instruction format
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

    println!("Testing approve using SPL instruction format...");

    // Approve delegate using SPL token instruction format
    {
        let delegate = Keypair::new();

        let mut approve_ix = spl_token_2022::instruction::approve(
            &spl_token_2022::ID,
            &account1_keypair.pubkey(),
            &delegate.pubkey(),
            &context.owner_keypair.pubkey(),
            &[],
            200, // Approve 200 tokens
        )
        .unwrap();

        // Change program_id to ctoken program for compatibility test
        approve_ix.program_id = light_compressed_token::ID;

        context
            .rpc
            .create_and_send_transaction(
                &[approve_ix],
                &payer_pubkey,
                &[&context.payer, &context.owner_keypair],
            )
            .await
            .unwrap();

        // Verify delegate was set
        let account1 = context
            .rpc
            .get_account(account1_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();
        let account1_data =
            spl_token_2022::state::Account::unpack_unchecked(&account1.data[..165]).unwrap();
        assert_eq!(
            account1_data.delegate,
            solana_sdk::program_option::COption::Some(delegate.pubkey()),
            "Delegate should be set"
        );
        assert_eq!(
            account1_data.delegated_amount, 200,
            "Delegated amount should be 200"
        );

        println!("Approve completed successfully");
    }

    println!("Testing revoke using SPL instruction format...");

    // Revoke delegate using SPL token instruction format
    {
        let mut revoke_ix = spl_token_2022::instruction::revoke(
            &spl_token_2022::ID,
            &account1_keypair.pubkey(),
            &context.owner_keypair.pubkey(),
            &[],
        )
        .unwrap();

        // Change program_id to ctoken program for compatibility test
        revoke_ix.program_id = light_compressed_token::ID;

        context
            .rpc
            .create_and_send_transaction(
                &[revoke_ix],
                &payer_pubkey,
                &[&context.payer, &context.owner_keypair],
            )
            .await
            .unwrap();

        // Verify delegate was revoked
        let account1 = context
            .rpc
            .get_account(account1_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();
        let account1_data =
            spl_token_2022::state::Account::unpack_unchecked(&account1.data[..165]).unwrap();
        assert_eq!(
            account1_data.delegate,
            solana_sdk::program_option::COption::None,
            "Delegate should be revoked"
        );

        println!("Revoke completed successfully");
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
    println!("   - Approved delegate using SPL approve");
    println!("   - Revoked delegate using SPL revoke");
    println!("   - Closed both accounts using SPL close_account");
    println!("   - All SPL token instructions are compatible with ctoken program");
}

/// Test SPL token instruction compatibility with ctoken program using decompressed cmint
///
/// This test uses a real decompressed cmint to test instructions that require mint data:
/// - transfer_checked,
/// - mint_to, mint_to_checked (require mint authority)
/// - burn, burn_checked (require token burning)
/// - freeze_account, thaw_account (require freeze authority)
#[tokio::test]
#[serial]
#[allow(deprecated)]
async fn test_spl_instruction_compatibility_with_cmint() {
    use light_program_test::ProgramTestConfig;
    use light_token_client::instructions::mint_action::DecompressMintParams;
    use light_token_sdk::compressed_token::create_compressed_mint::find_mint_address;

    // Set up test environment
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();
    let mint_seed = Keypair::new();
    let mint_authority = payer.insecure_clone();
    let freeze_authority = Keypair::new();
    let owner_keypair = Keypair::new();

    // Derive CMint PDA
    let (cmint_pda, _) = find_mint_address(&mint_seed.pubkey());
    let decimals: u8 = 8;

    println!("Creating decompressed cmint with freeze authority...");

    // Create compressed mint + CMint (decompressed mint)
    light_token_client::actions::mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &mint_authority,
        &payer,
        Some(DecompressMintParams::default()), // Creates CMint
        false,                                 // Don't compress and close
        vec![],                                // No compressed recipients
        vec![],                                // No ctoken recipients
        None,                                  // No mint authority update
        None,                                  // No freeze authority update
        Some(light_token_client::instructions::mint_action::NewMint {
            decimals,
            supply: 0,
            mint_authority: mint_authority.pubkey(),
            freeze_authority: Some(freeze_authority.pubkey()),
            metadata: None,
            version: 3,
        }),
    )
    .await
    .unwrap();

    println!("CMint created at: {}", cmint_pda);

    // Create two non-compressible Light Token accounts (165 bytes) using SPL instruction format
    let account1_keypair = Keypair::new();
    let account2_keypair = Keypair::new();

    println!("Creating first non-compressible Light Token account...");

    // Create first account
    {
        let rent = rpc
            .get_minimum_balance_for_rent_exemption(165)
            .await
            .unwrap();

        let create_account_ix = solana_sdk::system_instruction::create_account(
            &payer_pubkey,
            &account1_keypair.pubkey(),
            rent,
            165,
            &light_compressed_token::ID,
        );

        rpc.create_and_send_transaction(
            &[create_account_ix],
            &payer_pubkey,
            &[&payer, &account1_keypair],
        )
        .await
        .unwrap();

        // Initialize using SPL instruction format
        let mut init_ix = spl_token_2022::instruction::initialize_account3(
            &spl_token_2022::ID,
            &account1_keypair.pubkey(),
            &cmint_pda,
            &owner_keypair.pubkey(),
        )
        .unwrap();
        init_ix.program_id = light_compressed_token::ID;

        rpc.create_and_send_transaction(&[init_ix], &payer_pubkey, &[&payer])
            .await
            .unwrap();

        println!("First account created");
    }

    println!("Creating second non-compressible Light Token account...");

    // Create second account
    {
        let rent = rpc
            .get_minimum_balance_for_rent_exemption(165)
            .await
            .unwrap();

        let create_account_ix = solana_sdk::system_instruction::create_account(
            &payer_pubkey,
            &account2_keypair.pubkey(),
            rent,
            165,
            &light_compressed_token::ID,
        );

        rpc.create_and_send_transaction(
            &[create_account_ix],
            &payer_pubkey,
            &[&payer, &account2_keypair],
        )
        .await
        .unwrap();

        // Initialize using SPL instruction format
        let mut init_ix = spl_token_2022::instruction::initialize_account3(
            &spl_token_2022::ID,
            &account2_keypair.pubkey(),
            &cmint_pda,
            &owner_keypair.pubkey(),
        )
        .unwrap();
        init_ix.program_id = light_compressed_token::ID;

        rpc.create_and_send_transaction(&[init_ix], &payer_pubkey, &[&payer])
            .await
            .unwrap();

        println!("Second account created");
    }

    println!("Testing mint_to using SPL instruction format...");

    // MintTo using SPL instruction format
    {
        let mut mint_to_ix = spl_token_2022::instruction::mint_to(
            &spl_token_2022::ID,
            &cmint_pda,
            &account1_keypair.pubkey(),
            &mint_authority.pubkey(),
            &[],
            1000,
        )
        .unwrap();
        mint_to_ix.program_id = light_compressed_token::ID;

        rpc.create_and_send_transaction(&[mint_to_ix], &payer_pubkey, &[&payer, &mint_authority])
            .await
            .unwrap();

        // Verify balance
        let account1 = rpc
            .get_account(account1_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();
        let account1_data =
            spl_token_2022::state::Account::unpack_unchecked(&account1.data[..165]).unwrap();
        assert_eq!(
            account1_data.amount, 1000,
            "Account1 should have 1000 tokens"
        );

        println!("mint_to completed successfully");
    }

    println!("Testing mint_to_checked using SPL instruction format...");

    // MintToChecked using SPL instruction format
    {
        let mut mint_to_checked_ix = spl_token_2022::instruction::mint_to_checked(
            &spl_token_2022::ID,
            &cmint_pda,
            &account1_keypair.pubkey(),
            &mint_authority.pubkey(),
            &[],
            500,
            decimals,
        )
        .unwrap();
        mint_to_checked_ix.program_id = light_compressed_token::ID;

        rpc.create_and_send_transaction(
            &[mint_to_checked_ix],
            &payer_pubkey,
            &[&payer, &mint_authority],
        )
        .await
        .unwrap();

        // Verify balance
        let account1 = rpc
            .get_account(account1_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();
        let account1_data =
            spl_token_2022::state::Account::unpack_unchecked(&account1.data[..165]).unwrap();
        assert_eq!(
            account1_data.amount, 1500,
            "Account1 should have 1500 tokens"
        );

        println!("mint_to_checked completed successfully");
    }

    println!("Testing transfer_checked using SPL instruction format...");

    // TransferChecked using SPL instruction format
    {
        let mut transfer_checked_ix = spl_token_2022::instruction::transfer_checked(
            &spl_token_2022::ID,
            &account1_keypair.pubkey(),
            &cmint_pda,
            &account2_keypair.pubkey(),
            &owner_keypair.pubkey(),
            &[],
            500,
            decimals,
        )
        .unwrap();
        transfer_checked_ix.program_id = light_compressed_token::ID;

        rpc.create_and_send_transaction(
            &[transfer_checked_ix],
            &payer_pubkey,
            &[&payer, &owner_keypair],
        )
        .await
        .unwrap();

        // Verify balances
        let account1 = rpc
            .get_account(account1_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();
        let account1_data =
            spl_token_2022::state::Account::unpack_unchecked(&account1.data[..165]).unwrap();
        assert_eq!(
            account1_data.amount, 1000,
            "Account1 should have 1000 tokens"
        );

        let account2 = rpc
            .get_account(account2_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();
        let account2_data =
            spl_token_2022::state::Account::unpack_unchecked(&account2.data[..165]).unwrap();
        assert_eq!(account2_data.amount, 500, "Account2 should have 500 tokens");

        println!("transfer_checked completed successfully");
    }

    println!("Testing freeze_account using SPL instruction format...");

    // FreezeAccount using SPL instruction format
    {
        let mut freeze_ix = spl_token_2022::instruction::freeze_account(
            &spl_token_2022::ID,
            &account1_keypair.pubkey(),
            &cmint_pda,
            &freeze_authority.pubkey(),
            &[],
        )
        .unwrap();
        freeze_ix.program_id = light_compressed_token::ID;

        rpc.create_and_send_transaction(&[freeze_ix], &payer_pubkey, &[&payer, &freeze_authority])
            .await
            .unwrap();

        // Verify account is frozen
        let account1 = rpc
            .get_account(account1_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();
        let account1_data =
            spl_token_2022::state::Account::unpack_unchecked(&account1.data[..165]).unwrap();
        assert_eq!(
            account1_data.state,
            spl_token_2022::state::AccountState::Frozen,
            "Account should be frozen"
        );

        println!("freeze_account completed successfully");
    }

    println!("Testing thaw_account using SPL instruction format...");

    // ThawAccount using SPL instruction format
    {
        let mut thaw_ix = spl_token_2022::instruction::thaw_account(
            &spl_token_2022::ID,
            &account1_keypair.pubkey(),
            &cmint_pda,
            &freeze_authority.pubkey(),
            &[],
        )
        .unwrap();
        thaw_ix.program_id = light_compressed_token::ID;

        rpc.create_and_send_transaction(&[thaw_ix], &payer_pubkey, &[&payer, &freeze_authority])
            .await
            .unwrap();

        // Verify account is thawed
        let account1 = rpc
            .get_account(account1_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();
        let account1_data =
            spl_token_2022::state::Account::unpack_unchecked(&account1.data[..165]).unwrap();
        assert_eq!(
            account1_data.state,
            spl_token_2022::state::AccountState::Initialized,
            "Account should be thawed"
        );

        println!("thaw_account completed successfully");
    }

    println!("Testing burn using SPL instruction format...");

    // Burn using SPL instruction format
    {
        let mut burn_ix = spl_token_2022::instruction::burn(
            &spl_token_2022::ID,
            &account1_keypair.pubkey(),
            &cmint_pda,
            &owner_keypair.pubkey(),
            &[],
            100,
        )
        .unwrap();
        burn_ix.program_id = light_compressed_token::ID;

        rpc.create_and_send_transaction(&[burn_ix], &payer_pubkey, &[&payer, &owner_keypair])
            .await
            .unwrap();

        // Verify balance decreased
        let account1 = rpc
            .get_account(account1_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();
        let account1_data =
            spl_token_2022::state::Account::unpack_unchecked(&account1.data[..165]).unwrap();
        assert_eq!(
            account1_data.amount, 900,
            "Account1 should have 900 tokens after burn"
        );

        println!("burn completed successfully");
    }

    println!("Testing burn_checked using SPL instruction format...");

    // BurnChecked using SPL instruction format
    {
        let mut burn_checked_ix = spl_token_2022::instruction::burn_checked(
            &spl_token_2022::ID,
            &account1_keypair.pubkey(),
            &cmint_pda,
            &owner_keypair.pubkey(),
            &[],
            100,
            decimals,
        )
        .unwrap();
        burn_checked_ix.program_id = light_compressed_token::ID;

        rpc.create_and_send_transaction(
            &[burn_checked_ix],
            &payer_pubkey,
            &[&payer, &owner_keypair],
        )
        .await
        .unwrap();

        // Verify balance decreased
        let account1 = rpc
            .get_account(account1_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();
        let account1_data =
            spl_token_2022::state::Account::unpack_unchecked(&account1.data[..165]).unwrap();
        assert_eq!(
            account1_data.amount, 800,
            "Account1 should have 800 tokens after burn_checked"
        );

        println!("burn_checked completed successfully");
    }

    println!("\nSPL instruction compatibility with CMint test passed!");
    println!("   - Created 2 non-compressible Light Token accounts with CMint");
    println!("   - mint_to: Minted 1000 tokens");
    println!("   - mint_to_checked: Minted 500 tokens with decimals validation");
    println!("   - transfer_checked: Transferred 500 tokens with decimals validation");
    println!("   - freeze_account: Froze account");
    println!("   - thaw_account: Thawed account");
    println!("   - burn: Burned 100 tokens");
    println!("   - burn_checked: Burned 100 tokens with decimals validation");
}

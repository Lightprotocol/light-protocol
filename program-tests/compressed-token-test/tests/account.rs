// #![cfg(feature = "test-sbf")]

use light_compressed_token_sdk::instructions::{
    close::close_account, create_associated_token_account::derive_ctoken_ata, create_token_account,
};
use light_ctoken_types::{BASIC_TOKEN_ACCOUNT_SIZE, COMPRESSIBLE_TOKEN_ACCOUNT_SIZE};
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    assert_close_token_account::assert_close_token_account,
    assert_create_token_account::{
        assert_create_associated_token_account, assert_create_token_account, CompressibleData,
    },
    Rpc,
};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

#[tokio::test]
async fn test_create_and_close_token_account() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Create a mock mint pubkey (we don't need actual mint for this test)
    let mint_pubkey = Pubkey::new_unique();

    // Create owner for the token account
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();

    // Create a new keypair for the token account
    let token_account_keypair = Keypair::new();
    let token_account_pubkey = token_account_keypair.pubkey();

    // First create the account using system program
    let create_account_system_ix = solana_sdk::system_instruction::create_account(
        &payer_pubkey,
        &token_account_pubkey,
        rpc.get_minimum_balance_for_rent_exemption(165)
            .await
            .unwrap(), // SPL token account size
        165,
        &light_compressed_token::ID, // Our program owns the account
    );

    // Then use SPL token SDK format but with our compressed token program ID
    // This tests that our create_token_account instruction is compatible with SPL SDKs
    let mut initialize_account_ix =
        create_token_account(token_account_pubkey, mint_pubkey, owner_pubkey).unwrap();
    initialize_account_ix.data.push(0);
    // Execute both instructions in one transaction
    rpc.create_and_send_transaction(
        &[create_account_system_ix, initialize_account_ix],
        &payer.pubkey(),
        &[&payer, &token_account_keypair],
    )
    .await
    .expect("Failed to create token account using SPL SDK");

    // Verify the token account was created correctly
    assert_create_token_account(
        &mut rpc,
        token_account_pubkey,
        mint_pubkey,
        owner_pubkey,
        None, // Basic token account
    )
    .await;

    // Now test closing the account using SPL SDK format
    let destination_keypair = Keypair::new();
    let destination_pubkey = destination_keypair.pubkey();

    // Airdrop some lamports to destination account so it exists
    rpc.context.airdrop(&destination_pubkey, 1_000_000).unwrap();

    // Get initial destination lamports before closing
    let initial_destination_lamports = rpc
        .get_account(destination_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Create close account instruction using SPL SDK format
    let close_account_ix = close_account(
        &light_compressed_token::ID,
        &token_account_pubkey,
        &destination_pubkey,
        &owner_pubkey,
    );

    rpc.create_and_send_transaction(
        &[close_account_ix],
        &payer.pubkey(),
        &[&payer, &owner_keypair],
    )
    .await
    .expect("Failed to close token account using SPL SDK");

    // Verify the account was closed correctly
    assert_close_token_account(
        &mut rpc,
        token_account_pubkey,
        None,
        destination_pubkey,
        initial_destination_lamports,
    )
    .await;
}

#[tokio::test]
async fn test_create_and_close_account_with_rent_authority() {
    use solana_sdk::{signature::Signer, system_instruction};

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Create mint
    let mint_pubkey = Pubkey::new_unique();

    // Create account owner
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();

    // Create rent authority
    let rent_authority_keypair = Keypair::new();
    let rent_authority_pubkey = rent_authority_keypair.pubkey();

    // Create rent recipient
    let rent_recipient_keypair = Keypair::new();
    let rent_recipient_pubkey = rent_recipient_keypair.pubkey();

    // Airdrop lamports to rent recipient so it exists
    rpc.context
        .airdrop(&rent_recipient_pubkey, 1_000_000)
        .unwrap();

    // Create token account keypair
    let token_account_keypair = Keypair::new();
    let token_account_pubkey = token_account_keypair.pubkey();

    // Create system account for token account with space for compressible extension
    let rent_exempt_lamports = rpc
        .get_minimum_balance_for_rent_exemption(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize)
        .await
        .unwrap();

    let create_account_ix = system_instruction::create_account(
        &payer_pubkey,
        &token_account_pubkey,
        rent_exempt_lamports,
        COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
        &light_compressed_token::ID,
    );

    // Create token account using SDK function with compressible extension
    let create_token_account_ix =
        light_compressed_token_sdk::instructions::create_compressible_token_account(
            light_compressed_token_sdk::instructions::CreateCompressibleTokenAccount {
                account_pubkey: token_account_pubkey,
                mint_pubkey,
                owner_pubkey,
                rent_authority: rent_authority_pubkey,
                rent_recipient: rent_recipient_pubkey,
                slots_until_compression: 0, // Allow immediate compression
            },
        )
        .unwrap();

    rpc.create_and_send_transaction(
        &[create_account_ix, create_token_account_ix],
        &payer.pubkey(),
        &[&payer, &token_account_keypair],
    )
    .await
    .expect("Failed to create token account");

    // Verify the account was created correctly
    assert_create_token_account(
        &mut rpc,
        token_account_pubkey,
        mint_pubkey,
        owner_pubkey,
        Some(CompressibleData {
            rent_authority: rent_authority_pubkey,
            rent_recipient: rent_recipient_pubkey,
            slots_until_compression: 0,
        }),
    )
    .await;

    // Get initial recipient lamports before closing
    let initial_recipient_lamports = rpc
        .get_account(rent_recipient_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // First, try to close with rent authority (should fail for basic token account)
    let close_account_ix = close_account(
        &light_compressed_token::ID,
        &token_account_pubkey,
        &rent_recipient_pubkey, // Use rent recipient as destination
        &rent_authority_pubkey, // Use rent authority as authority
    );

    // Get account data before closing for assertion
    let account_data_before_close = rpc
        .get_account(token_account_pubkey)
        .await
        .unwrap()
        .unwrap()
        .data;

    rpc.create_and_send_transaction(
        &[close_account_ix],
        &payer.pubkey(),
        &[&payer, &rent_authority_keypair],
    )
    .await
    .unwrap();

    // Verify the account was closed correctly
    assert_close_token_account(
        &mut rpc,
        token_account_pubkey,
        Some(&account_data_before_close),
        rent_recipient_pubkey,
        initial_recipient_lamports,
    )
    .await;
}

#[tokio::test]
async fn test_create_compressible_account_insufficient_size() {
    use light_test_utils::spl::create_mint_helper;
    use solana_sdk::{signature::Signer, system_instruction};

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Create mint
    let mint_pubkey = create_mint_helper(&mut rpc, &payer).await;

    // Create owner and rent authority keypairs
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let rent_authority_keypair = Keypair::new();
    let rent_authority_pubkey = rent_authority_keypair.pubkey();
    let rent_recipient_keypair = Keypair::new();
    let rent_recipient_pubkey = rent_recipient_keypair.pubkey();

    // Create token account keypair
    let token_account_keypair = Keypair::new();
    let token_account_pubkey = token_account_keypair.pubkey();

    // Create system account with INSUFFICIENT size - too small for compressible extension
    let rent_exempt_lamports = rpc
        .get_minimum_balance_for_rent_exemption(BASIC_TOKEN_ACCOUNT_SIZE as usize)
        .await
        .unwrap();

    let create_account_ix = system_instruction::create_account(
        &payer_pubkey,
        &token_account_pubkey,
        rent_exempt_lamports,
        light_ctoken_types::BASIC_TOKEN_ACCOUNT_SIZE, // Intentionally too small for compressible extension
        &light_compressed_token::ID,
    );

    // Create token account using SDK function with compressible extension
    let create_token_account_ix =
        light_compressed_token_sdk::instructions::create_compressible_token_account(
            light_compressed_token_sdk::instructions::CreateCompressibleTokenAccount {
                account_pubkey: token_account_pubkey,
                mint_pubkey,
                owner_pubkey,
                rent_authority: rent_authority_pubkey,
                rent_recipient: rent_recipient_pubkey,
                slots_until_compression: 0,
            },
        )
        .unwrap();

    // Execute account creation - this should fail with account size error
    let result = rpc
        .create_and_send_transaction(
            &[create_account_ix, create_token_account_ix],
            &payer.pubkey(),
            &[&payer, &token_account_keypair],
        )
        .await;
    assert!(
        result.is_err(),
        "Expected account creation to fail due to insufficient account size"
    );

    println!("✅ Correctly failed to create compressible token account with insufficient size");
}

#[tokio::test]
async fn test_create_associated_token_account() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Create a mock mint pubkey
    let mint_pubkey = Pubkey::new_unique();

    // Create owner for the associated token account
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();

    // Create basic ATA instruction using SDK function
    let instruction = light_compressed_token_sdk::instructions::create_associated_token_account(
        payer_pubkey,
        owner_pubkey,
        mint_pubkey,
    )
    .unwrap();

    // Execute the instruction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("Failed to create associated token account");

    // Verify the associated token account was created correctly
    assert_create_associated_token_account(&mut rpc, owner_pubkey, mint_pubkey, None).await;

    // Test compressible associated token account creation
    println!("🧪 Testing compressible associated token account creation...");

    // Create rent authority and recipient for compressible account
    let rent_authority_keypair = Keypair::new();
    let rent_authority_pubkey = rent_authority_keypair.pubkey();
    let rent_recipient_keypair = Keypair::new();
    let rent_recipient_pubkey = rent_recipient_keypair.pubkey();

    // Airdrop lamports to rent recipient so it exists
    rpc.context
        .airdrop(&rent_recipient_pubkey, 1_000_000)
        .unwrap();

    // Create a different owner for the compressible account
    let compressible_owner_keypair = Keypair::new();
    let compressible_owner_pubkey = compressible_owner_keypair.pubkey();

    // Create compressible ATA instruction using SDK function
    let compressible_instruction = light_compressed_token_sdk::instructions::create_compressible_associated_token_account(
        light_compressed_token_sdk::instructions::CreateCompressibleAssociatedTokenAccountInputs {
            payer: payer_pubkey,
            owner: compressible_owner_pubkey,
            mint: mint_pubkey,
            rent_authority: rent_authority_pubkey,
            rent_recipient: rent_recipient_pubkey,
            slots_until_compression: 0,
        }
    ).unwrap();

    rpc.create_and_send_transaction(&[compressible_instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("Failed to create compressible associated token account");

    // Verify the compressible associated token account was created correctly
    assert_create_associated_token_account(
        &mut rpc,
        compressible_owner_pubkey,
        mint_pubkey,
        Some(CompressibleData {
            rent_authority: rent_authority_pubkey,
            rent_recipient: rent_recipient_pubkey,
            slots_until_compression: 0,
        }),
    )
    .await;

    // Test that we can close the compressible account using rent authority
    // Re-derive the ATA address for closing test
    let (expected_compressible_ata_pubkey, _) =
        derive_ctoken_ata(&compressible_owner_pubkey, &mint_pubkey);

    let initial_recipient_lamports = rpc
        .get_account(rent_recipient_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Close account with rent authority
    let close_account_ix = close_account(
        &light_compressed_token::ID,
        &expected_compressible_ata_pubkey,
        &rent_recipient_pubkey,
        &rent_authority_pubkey,
    );

    // Get account data before closing for assertion
    let account_data_before_close = rpc
        .get_account(expected_compressible_ata_pubkey)
        .await
        .unwrap()
        .unwrap()
        .data;

    rpc.create_and_send_transaction(
        &[close_account_ix],
        &payer.pubkey(),
        &[&payer, &rent_authority_keypair],
    )
    .await
    .unwrap();

    // Verify the compressible account was closed correctly
    assert_close_token_account(
        &mut rpc,
        expected_compressible_ata_pubkey,
        Some(&account_data_before_close),
        rent_recipient_pubkey,
        initial_recipient_lamports,
    )
    .await;

    println!("✅ Both basic and compressible associated token accounts work correctly!");
}

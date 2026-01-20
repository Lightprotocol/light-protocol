use light_test_utils::assert_create_token_account::assert_create_associated_token_account;
use light_token::instruction::{CloseAccount, CompressibleParams, CreateAssociatedTokenAccount};

use super::shared::*;

/// Test:
/// 1. SUCCESS: Create basic associated token account using SDK function
/// 2. SUCCESS: Verify basic ATA structure using existing assertion helper
/// 3. SUCCESS: Create compressible associated token account with rent authority
/// 4. SUCCESS: Verify compressible ATA structure using existing assertion helper
/// 5. SUCCESS: Close compressible ATA using rent authority
/// 6. SUCCESS: Verify lamports transferred to rent recipient using existing assertion helper
#[tokio::test]
#[serial]
async fn test_associated_token_account_operations() {
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();
    let owner_pubkey = context.owner_keypair.pubkey();

    // Create ATA with 0 prepaid epochs (immediately compressible)
    let compressible_params = CompressibleParams {
        compressible_config: context.compressible_config,
        rent_sponsor: context.rent_sponsor,
        pre_pay_num_epochs: 0,
        lamports_per_write: None,
        compress_to_account_pubkey: None,
        token_account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
        compression_only: true,
    };

    let instruction =
        CreateAssociatedTokenAccount::new(payer_pubkey, owner_pubkey, context.mint_pubkey)
            .with_compressible(compressible_params)
            .instruction()
            .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[instruction], &payer_pubkey, &[&context.payer])
        .await
        .unwrap();

    // Verify ATA creation using existing assertion helper
    // Pass CompressibleData with 0 prepaid epochs since all accounts now have compression infrastructure
    let compressible_data = CompressibleData {
        compression_authority: context.compression_authority,
        rent_sponsor: context.rent_sponsor,
        num_prepaid_epochs: 0,
        lamports_per_write: None,
        account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
        compress_to_pubkey: false,
        payer: payer_pubkey,
    };

    assert_create_associated_token_account(
        &mut context.rpc,
        owner_pubkey,
        context.mint_pubkey,
        Some(compressible_data),
        None,
    )
    .await;

    // Create compressible ATA with different owner
    let compressible_owner_keypair = Keypair::new();
    let compressible_owner_pubkey = compressible_owner_keypair.pubkey();

    let num_prepaid_epochs = 0;
    let lamports_per_write = Some(150);
    // Create compressible ATA
    let compressible_params = CompressibleParams {
        compressible_config: context.compressible_config,
        rent_sponsor: context.rent_sponsor,
        pre_pay_num_epochs: num_prepaid_epochs,
        lamports_per_write,
        compress_to_account_pubkey: None,
        token_account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
        compression_only: true,
    };

    let compressible_instruction = CreateAssociatedTokenAccount::new(
        payer_pubkey,
        compressible_owner_pubkey,
        context.mint_pubkey,
    )
    .with_compressible(compressible_params)
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(
            &[compressible_instruction],
            &payer_pubkey,
            &[&context.payer],
        )
        .await
        .unwrap();

    // Verify compressible ATA creation using existing assertion helper
    assert_create_associated_token_account(
        &mut context.rpc,
        compressible_owner_pubkey,
        context.mint_pubkey,
        Some(CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs, // Use actual balance with rent
            lamports_per_write,
            compress_to_pubkey: false,
            account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
            payer: payer_pubkey,
        }),
        None,
    )
    .await;

    // Test closing compressible ATA
    let (compressible_ata_pubkey, _) =
        derive_token_ata(&compressible_owner_pubkey, &context.mint_pubkey);

    // Create a separate destination account
    let destination = Keypair::new();
    context
        .rpc
        .airdrop_lamports(&destination.pubkey(), 1_000_000)
        .await
        .unwrap();

    // Close compressible ATA
    let close_account_ix = CloseAccount::new(
        light_compressed_token::ID,
        compressible_ata_pubkey,
        destination.pubkey(),                // destination for user funds
        compressible_owner_keypair.pubkey(), // authority
    )
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(
            &[close_account_ix],
            &payer_pubkey,
            &[&context.payer, &compressible_owner_keypair],
        )
        .await
        .unwrap();

    // Verify compressible ATA closure using existing assertion helper
    assert_close_token_account(
        &mut context.rpc,
        compressible_ata_pubkey,
        compressible_owner_keypair.pubkey(),
        destination.pubkey(), // destination
    )
    .await;
}

/// Test:
/// 1. SUCCESS: Create ATA using non-idempotent instruction
/// 2. FAIL: Attempt to create same ATA again using non-idempotent instruction (should fail)
/// 3. SUCCESS: Create same ATA using idempotent instruction (should succeed)
#[tokio::test]
#[serial]
async fn test_create_ata_idempotent() {
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();
    let owner_pubkey = context.owner_keypair.pubkey();

    // Create ATA with 0 prepaid epochs using non-idempotent instruction (first creation)
    let compressible_params = CompressibleParams {
        compressible_config: context.compressible_config,
        rent_sponsor: context.rent_sponsor,
        pre_pay_num_epochs: 0,
        lamports_per_write: None,
        compress_to_account_pubkey: None,
        token_account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
        compression_only: true,
    };

    let instruction =
        CreateAssociatedTokenAccount::new(payer_pubkey, owner_pubkey, context.mint_pubkey)
            .with_compressible(compressible_params)
            .instruction()
            .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[instruction], &payer_pubkey, &[&context.payer])
        .await
        .unwrap();

    // Verify ATA creation
    let compressible_data = CompressibleData {
        compression_authority: context.compression_authority,
        rent_sponsor: context.rent_sponsor,
        num_prepaid_epochs: 0,
        lamports_per_write: None,
        account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
        compress_to_pubkey: false,
        payer: payer_pubkey,
    };

    assert_create_associated_token_account(
        &mut context.rpc,
        owner_pubkey,
        context.mint_pubkey,
        Some(compressible_data),
        None,
    )
    .await;

    // Attempt to create the same ATA again using non-idempotent instruction (should fail)
    let instruction =
        CreateAssociatedTokenAccount::new(payer_pubkey, owner_pubkey, context.mint_pubkey)
            .instruction()
            .unwrap();

    let result = context
        .rpc
        .create_and_send_transaction(&[instruction], &payer_pubkey, &[&context.payer])
        .await;

    // This should fail because account already exists
    assert!(
        result.is_err(),
        "Non-idempotent ATA creation should fail when account already exists"
    );

    // Now try with idempotent instruction (should succeed)
    let instruction =
        CreateAssociatedTokenAccount::new(payer_pubkey, owner_pubkey, context.mint_pubkey)
            .idempotent()
            .instruction()
            .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[instruction], &payer_pubkey, &[&context.payer])
        .await
        .unwrap();

    // Verify ATA is still correct - account was created with compressible params so still has compression
    assert_create_associated_token_account(
        &mut context.rpc,
        owner_pubkey,
        context.mint_pubkey,
        Some(CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs: 0,
            lamports_per_write: None,
            account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        }),
        None,
    )
    .await;
}

/// Test: DoS prevention for ATA creation
/// 1. Derive ATA address
/// 2. Pre-fund the ATA address with lamports (simulating attacker donation)
/// 3. SUCCESS: Create ATA should succeed despite pre-funded lamports
#[tokio::test]
#[serial]
async fn test_create_ata_with_prefunded_lamports() {
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();
    let owner_pubkey = context.owner_keypair.pubkey();

    // Derive ATA address
    let (ata, bump) = derive_token_ata(&owner_pubkey, &context.mint_pubkey);

    // Pre-fund the ATA address with lamports (simulating attacker donation DoS attempt)
    let prefund_amount = 1_000; // 1000 lamports
    let transfer_ix = solana_sdk::system_instruction::transfer(&payer_pubkey, &ata, prefund_amount);

    context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer_pubkey, &[&context.payer])
        .await
        .unwrap();

    // Verify the ATA address now has lamports
    let ata_account = context.rpc.get_account(ata).await.unwrap();
    assert!(
        ata_account.is_some(),
        "ATA address should exist with lamports"
    );
    assert_eq!(
        ata_account.unwrap().lamports,
        prefund_amount,
        "ATA should have pre-funded lamports"
    );

    // Now create the ATA - this should succeed despite pre-funded lamports
    let compressible_params = CompressibleParams {
        compressible_config: context.compressible_config,
        rent_sponsor: context.rent_sponsor,
        pre_pay_num_epochs: 0,
        lamports_per_write: None,
        compress_to_account_pubkey: None,
        token_account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
        compression_only: true,
    };

    let instruction = CreateAssociatedTokenAccount {
        idempotent: false,
        bump,
        payer: payer_pubkey,
        owner: owner_pubkey,
        mint: context.mint_pubkey,
        associated_token_account: ata,
        compressible: compressible_params,
    }
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[instruction], &payer_pubkey, &[&context.payer])
        .await
        .unwrap();

    // Verify ATA was created correctly
    assert_create_associated_token_account(
        &mut context.rpc,
        owner_pubkey,
        context.mint_pubkey,
        Some(CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs: 0,
            lamports_per_write: None,
            account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        }),
        None,
    )
    .await;

    // Verify the ATA now has more lamports (rent-exempt + pre-funded)
    let final_ata_account = context.rpc.get_account(ata).await.unwrap().unwrap();
    assert!(
        final_ata_account.lamports > prefund_amount,
        "ATA should have rent-exempt balance plus pre-funded amount"
    );
}

/// Test: DoS prevention for token account creation with custom rent payer
/// 1. Generate token account keypair
/// 2. Pre-fund the token account address with lamports (simulating attacker donation)
/// 3. SUCCESS: Create token account should succeed despite pre-funded lamports
#[tokio::test]
#[serial]
async fn test_create_token_account_with_prefunded_lamports() {
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();
    let token_account_pubkey = context.token_account_keypair.pubkey();

    // Pre-fund the token account address with lamports (simulating attacker donation DoS attempt)
    let prefund_amount = 1_000; // 1000 lamports
    let transfer_ix = solana_sdk::system_instruction::transfer(
        &payer_pubkey,
        &token_account_pubkey,
        prefund_amount,
    );

    context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer_pubkey, &[&context.payer])
        .await
        .unwrap();

    // Verify the token account address now has lamports
    let token_account = context.rpc.get_account(token_account_pubkey).await.unwrap();
    assert!(
        token_account.is_some(),
        "Token account address should exist with lamports"
    );
    assert_eq!(
        token_account.unwrap().lamports,
        prefund_amount,
        "Token account should have pre-funded lamports"
    );

    // Now create the compressible token account - this should succeed despite pre-funded lamports
    let compressible_params = CompressibleParams {
        compressible_config: context.compressible_config,
        rent_sponsor: context.rent_sponsor,
        pre_pay_num_epochs: 0,
        lamports_per_write: Some(100),
        compress_to_account_pubkey: None,
        token_account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
        compression_only: false, // Must be false for non-restricted mints (non-ATA accounts)
    };

    let create_token_account_ix = CreateTokenAccount::new(
        payer_pubkey,
        token_account_pubkey,
        context.mint_pubkey,
        context.owner_keypair.pubkey(),
    )
    .with_compressible(compressible_params)
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(
            &[create_token_account_ix],
            &payer_pubkey,
            &[&context.payer, &context.token_account_keypair],
        )
        .await
        .unwrap();

    // Verify token account was created correctly
    assert_create_token_account(
        &mut context.rpc,
        token_account_pubkey,
        context.mint_pubkey,
        context.owner_keypair.pubkey(),
        Some(CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs: 0,
            lamports_per_write: Some(100),
            compress_to_pubkey: false,
            account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
            payer: payer_pubkey,
        }),
        None,
    )
    .await;

    // Verify the token account now has more lamports (rent-exempt + pre-funded)
    let final_token_account = context
        .rpc
        .get_account(token_account_pubkey)
        .await
        .unwrap()
        .unwrap();
    assert!(
        final_token_account.lamports > prefund_amount,
        "Token account should have rent-exempt balance plus pre-funded amount"
    );
}

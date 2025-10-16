use light_compressed_token_sdk::instructions::create_associated_token_account_idempotent;
use light_test_utils::assert_create_token_account::assert_create_associated_token_account;

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

    // Create basic ATA using SDK function
    let instruction = light_compressed_token_sdk::instructions::create_associated_token_account(
        payer_pubkey,
        owner_pubkey,
        context.mint_pubkey,
    )
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[instruction], &payer_pubkey, &[&context.payer])
        .await
        .unwrap();

    // Verify basic ATA creation using existing assertion helper
    assert_create_associated_token_account(
        &mut context.rpc,
        owner_pubkey,
        context.mint_pubkey,
        None,
    )
    .await;

    // Create compressible ATA with different owner
    let compressible_owner_keypair = Keypair::new();
    let compressible_owner_pubkey = compressible_owner_keypair.pubkey();

    let num_prepaid_epochs = 0;
    let lamports_per_write = Some(150);
    // Create compressible ATA
    let compressible_instruction = light_compressed_token_sdk::instructions::create_compressible_associated_token_account(
        light_compressed_token_sdk::instructions::CreateCompressibleAssociatedTokenAccountInputs {
            payer: payer_pubkey,
            owner: compressible_owner_pubkey,
            mint: context.mint_pubkey,
            compressible_config: context.compressible_config,
            rent_sponsor: context.rent_sponsor,
            pre_pay_num_epochs: num_prepaid_epochs,
            lamports_per_write,
            token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
        }
    ).unwrap();

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
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            payer: payer_pubkey,
        }),
    )
    .await;

    // Test closing compressible ATA
    let (compressible_ata_pubkey, _) =
        derive_ctoken_ata(&compressible_owner_pubkey, &context.mint_pubkey);

    // Create a separate destination account
    let destination = Keypair::new();
    context
        .rpc
        .airdrop_lamports(&destination.pubkey(), 1_000_000)
        .await
        .unwrap();

    // Close compressible ATA
    let close_account_ix = close_compressible_account(
        &light_compressed_token::ID,
        &compressible_ata_pubkey,
        &destination.pubkey(),                // destination for user funds
        &compressible_owner_keypair.pubkey(), // authority
        &context.rent_sponsor,                // rent_sponsor
    );

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
    // Create ATA using non-idempotent instruction (first creation)
    let instruction = light_compressed_token_sdk::instructions::create_associated_token_account::create_associated_token_account(
        payer_pubkey,
        owner_pubkey,
        context.mint_pubkey,
    )
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[instruction], &payer_pubkey, &[&context.payer])
        .await
        .unwrap();

    // Verify ATA creation
    assert_create_associated_token_account(
        &mut context.rpc,
        owner_pubkey,
        context.mint_pubkey,
        None,
    )
    .await;

    // Attempt to create the same ATA again using non-idempotent instruction (should fail)
    let instruction = light_compressed_token_sdk::instructions::create_associated_token_account::create_associated_token_account(
        payer_pubkey,
        owner_pubkey,
        context.mint_pubkey,
    )
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
        create_associated_token_account_idempotent(payer_pubkey, owner_pubkey, context.mint_pubkey)
            .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[instruction], &payer_pubkey, &[&context.payer])
        .await
        .unwrap();

    // Verify ATA is still correct
    assert_create_associated_token_account(
        &mut context.rpc,
        owner_pubkey,
        context.mint_pubkey,
        None,
    )
    .await;
}

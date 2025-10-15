use super::shared::*;

/// Test:
/// 1. SUCCESS: Create system account with compressible token size
/// 2. SUCCESS: Initialize compressible token account with rent authority and recipient
/// 3. SUCCESS: Verify compressible account structure using existing assertion helper
/// 4. SUCCESS: Close account using rent authority
/// 5. SUCCESS: Verify lamports transferred to rent recipient using existing assertion helper
#[tokio::test]
#[serial]
async fn test_compressible_account_with_custom_rent_payer_close_with_owner() {
    let mut context = setup_account_test().await.unwrap();
    let first_tx_payer = Keypair::new();
    context
        .rpc
        .airdrop_lamports(&first_tx_payer.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let payer_pubkey = first_tx_payer.pubkey();
    let token_account_pubkey = context.token_account_keypair.pubkey();

    // Create system account with compressible size
    let rent_exemption = context
        .rpc
        .get_minimum_balance_for_rent_exemption(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize)
        .await
        .unwrap();

    let num_prepaid_epochs = 2;
    let lamports_per_write = Some(100);

    // Initialize compressible token account
    let create_token_account_ix =
        light_compressed_token_sdk::instructions::create_compressible_token_account(
            light_compressed_token_sdk::instructions::CreateCompressibleTokenAccount {
                account_pubkey: token_account_pubkey,
                mint_pubkey: context.mint_pubkey,
                owner_pubkey: context.owner_keypair.pubkey(),
                compressible_config: context.compressible_config,
                rent_sponsor: payer_pubkey,
                pre_pay_num_epochs: num_prepaid_epochs,
                lamports_per_write,
                payer: payer_pubkey,
                compress_to_account_pubkey: None,
                token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            },
        )
        .unwrap();
    // Verify pool PDA balance decreased by only the rent-exempt amount (not the additional rent)
    let pool_balance_before = context
        .rpc
        .get_account(payer_pubkey)
        .await
        .unwrap()
        .expect("Pool PDA should exist")
        .lamports;

    // Execute account creation
    context
        .rpc
        .create_and_send_transaction(
            &[create_token_account_ix],
            &payer_pubkey,
            &[&first_tx_payer, &context.token_account_keypair],
        )
        .await
        .unwrap();

    assert_create_token_account(
        &mut context.rpc,
        token_account_pubkey,
        context.mint_pubkey,
        context.owner_keypair.pubkey(),
        Some(CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: payer_pubkey,
            num_prepaid_epochs,
            lamports_per_write,
            compress_to_pubkey: false,
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            payer: payer_pubkey,
        }),
    )
    .await;

    // Verify pool PDA balance decreased by only the rent-exempt amount (not the additional rent)

    // Verify payer balance decreased by exactly 11,000 lamports (the additional rent)
    let payer_balance_after = context
        .rpc
        .get_account(payer_pubkey)
        .await
        .unwrap()
        .expect("Payer should exist")
        .lamports;
    let rent = RentConfig::default()
        .get_rent_with_compression_cost(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE, num_prepaid_epochs);
    let tx_fee = 10_000; // Standard transaction fee
    assert_eq!(
        pool_balance_before - payer_balance_after,
        rent_exemption + rent + tx_fee,
        "Pool PDA should have paid {} lamports for account creation (rent-exempt), and the additional rent",
        rent_exemption
    );

    // TEST: Compress 0 tokens from the compressible account (edge case)
    // This tests whether compression works with an empty compressible account
    {
        // Assert expects slot to change since creation.
        context.rpc.warp_to_slot(4).unwrap();

        let output_queue = context
            .rpc
            .get_random_state_tree_info()
            .unwrap()
            .get_output_pubkey()
            .unwrap();
        println!("compressing");
        compress(
            &mut context.rpc,
            token_account_pubkey,
            0, // Compress 0 tokens for test
            context.owner_keypair.pubkey(),
            &context.owner_keypair,
            &context.payer,
        )
        .await
        .unwrap();

        // Create compress input for assertion
        let compress_input = CompressInput {
            compressed_token_account: None,
            solana_token_account: token_account_pubkey,
            to: context.owner_keypair.pubkey(),
            mint: context.mint_pubkey,
            amount: 0,
            authority: context.owner_keypair.pubkey(),
            output_queue,
            pool_index: None,
        };
        assert_transfer2_compress(&mut context.rpc, compress_input).await;
    }

    // Create a separate destination account
    let destination = Keypair::new();
    context
        .rpc
        .airdrop_lamports(&destination.pubkey(), 1_000_000)
        .await
        .unwrap();

    // Close compressible account using owner
    let close_account_ix = close_compressible_account(
        &light_compressed_token::ID,
        &token_account_pubkey,
        &destination.pubkey(),           // destination for user funds
        &context.owner_keypair.pubkey(), // authority
        &payer_pubkey,                   // rent_sponsor (custom rent payer)
    );

    context
        .rpc
        .create_and_send_transaction(
            &[close_account_ix],
            &context.payer.pubkey(),
            &[&context.owner_keypair, &context.payer],
        )
        .await
        .unwrap();

    // Verify account closure using existing assertion helper
    assert_close_token_account(
        &mut context.rpc,
        token_account_pubkey,
        context.owner_keypair.pubkey(),
        destination.pubkey(), // destination
    )
    .await;
}

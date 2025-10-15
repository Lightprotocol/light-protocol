use super::shared::*;
use light_test_utils::spl::create_mint_helper;

/// Test compress_and_close with rent authority:
/// 1. Create compressible token account with rent authority
/// 2. Compress and close account using rent authority
/// 3. Verify rent goes to rent recipient
#[tokio::test]
#[serial]
async fn test_compress_and_close_with_compression_authority() {
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();
    let token_account_pubkey = context.token_account_keypair.pubkey();

    let mint_pubkey = create_mint_helper(&mut context.rpc, &context.payer).await;

    let create_token_account_ix =
        light_compressed_token_sdk::instructions::create_compressible_token_account(
            light_compressed_token_sdk::instructions::CreateCompressibleTokenAccount {
                account_pubkey: token_account_pubkey,
                mint_pubkey,
                owner_pubkey: context.owner_keypair.pubkey(),
                compressible_config: context.compressible_config,
                rent_sponsor: context.rent_sponsor,
                pre_pay_num_epochs: 2,
                lamports_per_write: Some(150),
                payer: payer_pubkey,
                compress_to_account_pubkey: None,
                token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            },
        )
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

    // Top up rent for one more epoch (total: 2 prepaid + 1 topped up = 3 epochs)
    context
        .rpc
        .airdrop_lamports(
            &token_account_pubkey,
            RentConfig::default().get_rent(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE, 1),
        )
        .await
        .unwrap();

    // Advance to epoch 1 (account not yet compressible - still has 2 epochs remaining)
    // Account was created with 2 epochs prepaid + 1 topped up = 3 epochs total
    // At epoch 1, only 1 epoch has passed, so 2 epochs of funding remain
    context.rpc.warp_to_slot(SLOTS_PER_EPOCH + 1).unwrap();
    let forster_keypair = context.rpc.test_accounts.protocol.forester.insecure_clone();
    // This doesnt work anymore we need to invoke the registry program now
    // // Compress and close using rent authority (with 0 balance)
    let result = compress_and_close_forester(
        &mut context.rpc,
        &[token_account_pubkey],
        &forster_keypair,
        &context.payer,
        None,
    )
    .await;

    assert!(
        result
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("invalid account data for instruction"),
        "{}",
        result.unwrap_err().to_string()
    );
    // Advance to epoch 3 to make the account compressible
    // Account was created with 2 epochs prepaid + 1 topped up = 3 epochs total
    // At epoch 3, all 3 epochs have passed, so the account is now compressible
    context.rpc.warp_to_slot((SLOTS_PER_EPOCH * 3) + 1).unwrap();

    // Create a fresh destination pubkey to receive the compression incentive
    let destination = solana_sdk::signature::Keypair::new();
    println!("Test destination pubkey: {:?}", destination.pubkey());

    // Airdrop lamports to destination so it exists and can receive the compression incentive
    context
        .rpc
        .airdrop_lamports(&destination.pubkey(), 1_000_000)
        .await
        .unwrap();

    compress_and_close_forester(
        &mut context.rpc,
        &[token_account_pubkey],
        &forster_keypair,
        &context.payer,
        Some(destination.pubkey()),
    )
    .await
    .unwrap();
    // Use the new assert_transfer2_compress_and_close for comprehensive validation
    use light_test_utils::assert_transfer2::assert_transfer2_compress_and_close;
    use light_token_client::instructions::transfer2::CompressAndCloseInput;
    let output_queue = context.rpc.get_random_state_tree_info().unwrap().queue;

    assert_transfer2_compress_and_close(
        &mut context.rpc,
        CompressAndCloseInput {
            solana_ctoken_account: token_account_pubkey,
            authority: context.compression_authority,
            output_queue,
            destination: Some(destination.pubkey()),
            is_compressible: true,
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn test_compressible_account_with_custom_rent_payer_close_with_compression_authority() {
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
    // Close and compress account with rent authority
    {
        let payer_balance_before = context
            .rpc
            .get_account(payer_pubkey)
            .await
            .unwrap()
            .expect("Payer should exist")
            .lamports;
        context.rpc.warp_epoch_forward(2).await.unwrap();
        let payer_balance_after = context
            .rpc
            .get_account(payer_pubkey)
            .await
            .unwrap()
            .expect("Payer should exist")
            .lamports;
        let rent =
            RentConfig::default().get_rent(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE, num_prepaid_epochs);
        assert_eq!(
            payer_balance_after,
            payer_balance_before + rent_exemption + rent,
            "Pool PDA should have paid {} lamports for account creation (rent-exempt), and the additional rent",
            rent_exemption
        );
        use light_client::indexer::Indexer;
        let compressed_token_account = context
            .rpc
            .get_compressed_token_accounts_by_owner(&context.owner_keypair.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items;
        assert_eq!(compressed_token_account.len(), 1);
    }
}

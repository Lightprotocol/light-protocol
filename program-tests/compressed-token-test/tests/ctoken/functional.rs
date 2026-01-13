use super::shared::*;
/// Test:
/// 1. SUCCESS: Create CToken account with 0 prepaid epochs (immediately compressible)
/// 2. SUCCESS: Verify account structure and ownership using existing assertion helpers
/// 3. SUCCESS: Close account transferring lamports to destination
/// 4. SUCCESS: Verify account closure and lamport transfer using existing assertion helpers
#[tokio::test]
#[serial]
async fn test_spl_sdk_compatible_account_lifecycle() -> Result<(), RpcError> {
    let mut context = setup_account_test().await?;
    let payer_pubkey = context.payer.pubkey();
    let token_account_pubkey = context.token_account_keypair.pubkey();

    // Create CToken account with 0 prepaid epochs (immediately compressible)
    let compressible_params = CompressibleParams {
        compressible_config: context.compressible_config,
        rent_sponsor: context.rent_sponsor,
        pre_pay_num_epochs: 0,
        lamports_per_write: None,
        compress_to_account_pubkey: None,
        token_account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
        compression_only: false,
    };

    let create_ix = CreateTokenAccount::new(
        payer_pubkey,
        token_account_pubkey,
        context.mint_pubkey,
        context.owner_keypair.pubkey(),
    )
    .with_compressible(compressible_params)
    .instruction()
    .map_err(|e| {
        RpcError::AssertRpcError(format!("Failed to create token account instruction: {}", e))
    })?;

    // Execute account creation
    context
        .rpc
        .create_and_send_transaction(
            &[create_ix],
            &payer_pubkey,
            &[&context.payer, &context.token_account_keypair],
        )
        .await?;

    // Verify account creation using existing assertion helper
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

    assert_create_token_account(
        &mut context.rpc,
        token_account_pubkey,
        context.mint_pubkey,
        context.owner_keypair.pubkey(),
        Some(compressible_data),
        None,
    )
    .await;

    // Setup destination account for closure
    let (destination_keypair, _) = setup_destination_account(&mut context.rpc).await?;
    let destination_pubkey = destination_keypair.pubkey();

    // Close account using SPL SDK compatible instruction
    let close_account_ix = CloseAccount::new(
        light_compressed_token::ID,
        token_account_pubkey,
        destination_pubkey,
        context.owner_keypair.pubkey(),
    )
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(
            &[close_account_ix],
            &payer_pubkey,
            &[&context.payer, &context.owner_keypair],
        )
        .await?;

    // Verify account closure using existing assertion helper
    assert_close_token_account(
        &mut context.rpc,
        token_account_pubkey,
        context.owner_keypair.pubkey(),
        destination_pubkey,
    )
    .await;

    Ok(())
}

/// Test:
/// 1. SUCCESS: Create system account with compressible token size
/// 2. SUCCESS: Initialize compressible token account with rent authority and recipient
/// 3. SUCCESS: Verify compressible account structure using existing assertion helper
/// 4. SUCCESS: Close account using rent authority
/// 5. SUCCESS: Verify lamports transferred to rent recipient using existing assertion helper
#[tokio::test]
#[serial]
async fn test_compressible_account_with_compression_authority_lifecycle() {
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();
    let token_account_pubkey = context.token_account_keypair.pubkey();

    let payer_balance_before = context
        .rpc
        .get_account(payer_pubkey)
        .await
        .unwrap()
        .expect("Payer should exist")
        .lamports;

    // Calculate expected size for account with Compressible extension
    use light_token_interface::state::{
        calculate_token_account_size, CompressibleExtensionConfig, CompressionInfoConfig,
        ExtensionStructConfig,
    };
    let compressible_account_size =
        calculate_token_account_size(Some(&[ExtensionStructConfig::Compressible(
            CompressibleExtensionConfig {
                info: CompressionInfoConfig { rent_config: () },
            },
        )]))
        .unwrap();

    // Get rent exemption for the actual compressible account size
    let rent_exemption = context
        .rpc
        .get_minimum_balance_for_rent_exemption(compressible_account_size)
        .await
        .unwrap();

    let num_prepaid_epochs = 3; // 3 epochs for no top-up: epochs_funded_ahead = 3 - 1 = 2 >= 2
    let lamports_per_write = Some(100);

    // Initialize compressible token account
    let compressible_params = CompressibleParams {
        compressible_config: context.compressible_config,
        rent_sponsor: context.rent_sponsor,
        pre_pay_num_epochs: num_prepaid_epochs,
        lamports_per_write,
        compress_to_account_pubkey: None,
        token_account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
        compression_only: false,
    };

    let create_token_account_ix = CreateTokenAccount::new(
        payer_pubkey,
        token_account_pubkey,
        context.mint_pubkey,
        context.owner_keypair.pubkey(),
    )
    .with_compressible(compressible_params)
    .instruction()
    .map_err(|e| {
        RpcError::AssertRpcError(format!(
            "Failed to create compressible token account instruction: {}",
            e
        ))
    })
    .unwrap();
    // Verify pool PDA balance decreased by only the rent-exempt amount (not the additional rent)
    let pool_balance_before = context
        .rpc
        .get_account(context.rent_sponsor)
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
            &[&context.payer, &context.token_account_keypair],
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
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs,
            lamports_per_write,
            compress_to_pubkey: false,
            account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
            payer: payer_pubkey,
        }),
        None,
    )
    .await;

    // Verify pool PDA balance decreased by only the rent-exempt amount (not the additional rent)
    let pool_balance_after = context
        .rpc
        .get_account(context.rent_sponsor)
        .await
        .unwrap()
        .expect("Pool PDA should exist")
        .lamports;

    assert_eq!(
        pool_balance_before - pool_balance_after,
        rent_exemption,
        "Pool PDA should have paid only {} lamports for account creation (rent-exempt), not the additional rent",
        rent_exemption
    );

    // Verify payer balance decreased by exactly 11,000 lamports (the additional rent)
    let payer_balance_after = context
        .rpc
        .get_account(payer_pubkey)
        .await
        .unwrap()
        .expect("Payer should exist")
        .lamports;

    // Calculate transaction fee from the transaction result
    let tx_fee = 10_000; // Standard transaction fee
                         // Use RentConfig::default() to calculate expected rent
    let expected_additional_rent = RentConfig::default().get_rent_with_compression_cost(
        compressible_account_size as u64,
        num_prepaid_epochs as u64,
    );
    assert_eq!(
        payer_balance_before - payer_balance_after,
        expected_additional_rent + tx_fee,
        "Payer should have paid {} lamports for additional rent ({} epochs) plus {} tx fee",
        expected_additional_rent,
        num_prepaid_epochs,
        tx_fee
    );

    // TEST: Compress 0 tokens from the compressible account (edge case)
    // This tests whether compression works with an empty compressible account
    {
        // Assert expects slot to change since creation.
        context.rpc.warp_to_slot(4).unwrap();

        let output_queue = context
            .rpc
            .get_random_state_tree_info()
            .map_err(|e| RpcError::AssertRpcError(format!("Failed to get output queue: {}", e)))
            .unwrap()
            .get_output_pubkey()
            .map_err(|e| RpcError::AssertRpcError(format!("Failed to get output pubkey: {}", e)))
            .unwrap();
        println!("compressing");
        compress(
            &mut context.rpc,
            token_account_pubkey,
            0, // Compress 0 tokens for test
            context.owner_keypair.pubkey(),
            &context.owner_keypair,
            &context.payer,
            9,
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
            decimals: 9,
            version: None,
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
    let close_account_ix = CloseAccount::new(
        light_compressed_token::ID,
        token_account_pubkey,
        destination.pubkey(),           // destination for user funds
        context.owner_keypair.pubkey(), // authority
    )
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(
            &[close_account_ix],
            &payer_pubkey,
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

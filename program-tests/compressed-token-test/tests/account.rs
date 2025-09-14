// #![cfg(feature = "test-sbf")]

use anchor_spl::token_2022::spl_token_2022;
use light_compressed_token_sdk::instructions::{
    close::{close_account, close_compressible_account},
    create_associated_token_account::derive_ctoken_ata,
    create_associated_token_account_idempotent, create_token_account,
};
use light_compressible::rent::{get_rent, RentConfig, SLOTS_PER_EPOCH};
use light_ctoken_types::COMPRESSIBLE_TOKEN_ACCOUNT_SIZE;
use light_program_test::{
    forester::compress_and_close_forester, program_test::TestRpc, LightProgramTest,
    ProgramTestConfig,
};
use light_test_utils::{
    airdrop_lamports,
    assert_close_token_account::assert_close_token_account,
    assert_create_token_account::{
        assert_create_associated_token_account, assert_create_token_account, CompressibleData,
    },
    assert_transfer2::assert_transfer2_compress,
    spl::{create_mint_helper, create_token_2022_account, mint_spl_tokens},
    Rpc, RpcError,
};
use light_token_client::{
    actions::transfer2::{self, compress},
    instructions::transfer2::CompressInput,
};
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, system_instruction};
use spl_token_2022::pod::PodAccount;

/// Shared test context for account operations
struct AccountTestContext {
    pub rpc: LightProgramTest,
    pub payer: Keypair,
    pub mint_pubkey: Pubkey,
    pub owner_keypair: Keypair,
    pub token_account_keypair: Keypair,
    pub compressible_config: Pubkey,
    pub rent_sponsor: Pubkey,
    pub compression_authority: Pubkey,
}

/// Set up test environment with common accounts and context
async fn setup_account_test() -> Result<AccountTestContext, RpcError> {
    let rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();
    let mint_pubkey = Pubkey::new_unique();
    let owner_keypair = Keypair::new();
    let token_account_keypair = Keypair::new();

    Ok(AccountTestContext {
        compressible_config: rpc
            .test_accounts
            .funding_pool_config
            .compressible_config_pda,
        rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
        compression_authority: rpc
            .test_accounts
            .funding_pool_config
            .compression_authority_pda,
        rpc,
        payer,
        mint_pubkey,
        owner_keypair,
        token_account_keypair,
    })
}

/// Create destination account for testing account closure
async fn setup_destination_account(rpc: &mut LightProgramTest) -> Result<(Keypair, u64), RpcError> {
    let destination_keypair = Keypair::new();
    let destination_pubkey = destination_keypair.pubkey();

    // Fund destination account
    rpc.context
        .airdrop(&destination_pubkey, 1_000_000)
        .map_err(|_| RpcError::AssertRpcError("Failed to airdrop to destination".to_string()))?;

    let initial_lamports = rpc.get_account(destination_pubkey).await?.unwrap().lamports;

    Ok((destination_keypair, initial_lamports))
}

/// Test:
/// 1. SUCCESS: Create system account with SPL token size
/// 2. SUCCESS: Initialize basic token account using SPL SDK compatible instruction
/// 3. SUCCESS: Verify account structure and ownership using existing assertion helpers
/// 4. SUCCESS: Close account transferring lamports to destination
/// 5. SUCCESS: Verify account closure and lamport transfer using existing assertion helpers
#[tokio::test]
#[serial]
async fn test_spl_sdk_compatible_account_lifecycle() -> Result<(), RpcError> {
    let mut context = setup_account_test().await?;
    let payer_pubkey = context.payer.pubkey();
    let token_account_pubkey = context.token_account_keypair.pubkey();

    // Create system account with proper rent exemption
    let rent_exemption = context
        .rpc
        .get_minimum_balance_for_rent_exemption(165)
        .await?;

    let create_account_ix = system_instruction::create_account(
        &payer_pubkey,
        &token_account_pubkey,
        rent_exemption,
        165,
        &light_compressed_token::ID,
    );

    // Initialize token account using SPL SDK compatible instruction
    let mut initialize_account_ix = create_token_account(
        token_account_pubkey,
        context.mint_pubkey,
        context.owner_keypair.pubkey(),
    )
    .map_err(|e| {
        RpcError::AssertRpcError(format!("Failed to create token account instruction: {}", e))
    })?;
    initialize_account_ix.data.push(0);

    // Execute account creation
    context
        .rpc
        .create_and_send_transaction(
            &[create_account_ix, initialize_account_ix],
            &payer_pubkey,
            &[&context.payer, &context.token_account_keypair],
        )
        .await?;

    // Verify account creation using existing assertion helper
    assert_create_token_account(
        &mut context.rpc,
        token_account_pubkey,
        context.mint_pubkey,
        context.owner_keypair.pubkey(),
        None, // Basic token account
    )
    .await;

    // Setup destination account for closure
    let (destination_keypair, _) = setup_destination_account(&mut context.rpc).await?;
    let destination_pubkey = destination_keypair.pubkey();

    // Close account using SPL SDK compatible instruction
    let close_account_ix = close_account(
        &light_compressed_token::ID,
        &token_account_pubkey,
        &destination_pubkey,
        &context.owner_keypair.pubkey(),
    );

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
async fn test_compressible_account_with_compression_authority_lifecycle() -> Result<(), RpcError> {
    let mut context = setup_account_test().await?;
    let payer_pubkey = context.payer.pubkey();
    let token_account_pubkey = context.token_account_keypair.pubkey();

    let payer_balance_before = context
        .rpc
        .get_account(payer_pubkey)
        .await?
        .expect("Payer should exist")
        .lamports;

    // Create system account with compressible size
    let rent_exemption = context
        .rpc
        .get_minimum_balance_for_rent_exemption(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize)
        .await?;

    let num_prepaid_epochs = 1;
    let lamports_per_write = Some(100);

    // Initialize compressible token account
    let create_token_account_ix =
        light_compressed_token_sdk::instructions::create_compressible_token_account(
            light_compressed_token_sdk::instructions::CreateCompressibleTokenAccount {
                account_pubkey: token_account_pubkey,
                mint_pubkey: context.mint_pubkey,
                owner_pubkey: context.owner_keypair.pubkey(),
                compressible_config: context.compressible_config,
                rent_sponsor: context.rent_sponsor,
                pre_pay_num_epochs: num_prepaid_epochs,
                lamports_per_write,
                payer: payer_pubkey,
                compress_to_account_pubkey: None,
                token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            },
        )
        .map_err(|e| {
            RpcError::AssertRpcError(format!(
                "Failed to create compressible token account instruction: {}",
                e
            ))
        })?;
    // Verify pool PDA balance decreased by only the rent-exempt amount (not the additional rent)
    let pool_balance_before = context
        .rpc
        .get_account(context.rent_sponsor)
        .await?
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
        .await?;

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
        }),
    )
    .await;

    // Verify pool PDA balance decreased by only the rent-exempt amount (not the additional rent)
    let pool_balance_after = context
        .rpc
        .get_account(context.rent_sponsor)
        .await?
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
        .await?
        .expect("Payer should exist")
        .lamports;

    // Calculate transaction fee from the transaction result
    let tx_fee = 10_000; // Standard transaction fee
    assert_eq!(
        payer_balance_before - payer_balance_after,
        11_348 + tx_fee,
        "Payer should have paid exactly 14,830 lamports for additional rent (1 epoch) plus {} tx fee",
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
            .map_err(|e| RpcError::AssertRpcError(format!("Failed to get output queue: {}", e)))?
            .get_output_pubkey()
            .map_err(|e| RpcError::AssertRpcError(format!("Failed to get output pubkey: {}", e)))?;
        println!("compressing");
        compress(
            &mut context.rpc,
            token_account_pubkey,
            0, // Compress 0 tokens for test
            context.owner_keypair.pubkey(),
            &context.owner_keypair,
            &context.payer,
        )
        .await?;

        // Create compress input for assertion
        let compress_input = CompressInput {
            compressed_token_account: None,
            solana_token_account: token_account_pubkey,
            to: context.owner_keypair.pubkey(),
            mint: context.mint_pubkey,
            amount: 0,
            authority: context.owner_keypair.pubkey(),
            output_queue,
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
        &context.rent_sponsor,           // rent_sponsor
    );

    context
        .rpc
        .create_and_send_transaction(
            &[close_account_ix],
            &payer_pubkey,
            &[&context.owner_keypair, &context.payer],
        )
        .await?;

    // Verify account closure using existing assertion helper
    assert_close_token_account(
        &mut context.rpc,
        token_account_pubkey,
        context.owner_keypair.pubkey(),
        destination.pubkey(), // destination
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
async fn test_compressible_account_with_custom_rent_payer_close_with_owner() -> Result<(), RpcError>
{
    let mut context = setup_account_test().await?;
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
        .await?;

    let num_prepaid_epochs = 1;
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
        .map_err(|e| {
            RpcError::AssertRpcError(format!(
                "Failed to create compressible token account instruction: {}",
                e
            ))
        })?;
    // Verify pool PDA balance decreased by only the rent-exempt amount (not the additional rent)
    let pool_balance_before = context
        .rpc
        .get_account(payer_pubkey)
        .await?
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
        .await?;

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
        }),
    )
    .await;

    // Verify pool PDA balance decreased by only the rent-exempt amount (not the additional rent)

    // Verify payer balance decreased by exactly 11,000 lamports (the additional rent)
    let payer_balance_after = context
        .rpc
        .get_account(payer_pubkey)
        .await?
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
            .map_err(|e| RpcError::AssertRpcError(format!("Failed to get output queue: {}", e)))?
            .get_output_pubkey()
            .map_err(|e| RpcError::AssertRpcError(format!("Failed to get output pubkey: {}", e)))?;
        println!("compressing");
        compress(
            &mut context.rpc,
            token_account_pubkey,
            0, // Compress 0 tokens for test
            context.owner_keypair.pubkey(),
            &context.owner_keypair,
            &context.payer,
        )
        .await?;

        // Create compress input for assertion
        let compress_input = CompressInput {
            compressed_token_account: None,
            solana_token_account: token_account_pubkey,
            to: context.owner_keypair.pubkey(),
            mint: context.mint_pubkey,
            amount: 0,
            authority: context.owner_keypair.pubkey(),
            output_queue,
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
        .await?;

    // Verify account closure using existing assertion helper
    assert_close_token_account(
        &mut context.rpc,
        token_account_pubkey,
        context.owner_keypair.pubkey(),
        destination.pubkey(), // destination
    )
    .await;

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_compressible_account_with_custom_rent_payer_close_with_compression_authority(
) -> Result<(), RpcError> {
    let mut context = setup_account_test().await?;
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
        .await?;

    let num_prepaid_epochs = 1;
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
        .map_err(|e| {
            RpcError::AssertRpcError(format!(
                "Failed to create compressible token account instruction: {}",
                e
            ))
        })?;
    // Verify pool PDA balance decreased by only the rent-exempt amount (not the additional rent)
    let pool_balance_before = context
        .rpc
        .get_account(payer_pubkey)
        .await?
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
        .await?;

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
        }),
    )
    .await;

    // Verify pool PDA balance decreased by only the rent-exempt amount (not the additional rent)

    // Verify payer balance decreased by exactly 11,000 lamports (the additional rent)
    let payer_balance_after = context
        .rpc
        .get_account(payer_pubkey)
        .await?
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
            .await?
            .expect("Payer should exist")
            .lamports;
        context.rpc.warp_epoch_forward(2).await.unwrap();
        let payer_balance_after = context
            .rpc
            .get_account(payer_pubkey)
            .await?
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
    Ok(())
}

/// Test:
/// 1. SUCCESS: Create basic associated token account using SDK function
/// 2. SUCCESS: Verify basic ATA structure using existing assertion helper
/// 3. SUCCESS: Create compressible associated token account with rent authority
/// 4. SUCCESS: Verify compressible ATA structure using existing assertion helper
/// 5. SUCCESS: Close compressible ATA using rent authority
/// 6. SUCCESS: Verify lamports transferred to rent recipient using existing assertion helper
#[tokio::test]
#[serial]
async fn test_associated_token_account_operations() -> Result<(), RpcError> {
    let mut context = setup_account_test().await?;
    let payer_pubkey = context.payer.pubkey();
    let owner_pubkey = context.owner_keypair.pubkey();

    // Create basic ATA using SDK function
    let instruction = light_compressed_token_sdk::instructions::create_associated_token_account(
        payer_pubkey,
        owner_pubkey,
        context.mint_pubkey,
    )
    .map_err(|e| RpcError::AssertRpcError(format!("Failed to create ATA instruction: {}", e)))?;

    context
        .rpc
        .create_and_send_transaction(&[instruction], &payer_pubkey, &[&context.payer])
        .await?;

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
    ).map_err(|e| RpcError::AssertRpcError(format!("Failed to create compressible ATA instruction: {}", e)))?;

    context
        .rpc
        .create_and_send_transaction(
            &[compressible_instruction],
            &payer_pubkey,
            &[&context.payer],
        )
        .await?;

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
        .await?;

    // Verify compressible ATA closure using existing assertion helper
    assert_close_token_account(
        &mut context.rpc,
        compressible_ata_pubkey,
        compressible_owner_keypair.pubkey(),
        destination.pubkey(), // destination
    )
    .await;

    Ok(())
}

/// Test compress_and_close with rent authority:
/// 1. Create compressible token account with rent authority
/// 2. Compress and close account using rent authority
/// 3. Verify rent goes to rent recipient
#[tokio::test]
#[serial]
async fn test_compress_and_close_with_compression_authority() -> Result<(), RpcError> {
    let mut context = setup_account_test().await?;
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
                pre_pay_num_epochs: 1,
                lamports_per_write: Some(150),
                payer: payer_pubkey,
                compress_to_account_pubkey: None,
                token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            },
        )
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {}", e)))?;

    context
        .rpc
        .create_and_send_transaction(
            &[create_token_account_ix],
            &payer_pubkey,
            &[&context.payer, &context.token_account_keypair],
        )
        .await?;

    // Top up rent for one more epoch
    context
        .rpc
        .airdrop_lamports(
            &token_account_pubkey,
            get_rent(
                RentConfig::default().base_rent as u64,
                RentConfig::default().lamports_per_byte_per_epoch as u64,
                COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
                1,
            ),
        )
        .await
        .unwrap();

    // Advance to epoch 1 to make the account compressible
    // Account was created with 0 epochs of rent prepaid, so it's instantly compressible
    // But we still need to advance time to trigger the rent authority logic
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
    // Advance to epoch 1 to make the account compressible
    // Account was created with 0 epochs of rent prepaid, so it's instantly compressible
    // But we still need to advance time to trigger the rent authority logic
    context.rpc.warp_to_slot((SLOTS_PER_EPOCH * 2) + 1).unwrap();

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
        },
    )
    .await;

    Ok(())
}

/// Test:
/// 1. SUCCESS: Create ATA using non-idempotent instruction
/// 2. FAIL: Attempt to create same ATA again using non-idempotent instruction (should fail)
/// 3. SUCCESS: Create same ATA using idempotent instruction (should succeed)
#[tokio::test]
#[serial]
async fn test_create_ata_idempotent() -> Result<(), RpcError> {
    let mut context = setup_account_test().await?;
    let payer_pubkey = context.payer.pubkey();
    let owner_pubkey = context.owner_keypair.pubkey();
    // Create ATA using non-idempotent instruction (first creation)
    let instruction = light_compressed_token_sdk::instructions::create_associated_token_account::create_associated_token_account(
        payer_pubkey,
        owner_pubkey,
        context.mint_pubkey,
    )
    .map_err(|e| {
        RpcError::AssertRpcError(format!("Failed to create non-idempotent ATA instruction: {}", e))
    })?;

    context
        .rpc
        .create_and_send_transaction(&[instruction], &payer_pubkey, &[&context.payer])
        .await?;

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
    .map_err(|e| {
        RpcError::AssertRpcError(format!("Failed to create non-idempotent ATA instruction: {}", e))
    })?;

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
            .map_err(|e| {
                RpcError::AssertRpcError(format!(
                    "Failed to create idempotent ATA instruction: {}",
                    e
                ))
            })?;

    context
        .rpc
        .create_and_send_transaction(&[instruction], &payer_pubkey, &[&context.payer])
        .await
        .map_err(|e| {
            RpcError::AssertRpcError(format!(
                "Idempotent ATA creation should succeed even when account exists: {}",
                e
            ))
        })?;

    // Verify ATA is still correct
    assert_create_associated_token_account(
        &mut context.rpc,
        owner_pubkey,
        context.mint_pubkey,
        None,
    )
    .await;
    Ok(())
}

#[tokio::test]
async fn test_spl_to_ctoken_transfer() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(true, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let sender = Keypair::new();
    airdrop_lamports(&mut rpc, &sender.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let mint = create_mint_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    let transfer_amount = 5000u64;

    // Create SPL token account and mint tokens
    let spl_token_account_keypair = Keypair::new();
    create_token_2022_account(&mut rpc, &mint, &spl_token_account_keypair, &sender, false)
        .await
        .unwrap();
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &spl_token_account_keypair.pubkey(),
        &payer.pubkey(),
        &payer,
        amount,
        false,
    )
    .await
    .unwrap();
    println!(
        "spl_token_account_keypair {:?}",
        spl_token_account_keypair.pubkey()
    );
    // Create recipient for compressed tokens
    let recipient = Keypair::new();
    airdrop_lamports(&mut rpc, &recipient.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // Create compressed token ATA for recipient
    let instruction = light_compressed_token_sdk::instructions::create_associated_token_account(
        payer.pubkey(),
        recipient.pubkey(),
        mint,
    )
    .map_err(|e| RpcError::AssertRpcError(format!("Failed to create ATA instruction: {}", e)))?;
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await?;
    let associated_token_account = derive_ctoken_ata(&recipient.pubkey(), &mint).0;

    // Get initial SPL token balance
    let spl_account_data = rpc
        .get_account(spl_token_account_keypair.pubkey())
        .await?
        .unwrap();
    let spl_account = spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data)
        .map_err(|e| {
            RpcError::AssertRpcError(format!("Failed to parse SPL token account: {}", e))
        })?;
    let initial_spl_balance: u64 = spl_account.amount.into();
    assert_eq!(initial_spl_balance, amount);

    // Use the new spl_to_ctoken_transfer action from light-token-client
    transfer2::spl_to_ctoken_transfer(
        &mut rpc,
        spl_token_account_keypair.pubkey(),
        associated_token_account,
        transfer_amount,
        &sender,
        &payer,
    )
    .await?;

    {
        // Verify SPL token balance decreased
        let spl_account_data = rpc
            .get_account(spl_token_account_keypair.pubkey())
            .await?
            .unwrap();
        let spl_account = spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data)
            .map_err(|e| {
            RpcError::AssertRpcError(format!("Failed to parse SPL token account: {}", e))
        })?;
        let final_spl_balance: u64 = spl_account.amount.into();
        assert_eq!(final_spl_balance, amount - transfer_amount);
    }
    {
        // Verify compressed token balance increased
        let spl_account_data = rpc.get_account(associated_token_account).await?.unwrap();
        let spl_account =
            spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data[..165])
                .map_err(|e| {
                    RpcError::AssertRpcError(format!("Failed to parse SPL token account: {}", e))
                })?;
        assert_eq!(
            u64::from(spl_account.amount),
            transfer_amount,
            "Recipient should have {} compressed tokens",
            transfer_amount
        );
    }

    // Now transfer back from compressed token to SPL token account
    println!("Testing reverse transfer: ctoken to SPL");

    // Transfer from recipient's compressed token account back to sender's SPL token account
    transfer2::ctoken_to_spl_transfer(
        &mut rpc,
        associated_token_account,
        spl_token_account_keypair.pubkey(),
        transfer_amount,
        &recipient,
        mint,
        &payer,
        SPL_TOKEN_PROGRAM_ID.into(),
    )
    .await?;

    // Verify final balances
    {
        // Verify SPL token balance is restored
        let spl_account_data = rpc
            .get_account(spl_token_account_keypair.pubkey())
            .await?
            .unwrap();
        let spl_account = spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data)
            .map_err(|e| {
            RpcError::AssertRpcError(format!("Failed to parse SPL token account: {}", e))
        })?;
        let restored_spl_balance: u64 = spl_account.amount.into();
        assert_eq!(
            restored_spl_balance, amount,
            "SPL token balance should be restored to original amount"
        );
    }

    {
        // Verify compressed token balance is now 0
        let ctoken_account_data = rpc.get_account(associated_token_account).await?.unwrap();
        let ctoken_account =
            spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&ctoken_account_data.data[..165])
                .map_err(|e| {
                    RpcError::AssertRpcError(format!(
                        "Failed to parse compressed token account: {}",
                        e
                    ))
                })?;
        assert_eq!(
            u64::from(ctoken_account.amount),
            0,
            "Compressed token account should be empty after transfer back"
        );
    }

    println!("Successfully completed round-trip transfer: SPL -> CToken -> SPL");

    Ok(())
}

// #![cfg(feature = "test-sbf")]

use light_compressed_token_sdk::instructions::{
    close::close_account, create_associated_token_account::derive_ctoken_ata, create_token_account,
};
use light_ctoken_types::COMPRESSIBLE_TOKEN_ACCOUNT_SIZE;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    assert_close_token_account::assert_close_token_account,
    assert_create_token_account::{
        assert_create_associated_token_account, assert_create_token_account, CompressibleData,
    },
    Rpc, RpcError,
};
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, system_instruction};

/// Shared test context for account operations
struct AccountTestContext {
    pub rpc: LightProgramTest,
    pub payer: Keypair,
    pub mint_pubkey: Pubkey,
    pub owner_keypair: Keypair,
    pub token_account_keypair: Keypair,
}

/// Set up test environment with common accounts and context
async fn setup_account_test() -> Result<AccountTestContext, RpcError> {
    let rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();
    let mint_pubkey = Pubkey::new_unique();
    let owner_keypair = Keypair::new();
    let token_account_keypair = Keypair::new();

    Ok(AccountTestContext {
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
    let (destination_keypair, initial_destination_lamports) =
        setup_destination_account(&mut context.rpc).await?;
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
        None,
        destination_pubkey,
        initial_destination_lamports,
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
async fn test_compressible_account_with_rent_authority_lifecycle() -> Result<(), RpcError> {
    let mut context = setup_account_test().await?;
    let payer_pubkey = context.payer.pubkey();
    let token_account_pubkey = context.token_account_keypair.pubkey();

    // Create rent authority and recipient
    let rent_authority_keypair = Keypair::new();
    let rent_authority_pubkey = rent_authority_keypair.pubkey();
    let rent_recipient_keypair = Keypair::new();
    let rent_recipient_pubkey = rent_recipient_keypair.pubkey();

    // Fund rent recipient
    context
        .rpc
        .context
        .airdrop(&rent_recipient_pubkey, 1_000_000)
        .map_err(|_| RpcError::AssertRpcError("Failed to airdrop to rent recipient".to_string()))?;

    // Create system account with compressible size
    let rent_exemption = context
        .rpc
        .get_minimum_balance_for_rent_exemption(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize)
        .await?;

    let create_account_ix = system_instruction::create_account(
        &payer_pubkey,
        &token_account_pubkey,
        rent_exemption,
        COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
        &light_compressed_token::ID,
    );

    // Initialize compressible token account
    let create_token_account_ix =
        light_compressed_token_sdk::instructions::create_compressible_token_account(
            light_compressed_token_sdk::instructions::CreateCompressibleTokenAccount {
                account_pubkey: token_account_pubkey,
                mint_pubkey: context.mint_pubkey,
                owner_pubkey: context.owner_keypair.pubkey(),
                rent_authority: rent_authority_pubkey,
                rent_recipient: rent_recipient_pubkey,
                slots_until_compression: 0,
            },
        )
        .map_err(|e| {
            RpcError::AssertRpcError(format!(
                "Failed to create compressible token account instruction: {}",
                e
            ))
        })?;

    // Execute account creation
    context
        .rpc
        .create_and_send_transaction(
            &[create_account_ix, create_token_account_ix],
            &payer_pubkey,
            &[&context.payer, &context.token_account_keypair],
        )
        .await?;

    // Verify compressible account creation using existing assertion helper
    assert_create_token_account(
        &mut context.rpc,
        token_account_pubkey,
        context.mint_pubkey,
        context.owner_keypair.pubkey(),
        Some(CompressibleData {
            rent_authority: rent_authority_pubkey,
            rent_recipient: rent_recipient_pubkey,
            slots_until_compression: 0,
        }),
    )
    .await;

    // Get initial recipient lamports before closing
    let initial_recipient_lamports = context
        .rpc
        .get_account(rent_recipient_pubkey)
        .await?
        .unwrap()
        .lamports;

    // Get account data before closing for assertion helper
    let account_data_before_close = context
        .rpc
        .get_account(token_account_pubkey)
        .await?
        .unwrap()
        .data;

    // Close account using rent authority
    let close_account_ix = close_account(
        &light_compressed_token::ID,
        &token_account_pubkey,
        &rent_recipient_pubkey,
        &rent_authority_pubkey,
    );

    context
        .rpc
        .create_and_send_transaction(
            &[close_account_ix],
            &payer_pubkey,
            &[&context.payer, &rent_authority_keypair],
        )
        .await?;

    // Verify account closure using existing assertion helper
    assert_close_token_account(
        &mut context.rpc,
        token_account_pubkey,
        Some(&account_data_before_close),
        rent_recipient_pubkey,
        initial_recipient_lamports,
    )
    .await;

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
    let rent_authority_keypair = Keypair::new();
    let rent_authority_pubkey = rent_authority_keypair.pubkey();
    let rent_recipient_keypair = Keypair::new();
    let rent_recipient_pubkey = rent_recipient_keypair.pubkey();

    // Fund rent recipient
    context
        .rpc
        .context
        .airdrop(&rent_recipient_pubkey, 1_000_000)
        .map_err(|_| RpcError::AssertRpcError("Failed to airdrop to rent recipient".to_string()))?;

    // Create compressible ATA
    let compressible_instruction = light_compressed_token_sdk::instructions::create_compressible_associated_token_account(
        light_compressed_token_sdk::instructions::CreateCompressibleAssociatedTokenAccountInputs {
            payer: payer_pubkey,
            owner: compressible_owner_pubkey,
            mint: context.mint_pubkey,
            rent_authority: rent_authority_pubkey,
            rent_recipient: rent_recipient_pubkey,
            slots_until_compression: 0,
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
            rent_authority: rent_authority_pubkey,
            rent_recipient: rent_recipient_pubkey,
            slots_until_compression: 0,
        }),
    )
    .await;

    // Test closing compressible ATA
    let (compressible_ata_pubkey, _) =
        derive_ctoken_ata(&compressible_owner_pubkey, &context.mint_pubkey);

    let initial_recipient_lamports = context
        .rpc
        .get_account(rent_recipient_pubkey)
        .await?
        .unwrap()
        .lamports;

    // Get account data before closing for assertion helper
    let account_data_before_close = context
        .rpc
        .get_account(compressible_ata_pubkey)
        .await?
        .unwrap()
        .data;

    // Close compressible ATA
    let close_account_ix = close_account(
        &light_compressed_token::ID,
        &compressible_ata_pubkey,
        &rent_recipient_pubkey,
        &rent_authority_pubkey,
    );

    context
        .rpc
        .create_and_send_transaction(
            &[close_account_ix],
            &payer_pubkey,
            &[&context.payer, &rent_authority_keypair],
        )
        .await?;

    // Verify compressible ATA closure using existing assertion helper
    assert_close_token_account(
        &mut context.rpc,
        compressible_ata_pubkey,
        Some(&account_data_before_close),
        rent_recipient_pubkey,
        initial_recipient_lamports,
    )
    .await;

    Ok(())
}

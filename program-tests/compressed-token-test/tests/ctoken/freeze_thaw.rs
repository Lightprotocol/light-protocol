//! Tests for CToken freeze and thaw instructions
//!
//! These tests verify that freeze and thaw instructions work correctly
//! for both basic mints and Token-2022 mints with extensions.

use anchor_lang::AnchorDeserialize;
use light_ctoken_interface::state::{AccountState, CToken, TokenDataVersion};
use light_ctoken_sdk::ctoken::{CompressibleParams, CreateCTokenAccount, FreezeCToken, ThawCToken};
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    assert_ctoken_freeze_thaw::{assert_ctoken_freeze, assert_ctoken_thaw},
    spl::create_mint_helper,
    Rpc, RpcError,
};
use serial_test::serial;
use solana_sdk::{signature::Keypair, signer::Signer};

use super::extensions::setup_extensions_test;

/// Test freeze and thaw with a basic SPL Token mint (not Token-2022)
/// Uses create_mint_helper which creates a mint with freeze_authority = payer
#[tokio::test]
#[serial]
async fn test_freeze_thaw_with_basic_mint() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();
    let owner = Keypair::new();

    // 1. Create SPL Token mint with freeze_authority = payer
    let mint_pubkey = create_mint_helper(&mut rpc, &payer).await;

    // 2. Create CToken account with 0 prepaid epochs (immediately compressible)
    let token_account_keypair = Keypair::new();
    let token_account_pubkey = token_account_keypair.pubkey();

    let compressible_params = CompressibleParams {
        compressible_config: rpc
            .test_accounts
            .funding_pool_config
            .compressible_config_pda,
        rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
        pre_pay_num_epochs: 0,
        lamports_per_write: None,
        compress_to_account_pubkey: None,
        token_account_version: TokenDataVersion::ShaFlat,
        compression_only: false,
    };

    let create_ix = CreateCTokenAccount::new(
        payer.pubkey(),
        token_account_pubkey,
        mint_pubkey,
        owner.pubkey(),
    )
    .with_compressible(compressible_params)
    .instruction()
    .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {}", e)))?;

    rpc.create_and_send_transaction(
        &[create_ix],
        &payer.pubkey(),
        &[&payer, &token_account_keypair],
    )
    .await?;

    // Verify initial state is Initialized
    let account_data = rpc.get_account(token_account_pubkey).await?.unwrap();
    let ctoken_before =
        CToken::deserialize(&mut &account_data.data[..]).expect("Failed to deserialize CToken");
    assert_eq!(
        ctoken_before.state,
        AccountState::Initialized,
        "Initial state should be Initialized"
    );

    // 3. Freeze the account
    let freeze_ix = FreezeCToken {
        token_account: token_account_pubkey,
        mint: mint_pubkey,
        freeze_authority: payer.pubkey(),
    }
    .instruction()
    .map_err(|e| RpcError::AssertRpcError(format!("Failed to create freeze instruction: {}", e)))?;

    rpc.create_and_send_transaction(&[freeze_ix], &payer.pubkey(), &[&payer])
        .await?;

    // 4. Assert state is Frozen
    assert_ctoken_freeze(&mut rpc, token_account_pubkey).await;

    // 5. Thaw the account
    let thaw_ix = ThawCToken {
        token_account: token_account_pubkey,
        mint: mint_pubkey,
        freeze_authority: payer.pubkey(),
    }
    .instruction()
    .map_err(|e| RpcError::AssertRpcError(format!("Failed to create thaw instruction: {}", e)))?;

    rpc.create_and_send_transaction(&[thaw_ix], &payer.pubkey(), &[&payer])
        .await?;

    // 6. Assert state is Initialized again
    assert_ctoken_thaw(&mut rpc, token_account_pubkey).await;

    println!("Successfully tested freeze and thaw with basic mint");
    Ok(())
}

/// Test freeze and thaw with a Token-2022 mint that has all extensions
/// Verifies that extensions are preserved through freeze/thaw cycle
#[tokio::test]
#[serial]
async fn test_freeze_thaw_with_extensions() -> Result<(), RpcError> {
    let mut context = setup_extensions_test().await?;
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;
    let owner = Keypair::new();

    // 1. Create compressible CToken account with all extensions
    let account_keypair = Keypair::new();
    let account_pubkey = account_keypair.pubkey();

    let create_ix =
        CreateCTokenAccount::new(payer.pubkey(), account_pubkey, mint_pubkey, owner.pubkey())
            .with_compressible(CompressibleParams {
                compressible_config: context
                    .rpc
                    .test_accounts
                    .funding_pool_config
                    .compressible_config_pda,
                rent_sponsor: context
                    .rpc
                    .test_accounts
                    .funding_pool_config
                    .rent_sponsor_pda,
                pre_pay_num_epochs: 2,
                lamports_per_write: Some(100),
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: true,
            })
            .instruction()
            .map_err(|e| {
                RpcError::AssertRpcError(format!("Failed to create instruction: {}", e))
            })?;

    context
        .rpc
        .create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer, &account_keypair])
        .await?;

    // Verify account was created with correct size (275 bytes with all extensions)
    let account_data_initial = context.rpc.get_account(account_pubkey).await?.unwrap();
    assert_eq!(
        account_data_initial.data.len(),
        275,
        "CToken account should be 275 bytes with all extensions"
    );

    // Deserialize and verify initial state
    let ctoken_initial = CToken::deserialize(&mut &account_data_initial.data[..])
        .expect("Failed to deserialize CToken");
    assert_eq!(
        ctoken_initial.state,
        AccountState::Initialized,
        "Initial state should be Initialized"
    );

    // 2. Freeze the account
    let freeze_ix = FreezeCToken {
        token_account: account_pubkey,
        mint: mint_pubkey,
        freeze_authority: payer.pubkey(),
    }
    .instruction()
    .map_err(|e| RpcError::AssertRpcError(format!("Failed to create freeze instruction: {}", e)))?;

    context
        .rpc
        .create_and_send_transaction(&[freeze_ix], &payer.pubkey(), &[&payer])
        .await?;

    // 3. Assert state is Frozen with all extensions preserved
    assert_ctoken_freeze(&mut context.rpc, account_pubkey).await;

    // 4. Thaw the account
    let thaw_ix = ThawCToken {
        token_account: account_pubkey,
        mint: mint_pubkey,
        freeze_authority: payer.pubkey(),
    }
    .instruction()
    .map_err(|e| RpcError::AssertRpcError(format!("Failed to create thaw instruction: {}", e)))?;

    context
        .rpc
        .create_and_send_transaction(&[thaw_ix], &payer.pubkey(), &[&payer])
        .await?;

    // 5. Assert state is Initialized again with all extensions preserved
    assert_ctoken_thaw(&mut context.rpc, account_pubkey).await;

    println!("Successfully tested freeze and thaw with Token-2022 extensions");
    Ok(())
}

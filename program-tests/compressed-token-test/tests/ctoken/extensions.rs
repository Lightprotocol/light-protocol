//! Tests for Token 2022 mint with multiple extensions
//!
//! This module tests the creation and verification of Token 2022 mints
//! with all supported extensions.

use light_client::indexer::Indexer;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    assert_transfer2::assert_transfer2_compress,
    mint_2022::{
        create_mint_22_with_extensions, create_token_22_account, mint_spl_tokens_22,
        verify_mint_extensions, Token22ExtensionConfig,
    },
    Rpc, RpcError,
};
use light_token_client::instructions::transfer2::{
    create_generic_transfer2_instruction, CompressInput, Transfer2InstructionType,
};
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

/// Test context for extension-related tests
pub struct ExtensionsTestContext {
    pub rpc: LightProgramTest,
    pub payer: Keypair,
    pub _mint_keypair: Keypair,
    pub mint_pubkey: Pubkey,
    pub extension_config: Token22ExtensionConfig,
}

/// Set up test environment with a Token 2022 mint with all extensions
pub async fn setup_extensions_test() -> Result<ExtensionsTestContext, RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();

    // Create mint with all extensions
    let (mint_keypair, extension_config) =
        create_mint_22_with_extensions(&mut rpc, &payer, 9).await;

    let mint_pubkey = mint_keypair.pubkey();

    Ok(ExtensionsTestContext {
        rpc,
        payer,
        _mint_keypair: mint_keypair,
        mint_pubkey,
        extension_config,
    })
}

#[tokio::test]
#[serial]
async fn test_setup_mint_22_with_all_extensions() {
    let mut context = setup_extensions_test().await.unwrap();

    // Verify all extensions are present
    verify_mint_extensions(&mut context.rpc, &context.mint_pubkey)
        .await
        .unwrap();

    // Verify the extension config has correct values
    assert_eq!(context.extension_config.mint, context.mint_pubkey);

    // Verify token pool was created
    let token_pool_account = context
        .rpc
        .get_account(context.extension_config.token_pool)
        .await
        .unwrap();
    assert!(
        token_pool_account.is_some(),
        "Token pool account should exist"
    );

    assert_eq!(
        context.extension_config.close_authority,
        context.payer.pubkey()
    );
    assert_eq!(
        context.extension_config.transfer_fee_config_authority,
        context.payer.pubkey()
    );
    assert_eq!(
        context.extension_config.withdraw_withheld_authority,
        context.payer.pubkey()
    );
    assert_eq!(
        context.extension_config.permanent_delegate,
        context.payer.pubkey()
    );
    assert_eq!(
        context.extension_config.metadata_update_authority,
        context.payer.pubkey()
    );
    assert_eq!(
        context.extension_config.pause_authority,
        context.payer.pubkey()
    );
    assert_eq!(
        context.extension_config.confidential_transfer_authority,
        context.payer.pubkey()
    );
    assert_eq!(
        context.extension_config.confidential_transfer_fee_authority,
        context.payer.pubkey()
    );

    println!(
        "Mint with all extensions created successfully: {}",
        context.mint_pubkey
    );
}

/// Test minting SPL tokens and compressing them with a Token 2022 mint with all extensions
#[tokio::test]
#[serial]
async fn test_mint_and_compress_with_extensions() {
    let mut context = setup_extensions_test().await.unwrap();
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    // 1. Create a Token 2022 token account for the payer
    let token_account =
        create_token_22_account(&mut context.rpc, &payer, &mint_pubkey, &payer.pubkey()).await;

    println!("Created token account: {}", token_account);

    // 2. Mint SPL tokens to the token account
    let mint_amount = 1_000_000_000u64; // 1 token with 9 decimals
    mint_spl_tokens_22(
        &mut context.rpc,
        &payer,
        &mint_pubkey,
        &token_account,
        mint_amount,
    )
    .await;

    println!("Minted {} tokens to {}", mint_amount, token_account);

    // 3. Compress the tokens using transfer2
    let compress_amount = 500_000_000u64; // Compress half
    let compress_recipient = Keypair::new();
    let output_queue = context.rpc.get_random_state_tree_info().unwrap().queue;

    let compress_instruction = create_generic_transfer2_instruction(
        &mut context.rpc,
        vec![Transfer2InstructionType::Compress(CompressInput {
            compressed_token_account: None, // No existing compressed tokens
            solana_token_account: token_account,
            to: compress_recipient.pubkey(),
            mint: mint_pubkey,
            amount: compress_amount,
            authority: payer.pubkey(),
            output_queue,
            pool_index: None,
        })],
        payer.pubkey(),
        true, // is_token_22
    )
    .await
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[compress_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify compression succeeded
    assert_transfer2_compress(
        &mut context.rpc,
        CompressInput {
            compressed_token_account: None,
            solana_token_account: token_account,
            to: compress_recipient.pubkey(),
            mint: mint_pubkey,
            amount: compress_amount,
            authority: payer.pubkey(),
            output_queue,
            pool_index: None,
        },
    )
    .await;

    // Verify the recipient has compressed tokens
    let recipient_accounts = context
        .rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&compress_recipient.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert!(
        !recipient_accounts.is_empty(),
        "Recipient should have compressed tokens"
    );
    assert_eq!(
        recipient_accounts[0].token.amount, compress_amount,
        "Compressed token amount should match"
    );
    println!(" recipient_accounts[0] {:?}", recipient_accounts[0]);
    println!(
        "Successfully compressed {} tokens to {}",
        compress_amount,
        compress_recipient.pubkey()
    );
}

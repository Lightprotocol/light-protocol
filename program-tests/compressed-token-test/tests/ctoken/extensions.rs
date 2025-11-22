//! Tests for Token 2022 mint with multiple extensions
//!
//! This module tests the creation and verification of Token 2022 mints
//! with all supported extensions.

use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    mint_2022::{create_mint_22_with_extensions, verify_mint_extensions, Token22ExtensionConfig},
    Rpc, RpcError,
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

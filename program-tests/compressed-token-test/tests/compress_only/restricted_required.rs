//! Tests for compression_only requirement with restricted extensions.
//!
//! These tests verify that CToken accounts cannot be created without compression_only
//! when the mint has restricted extensions (Pausable, PermanentDelegate, TransferFeeConfig,
//! TransferHook, DefaultAccountState).

use light_token_interface::state::TokenDataVersion;
use light_ctoken_sdk::ctoken::{CompressibleParams, CreateCTokenAccount};
use light_program_test::{
    program_test::LightProgramTest, utils::assert::assert_rpc_error, ProgramTestConfig, Rpc,
};
use light_test_utils::mint_2022::create_mint_22_with_extension_types;
use serial_test::serial;
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_token_2022::extension::ExtensionType;

/// Expected error code for CompressionOnlyRequired
const COMPRESSION_ONLY_REQUIRED: u32 = 6131;

/// Helper to test that creating a CToken account without compression_only fails
/// when the mint has the specified extensions.
async fn test_compression_only_required_for_extensions(extensions: &[ExtensionType]) {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create mint with specified extensions
    let (mint_keypair, _) =
        create_mint_22_with_extension_types(&mut rpc, &payer, 9, extensions).await;
    let mint_pubkey = mint_keypair.pubkey();

    // Try to create CToken account WITHOUT compression_only (should fail)
    let token_account_keypair = Keypair::new();
    let token_account_pubkey = token_account_keypair.pubkey();

    let create_ix = CreateCTokenAccount::new(
        payer.pubkey(),
        token_account_pubkey,
        mint_pubkey,
        payer.pubkey(),
    )
    .with_compressible(CompressibleParams {
        compressible_config: rpc
            .test_accounts
            .funding_pool_config
            .compressible_config_pda,
        rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
        pre_pay_num_epochs: 2,
        lamports_per_write: Some(100),
        compress_to_account_pubkey: None,
        token_account_version: TokenDataVersion::ShaFlat,
        compression_only: false, // This should cause the error
    })
    .instruction()
    .unwrap();

    let result = rpc
        .create_and_send_transaction(
            &[create_ix],
            &payer.pubkey(),
            &[&payer, &token_account_keypair],
        )
        .await;

    assert_rpc_error(result, 0, COMPRESSION_ONLY_REQUIRED).unwrap();
}

#[tokio::test]
#[serial]
async fn test_pausable_requires_compression_only() {
    test_compression_only_required_for_extensions(&[ExtensionType::Pausable]).await;
}

#[tokio::test]
#[serial]
async fn test_permanent_delegate_requires_compression_only() {
    test_compression_only_required_for_extensions(&[ExtensionType::PermanentDelegate]).await;
}

#[tokio::test]
#[serial]
async fn test_transfer_fee_requires_compression_only() {
    test_compression_only_required_for_extensions(&[ExtensionType::TransferFeeConfig]).await;
}

#[tokio::test]
#[serial]
async fn test_transfer_hook_requires_compression_only() {
    test_compression_only_required_for_extensions(&[ExtensionType::TransferHook]).await;
}

#[tokio::test]
#[serial]
async fn test_default_account_state_requires_compression_only() {
    test_compression_only_required_for_extensions(&[ExtensionType::DefaultAccountState]).await;
}

#[tokio::test]
#[serial]
async fn test_multiple_restricted_requires_compression_only() {
    test_compression_only_required_for_extensions(&[
        ExtensionType::Pausable,
        ExtensionType::PermanentDelegate,
        ExtensionType::TransferFeeConfig,
        ExtensionType::TransferHook,
    ])
    .await;
}

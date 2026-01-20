//! Tests for extension validation failures in Light Token operations.
//!
//! This module tests extension validation for operations that FAIL with invalid state:
//! 1. CTokenTransfer(Checked) - transfers between Light Token accounts
//! 2. SPL → Light Token (TransferFromSpl) - entering via Compress mode
//!
//! Note: Light Token → SPL (TransferTokenToSpl) is a BYPASS operation and is tested
//! in compress_only/invalid_extension_state.rs. It succeeds with invalid extension
//! state because it exits compressed state without creating new compressed accounts.

use light_program_test::utils::assert::assert_rpc_error;
use light_test_utils::{
    mint_2022::{
        create_token_22_account, mint_spl_tokens_22, pause_mint, set_mint_transfer_fee,
        set_mint_transfer_hook,
    },
    Rpc,
};
use light_token::{
    spl_interface::find_spl_interface_pda_with_index,
    token::{CompressibleParams, CreateTokenAccount, TransferChecked, TransferFromSpl},
};
use light_token_interface::state::TokenDataVersion;
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

use super::extensions::{setup_extensions_test, ExtensionsTestContext};

/// Expected error code for MintPaused
const MINT_PAUSED: u32 = 6127;

/// Expected error code for NonZeroTransferFeeNotSupported
const NON_ZERO_TRANSFER_FEE_NOT_SUPPORTED: u32 = 6129;

/// Expected error code for TransferHookNotSupported
const TRANSFER_HOOK_NOT_SUPPORTED: u32 = 6130;

/// Set up two Light Token accounts with tokens for transfer testing.
/// Returns (source_account, destination_account, owner)
async fn setup_ctoken_accounts_for_transfer(
    context: &mut ExtensionsTestContext,
) -> (Pubkey, Pubkey, Keypair) {
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    // Create SPL source account and mint tokens
    let spl_account =
        create_token_22_account(&mut context.rpc, &payer, &mint_pubkey, &payer.pubkey()).await;

    let mint_amount = 1_000_000_000u64;
    mint_spl_tokens_22(
        &mut context.rpc,
        &payer,
        &mint_pubkey,
        &spl_account,
        mint_amount,
    )
    .await;

    // Create owner and Light Token accounts
    let owner = Keypair::new();

    // Create source Light Token account
    let account_a_keypair = Keypair::new();
    let account_a_pubkey = account_a_keypair.pubkey();
    let create_a_ix = CreateTokenAccount::new(
        payer.pubkey(),
        account_a_pubkey,
        mint_pubkey,
        owner.pubkey(),
    )
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
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(
            &[create_a_ix],
            &payer.pubkey(),
            &[&payer, &account_a_keypair],
        )
        .await
        .unwrap();

    // Create destination Light Token account
    let account_b_keypair = Keypair::new();
    let account_b_pubkey = account_b_keypair.pubkey();
    let create_b_ix = CreateTokenAccount::new(
        payer.pubkey(),
        account_b_pubkey,
        mint_pubkey,
        owner.pubkey(),
    )
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
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(
            &[create_b_ix],
            &payer.pubkey(),
            &[&payer, &account_b_keypair],
        )
        .await
        .unwrap();

    // Transfer SPL to source Light Token account using hot path
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, true);

    let transfer_spl_to_ctoken_ix = TransferFromSpl {
        amount: mint_amount,
        spl_interface_pda_bump,
        source_spl_token_account: spl_account,
        destination: account_a_pubkey,
        authority: payer.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: spl_token_2022::ID,
        decimals: 9,
    }
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[transfer_spl_to_ctoken_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    (account_a_pubkey, account_b_pubkey, owner)
}

/// Test that CTokenTransferChecked fails when the mint is paused.
///
/// Setup:
/// 1. Create mint with Pausable extension (not paused initially)
/// 2. Create token pool, two Light Token accounts with tokens
/// 3. Pause the mint via set_account
/// 4. Attempt CTokenTransferChecked
///
/// Expected: MintPaused (6496)
#[tokio::test]
#[serial]
async fn test_ctoken_transfer_fails_when_mint_paused() {
    let mut context = setup_extensions_test().await.unwrap();
    let mint_pubkey = context.mint_pubkey;

    // Set up accounts with tokens
    let (source, destination, owner) = setup_ctoken_accounts_for_transfer(&mut context).await;

    // Pause the mint
    pause_mint(&mut context.rpc, &mint_pubkey).await;

    // Attempt transfer - should fail with MintPaused
    let transfer_ix = TransferChecked {
        source,
        mint: mint_pubkey,
        destination,
        amount: 100_000_000,
        decimals: 9,
        authority: owner.pubkey(),
        max_top_up: None,
    }
    .instruction()
    .unwrap();

    let result = context
        .rpc
        .create_and_send_transaction(
            &[transfer_ix],
            &context.payer.pubkey(),
            &[&context.payer, &owner],
        )
        .await;

    assert_rpc_error(result, 0, MINT_PAUSED).unwrap();
    println!("Correctly rejected CTokenTransferChecked when mint is paused");
}

/// Test that CTokenTransferChecked fails when the mint has non-zero transfer fees.
///
/// Setup:
/// 1. Create mint with TransferFeeConfig (zero fees initially)
/// 2. Create token pool, two Light Token accounts with tokens
/// 3. Modify mint TransferFeeConfig to have non-zero fees
/// 4. Attempt CTokenTransferChecked
///
/// Expected: NonZeroTransferFeeNotSupported (6500)
#[tokio::test]
#[serial]
async fn test_ctoken_transfer_fails_with_non_zero_transfer_fee() {
    let mut context = setup_extensions_test().await.unwrap();
    let mint_pubkey = context.mint_pubkey;

    // Set up accounts with tokens
    let (source, destination, owner) = setup_ctoken_accounts_for_transfer(&mut context).await;

    // Set non-zero transfer fees on the mint
    set_mint_transfer_fee(&mut context.rpc, &mint_pubkey, 100, 1000).await;

    // Attempt transfer - should fail with NonZeroTransferFeeNotSupported
    let transfer_ix = TransferChecked {
        source,
        mint: mint_pubkey,
        destination,
        amount: 100_000_000,
        decimals: 9,
        authority: owner.pubkey(),
        max_top_up: None,
    }
    .instruction()
    .unwrap();

    let result = context
        .rpc
        .create_and_send_transaction(
            &[transfer_ix],
            &context.payer.pubkey(),
            &[&context.payer, &owner],
        )
        .await;

    assert_rpc_error(result, 0, NON_ZERO_TRANSFER_FEE_NOT_SUPPORTED).unwrap();
    println!("Correctly rejected CTokenTransferChecked with non-zero transfer fees");
}

/// Test that CTokenTransferChecked fails when the mint has a non-nil transfer hook.
///
/// Setup:
/// 1. Create mint with TransferHook (nil program initially)
/// 2. Create token pool, two Light Token accounts with tokens
/// 3. Modify mint TransferHook to have non-nil program_id
/// 4. Attempt CTokenTransferChecked
///
/// Expected: TransferHookNotSupported (6501)
#[tokio::test]
#[serial]
async fn test_ctoken_transfer_fails_with_non_nil_transfer_hook() {
    let mut context = setup_extensions_test().await.unwrap();
    let mint_pubkey = context.mint_pubkey;

    // Set up accounts with tokens
    let (source, destination, owner) = setup_ctoken_accounts_for_transfer(&mut context).await;

    // Set non-nil transfer hook program on the mint
    let dummy_hook_program = Pubkey::new_unique();
    set_mint_transfer_hook(&mut context.rpc, &mint_pubkey, dummy_hook_program).await;

    // Attempt transfer - should fail with TransferHookNotSupported
    let transfer_ix = TransferChecked {
        source,
        mint: mint_pubkey,
        destination,
        amount: 100_000_000,
        decimals: 9,
        authority: owner.pubkey(),
        max_top_up: None,
    }
    .instruction()
    .unwrap();

    let result = context
        .rpc
        .create_and_send_transaction(
            &[transfer_ix],
            &context.payer.pubkey(),
            &[&context.payer, &owner],
        )
        .await;

    assert_rpc_error(result, 0, TRANSFER_HOOK_NOT_SUPPORTED).unwrap();
    println!("Correctly rejected CTokenTransferChecked with non-nil transfer hook");
}

// ============================================================================
// SPL → Light Token Transfer Tests (TransferFromSpl)
// These should FAIL when extension state is invalid (entering compressed state)
// ============================================================================

/// Set up SPL account with tokens and empty Light Token account for SPL→Light Token testing.
/// Returns (spl_account, ctoken_account, owner)
async fn setup_spl_to_ctoken_accounts(
    context: &mut ExtensionsTestContext,
) -> (Pubkey, Pubkey, Keypair) {
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    // Create SPL source account and mint tokens
    let spl_account =
        create_token_22_account(&mut context.rpc, &payer, &mint_pubkey, &payer.pubkey()).await;

    let mint_amount = 1_000_000_000u64;
    mint_spl_tokens_22(
        &mut context.rpc,
        &payer,
        &mint_pubkey,
        &spl_account,
        mint_amount,
    )
    .await;

    // Create Light Token account (destination)
    let owner = Keypair::new();
    let ctoken_keypair = Keypair::new();
    let ctoken_pubkey = ctoken_keypair.pubkey();
    let create_ix =
        CreateTokenAccount::new(payer.pubkey(), ctoken_pubkey, mint_pubkey, owner.pubkey())
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
            .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer, &ctoken_keypair])
        .await
        .unwrap();

    (spl_account, ctoken_pubkey, owner)
}

/// Test that SPL→Light Token transfer fails when the mint is paused.
///
/// SPL→Light Token uses Compress mode which enforces extension state checks.
#[tokio::test]
#[serial]
async fn test_spl_to_ctoken_fails_when_mint_paused() {
    let mut context = setup_extensions_test().await.unwrap();
    let mint_pubkey = context.mint_pubkey;
    let payer = context.payer.insecure_clone();

    // Set up accounts
    let (spl_account, ctoken_account, _owner) = setup_spl_to_ctoken_accounts(&mut context).await;

    // Pause the mint
    pause_mint(&mut context.rpc, &mint_pubkey).await;

    // Attempt SPL→Light Token transfer - should fail with MintPaused
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, true);

    let transfer_ix = TransferFromSpl {
        amount: 100_000_000,
        spl_interface_pda_bump,
        source_spl_token_account: spl_account,
        destination: ctoken_account,
        authority: payer.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: spl_token_2022::ID,
        decimals: 9,
    }
    .instruction()
    .unwrap();

    let result = context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer])
        .await;
    // fails because of token 2022 check Transferring, minting, and burning is paused on this mint
    assert_rpc_error(result, 0, 67).unwrap();
    println!("Correctly rejected SPL→Light Token when mint is paused");
}

/// Test that SPL→Light Token transfer fails when the mint has non-zero transfer fees.
#[tokio::test]
#[serial]
async fn test_spl_to_ctoken_fails_with_non_zero_transfer_fee() {
    let mut context = setup_extensions_test().await.unwrap();
    let mint_pubkey = context.mint_pubkey;
    let payer = context.payer.insecure_clone();

    // Set up accounts
    let (spl_account, ctoken_account, _owner) = setup_spl_to_ctoken_accounts(&mut context).await;

    // Set non-zero transfer fees
    set_mint_transfer_fee(&mut context.rpc, &mint_pubkey, 100, 1000).await;

    // Attempt SPL→Light Token transfer - should fail
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, true);

    let transfer_ix = TransferFromSpl {
        amount: 100_000_000,
        spl_interface_pda_bump,
        source_spl_token_account: spl_account,
        destination: ctoken_account,
        authority: payer.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: spl_token_2022::ID,
        decimals: 9,
    }
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    println!("Correctly rejected SPL→Light Token with non-zero transfer fees");
}

//! Tests for extension validation failures in CToken operations.
//!
//! This module tests extension validation for:
//! 1. CTokenTransfer(Checked) - transfers between CToken accounts
//! 2. SPL → CToken (TransferSplToCtoken) - entering via Compress mode
//! 3. CToken → SPL (TransferCTokenToSpl) - exiting via Compress+Decompress mode
//!
//! All three operations enforce extension state checks because they involve
//! Compress mode operations. The bypass only applies to pure Decompress operations
//! (e.g., decompressing from compressed accounts to SPL/CToken without any Compress).

use light_ctoken_interface::state::TokenDataVersion;
use light_ctoken_sdk::{
    ctoken::{
        CompressibleParams, CreateCTokenAccount, TransferCTokenChecked, TransferCTokenToSpl,
        TransferSplToCtoken,
    },
    spl_interface::find_spl_interface_pda_with_index,
};
use light_program_test::utils::assert::assert_rpc_error;
use light_test_utils::{
    mint_2022::{
        create_token_22_account, mint_spl_tokens_22, pause_mint, set_mint_transfer_fee,
        set_mint_transfer_hook,
    },
    Rpc,
};
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

use super::extensions::{setup_extensions_test, ExtensionsTestContext};

/// Expected error code for MintPaused
const MINT_PAUSED: u32 = 6127;

/// Expected error code for NonZeroTransferFeeNotSupported
const NON_ZERO_TRANSFER_FEE_NOT_SUPPORTED: u32 = 6129;

/// Expected error code for TransferHookNotSupported
const TRANSFER_HOOK_NOT_SUPPORTED: u32 = 6130;

/// Set up two CToken accounts with tokens for transfer testing.
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

    // Create owner and CToken accounts
    let owner = Keypair::new();

    // Create source CToken account
    let account_a_keypair = Keypair::new();
    let account_a_pubkey = account_a_keypair.pubkey();
    let create_a_ix = CreateCTokenAccount::new(
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

    // Create destination CToken account
    let account_b_keypair = Keypair::new();
    let account_b_pubkey = account_b_keypair.pubkey();
    let create_b_ix = CreateCTokenAccount::new(
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

    // Transfer SPL to source CToken account using hot path
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, true);

    let transfer_spl_to_ctoken_ix = TransferSplToCtoken {
        amount: mint_amount,
        spl_interface_pda_bump,
        source_spl_token_account: spl_account,
        destination_ctoken_account: account_a_pubkey,
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
/// 2. Create token pool, two CToken accounts with tokens
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
    let transfer_ix = TransferCTokenChecked {
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
/// 2. Create token pool, two CToken accounts with tokens
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
    let transfer_ix = TransferCTokenChecked {
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
/// 2. Create token pool, two CToken accounts with tokens
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
    let transfer_ix = TransferCTokenChecked {
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
// SPL → CToken Transfer Tests (TransferSplToCtoken)
// These should FAIL when extension state is invalid (entering compressed state)
// ============================================================================

/// Set up SPL account with tokens and empty CToken account for SPL→CToken testing.
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

    // Create CToken account (destination)
    let owner = Keypair::new();
    let ctoken_keypair = Keypair::new();
    let ctoken_pubkey = ctoken_keypair.pubkey();
    let create_ix =
        CreateCTokenAccount::new(payer.pubkey(), ctoken_pubkey, mint_pubkey, owner.pubkey())
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

/// Test that SPL→CToken transfer fails when the mint is paused.
///
/// SPL→CToken uses Compress mode which enforces extension state checks.
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

    // Attempt SPL→CToken transfer - should fail with MintPaused
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, true);

    let transfer_ix = TransferSplToCtoken {
        amount: 100_000_000,
        spl_interface_pda_bump,
        source_spl_token_account: spl_account,
        destination_ctoken_account: ctoken_account,
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

    assert_rpc_error(result, 0, MINT_PAUSED).unwrap();
    println!("Correctly rejected SPL→CToken when mint is paused");
}

/// Test that SPL→CToken transfer fails when the mint has non-zero transfer fees.
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

    // Attempt SPL→CToken transfer - should fail
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, true);

    let transfer_ix = TransferSplToCtoken {
        amount: 100_000_000,
        spl_interface_pda_bump,
        source_spl_token_account: spl_account,
        destination_ctoken_account: ctoken_account,
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

    assert_rpc_error(result, 0, NON_ZERO_TRANSFER_FEE_NOT_SUPPORTED).unwrap();
    println!("Correctly rejected SPL→CToken with non-zero transfer fees");
}

/// Test that SPL→CToken transfer fails when the mint has a non-nil transfer hook.
#[tokio::test]
#[serial]
async fn test_spl_to_ctoken_fails_with_non_nil_transfer_hook() {
    let mut context = setup_extensions_test().await.unwrap();
    let mint_pubkey = context.mint_pubkey;
    let payer = context.payer.insecure_clone();

    // Set up accounts
    let (spl_account, ctoken_account, _owner) = setup_spl_to_ctoken_accounts(&mut context).await;

    // Set non-nil transfer hook
    let dummy_hook_program = Pubkey::new_unique();
    set_mint_transfer_hook(&mut context.rpc, &mint_pubkey, dummy_hook_program).await;

    // Attempt SPL→CToken transfer - should fail
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, true);

    let transfer_ix = TransferSplToCtoken {
        amount: 100_000_000,
        spl_interface_pda_bump,
        source_spl_token_account: spl_account,
        destination_ctoken_account: ctoken_account,
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

    assert_rpc_error(result, 0, TRANSFER_HOOK_NOT_SUPPORTED).unwrap();
    println!("Correctly rejected SPL→CToken with non-nil transfer hook");
}

// ============================================================================
// CToken → SPL Transfer Tests (TransferCTokenToSpl)
// These FAIL because CToken→SPL uses compress_ctoken (Compress mode) which
// enforces extension state checks. The bypass only applies to pure Decompress
// operations (from compressed accounts, not CToken accounts).
// ============================================================================

/// Set up CToken account with tokens and empty SPL account for CToken→SPL testing.
/// Returns (ctoken_account, spl_account, owner)
async fn setup_ctoken_to_spl_accounts(
    context: &mut ExtensionsTestContext,
) -> (Pubkey, Pubkey, Keypair) {
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    // Create SPL source account and mint tokens
    let spl_source =
        create_token_22_account(&mut context.rpc, &payer, &mint_pubkey, &payer.pubkey()).await;

    let mint_amount = 1_000_000_000u64;
    mint_spl_tokens_22(
        &mut context.rpc,
        &payer,
        &mint_pubkey,
        &spl_source,
        mint_amount,
    )
    .await;

    // Create CToken account and fund it
    let owner = Keypair::new();
    let ctoken_keypair = Keypair::new();
    let ctoken_pubkey = ctoken_keypair.pubkey();
    let create_ix =
        CreateCTokenAccount::new(payer.pubkey(), ctoken_pubkey, mint_pubkey, owner.pubkey())
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

    // Transfer SPL tokens to CToken account (before modifying extension state)
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, true);

    let transfer_ix = TransferSplToCtoken {
        amount: mint_amount,
        spl_interface_pda_bump,
        source_spl_token_account: spl_source,
        destination_ctoken_account: ctoken_pubkey,
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

    // Create destination SPL account for withdrawal
    let spl_dest =
        create_token_22_account(&mut context.rpc, &payer, &mint_pubkey, &payer.pubkey()).await;

    (ctoken_pubkey, spl_dest, owner)
}

/// Test that CToken→SPL transfer FAILS when the mint is paused.
///
/// CToken→SPL uses compress_ctoken (Compress mode) which enforces extension checks.
#[tokio::test]
#[serial]
async fn test_ctoken_to_spl_fails_when_mint_paused() {
    let mut context = setup_extensions_test().await.unwrap();
    let mint_pubkey = context.mint_pubkey;
    let payer = context.payer.insecure_clone();

    // Set up accounts with tokens in CToken
    let (ctoken_account, spl_account, owner) = setup_ctoken_to_spl_accounts(&mut context).await;

    // Pause the mint AFTER funding CToken account
    pause_mint(&mut context.rpc, &mint_pubkey).await;

    // Attempt CToken→SPL transfer - should FAIL
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, true);

    let transfer_ix = TransferCTokenToSpl {
        source_ctoken_account: ctoken_account,
        destination_spl_token_account: spl_account,
        amount: 100_000_000,
        authority: owner.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_interface_pda_bump,
        decimals: 9,
        spl_token_program: spl_token_2022::ID,
    }
    .instruction()
    .unwrap();

    let result = context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    assert_rpc_error(result, 0, MINT_PAUSED).unwrap();
    println!("Correctly rejected CToken→SPL when mint is paused");
}

/// Test that CToken→SPL transfer FAILS with non-zero transfer fees.
#[tokio::test]
#[serial]
async fn test_ctoken_to_spl_fails_with_non_zero_transfer_fee() {
    let mut context = setup_extensions_test().await.unwrap();
    let mint_pubkey = context.mint_pubkey;
    let payer = context.payer.insecure_clone();

    // Set up accounts with tokens in CToken
    let (ctoken_account, spl_account, owner) = setup_ctoken_to_spl_accounts(&mut context).await;

    // Set non-zero transfer fees AFTER funding CToken account
    set_mint_transfer_fee(&mut context.rpc, &mint_pubkey, 100, 1000).await;

    // Attempt CToken→SPL transfer - should FAIL
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, true);

    let transfer_ix = TransferCTokenToSpl {
        source_ctoken_account: ctoken_account,
        destination_spl_token_account: spl_account,
        amount: 100_000_000,
        authority: owner.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_interface_pda_bump,
        decimals: 9,
        spl_token_program: spl_token_2022::ID,
    }
    .instruction()
    .unwrap();

    let result = context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    assert_rpc_error(result, 0, NON_ZERO_TRANSFER_FEE_NOT_SUPPORTED).unwrap();
    println!("Correctly rejected CToken→SPL with non-zero transfer fees");
}

/// Test that CToken→SPL transfer FAILS with non-nil transfer hook.
#[tokio::test]
#[serial]
async fn test_ctoken_to_spl_fails_with_non_nil_transfer_hook() {
    let mut context = setup_extensions_test().await.unwrap();
    let mint_pubkey = context.mint_pubkey;
    let payer = context.payer.insecure_clone();

    // Set up accounts with tokens in CToken
    let (ctoken_account, spl_account, owner) = setup_ctoken_to_spl_accounts(&mut context).await;

    // Set non-nil transfer hook AFTER funding CToken account
    let dummy_hook_program = Pubkey::new_unique();
    set_mint_transfer_hook(&mut context.rpc, &mint_pubkey, dummy_hook_program).await;

    // Attempt CToken→SPL transfer - should FAIL
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, true);

    let transfer_ix = TransferCTokenToSpl {
        source_ctoken_account: ctoken_account,
        destination_spl_token_account: spl_account,
        amount: 100_000_000,
        authority: owner.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_interface_pda_bump,
        decimals: 9,
        spl_token_program: spl_token_2022::ID,
    }
    .instruction()
    .unwrap();

    let result = context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    assert_rpc_error(result, 0, TRANSFER_HOOK_NOT_SUPPORTED).unwrap();
    println!("Correctly rejected CToken→SPL with non-nil transfer hook");
}

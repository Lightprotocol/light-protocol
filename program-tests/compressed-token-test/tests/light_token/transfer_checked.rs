//! Tests for CTokenTransfer vs CTokenTransferChecked with restricted extensions
//!
//! Verifies that mints with restricted T22 extensions (Pausable, PermanentDelegate,
//! TransferFee, TransferHook) cannot use CTokenTransfer and must use CTokenTransferChecked.

use anchor_spl::token_2022::spl_token_2022;
use light_program_test::utils::assert::assert_rpc_error;
use light_test_utils::{
    assert_ctoken_transfer::assert_ctoken_transfer,
    mint_2022::{create_token_22_account, mint_spl_tokens_22},
    Rpc,
};
use light_token_interface::state::TokenDataVersion;
use light_token_sdk::{
    spl_interface::find_spl_interface_pda_with_index,
    token::{CompressibleParams, CreateTokenAccount, Transfer, TransferChecked, TransferFromSpl},
};
use serial_test::serial;
use solana_sdk::{native_token::LAMPORTS_PER_SOL, signature::Keypair, signer::Signer};

use crate::extensions::setup_extensions_test;

/// Test that CTokenTransfer fails with MintRequiredForTransfer (6128) for accounts with
/// restricted extensions, while CTokenTransferChecked succeeds.
#[tokio::test]
#[serial]
async fn test_transfer_requires_checked_for_restricted_extensions() {
    let mut context = setup_extensions_test().await.unwrap();
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    // Step 1: Create SPL Token-2022 account and mint tokens
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

    // Step 2: Create two compressible Light Token accounts (A and B) with all extensions
    let owner = Keypair::new();
    context
        .rpc
        .airdrop_lamports(&owner.pubkey(), LAMPORTS_PER_SOL)
        .await
        .unwrap();

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

    // Step 3: Transfer SPL to Light Token account A using hot path
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, true);

    let transfer_spl_to_ctoken_ix = TransferFromSpl {
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

    // Step 4: Try CTokenTransfer (discriminator 3) - should FAIL with MintRequiredForTransfer (6128)
    let transfer_amount = 500_000_000u64;

    let transfer_ix = Transfer {
        source: account_a_pubkey,
        destination: account_b_pubkey,
        amount: transfer_amount,
        authority: owner.pubkey(),
        max_top_up: Some(0), // 0 = no limit, but includes system program for compressible
    }
    .instruction()
    .unwrap();

    let result = context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Error 6128 = MintRequiredForTransfer
    assert_rpc_error(result, 0, 6128).unwrap();

    println!("CTokenTransfer correctly rejected with MintRequiredForTransfer (6128)");

    // Step 5: Use CTokenTransferChecked (discriminator 12) - should SUCCEED
    let transfer_checked_ix = TransferChecked {
        source: account_a_pubkey,
        mint: mint_pubkey,
        destination: account_b_pubkey,
        amount: transfer_amount,
        decimals: 9,
        authority: owner.pubkey(),
        max_top_up: Some(0), // 0 = no limit, but includes system program for compressible
    }
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[transfer_checked_ix], &payer.pubkey(), &[&payer, &owner])
        .await
        .unwrap();

    // Verify transfer using helper
    assert_ctoken_transfer(
        &mut context.rpc,
        account_a_pubkey,
        account_b_pubkey,
        transfer_amount,
    )
    .await;

    println!(
        "CTokenTransferChecked succeeded: transferred {} tokens from A to B",
        transfer_amount
    );
}

use super::shared::*;

// ============================================================================
// Test Plan: CreateTokenAccount (Discriminator 18)
// ============================================================================
//
// This file tests the CreateTokenAccount instruction, which is equivalent to
// SPL Token's InitializeAccount3. It creates ctoken solana accounts with and
// without the Compressible extension.
//
// Existing Tests (in compress_and_close.rs):
// -------------------------------------------
// - test_compress_and_close_with_compression_authority: pre_pay_num_epochs = 2
// - test_compressible_account_with_custom_rent_payer_close_with_compression_authority: pre_pay_num_epochs = 2
//
// Planned Functional Tests (Single Transaction):
// -----------------------------------------------
// 1. test_create_token_account_zero_epoch_prefunding
//    - Create compressible token account with pre_pay_num_epochs = 0
//    - Validates: Account creation succeeds, no additional rent charged (only rent exemption),
//      account is immediately compressible
//
// 2. test_create_token_account_three_epoch_prefunding
//    - Create compressible account with pre_pay_num_epochs = 3
//    - Validates: Account created successfully, correct rent calculation for 3 epochs
//
// 3. test_create_token_account_ten_epoch_prefunding
//    - Create compressible account with pre_pay_num_epochs = 10
//    - Validates: Account created successfully, correct rent calculation for 10 epochs
//
// Coverage Notes:
// ---------------
// - One epoch prefunding (pre_pay_num_epochs = 1) is FORBIDDEN and tested in failing tests
// - Two epoch prefunding is already tested (see existing tests above)
// - Zero epoch creates immediately compressible accounts
// - TokenDataVersion::ShaFlat (V3) is already tested in existing tests
//
// ============================================================================

#[tokio::test]
#[serial]
async fn test_create_token_account_zero_epoch_prefunding() {
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();
    let token_account_pubkey = context.token_account_keypair.pubkey();

    let compressible_data = CompressibleData {
        compression_authority: context.compression_authority,
        rent_sponsor: context.rent_sponsor,
        num_prepaid_epochs: 0,
        lamports_per_write: Some(100),
        account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
        compress_to_pubkey: false,
    };

    let create_token_account_ix =
        light_compressed_token_sdk::instructions::create_compressible_token_account(
            light_compressed_token_sdk::instructions::CreateCompressibleTokenAccount {
                account_pubkey: token_account_pubkey,
                mint_pubkey: context.mint_pubkey,
                owner_pubkey: context.owner_keypair.pubkey(),
                compressible_config: context.compressible_config,
                rent_sponsor: context.rent_sponsor,
                pre_pay_num_epochs: compressible_data.num_prepaid_epochs,
                lamports_per_write: compressible_data.lamports_per_write,
                payer: payer_pubkey,
                compress_to_account_pubkey: None,
                token_account_version: compressible_data.account_version,
            },
        )
        .unwrap();

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
        Some(compressible_data),
    )
    .await;
}

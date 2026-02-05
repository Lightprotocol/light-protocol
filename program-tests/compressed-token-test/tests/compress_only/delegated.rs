//! Tests for delegate-related behavior during compress/decompress.
//!
//! This module tests:
//! - Delegated amount preservation through compress -> decompress cycle
//! - Regular delegate decompression authorization

use serial_test::serial;
use solana_sdk::signature::Keypair;

use super::shared::{
    run_compress_and_close_extension_test, CompressAndCloseTestConfig, ALL_EXTENSIONS,
};

/// Test that delegated amount is preserved through compress -> decompress cycle.
#[tokio::test]
#[serial]
async fn test_compress_and_close_with_delegated_amount() {
    let delegate = Keypair::new();
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        extensions: ALL_EXTENSIONS,
        delegate_config: Some((delegate, 500_000_000)),
        is_frozen: false,
        use_permanent_delegate_for_decompress: false,
        use_delegate_for_decompress: false,
    })
    .await
    .unwrap();
}

/// Test that regular delegate can decompress CompressedOnly tokens.
#[tokio::test]
#[serial]
async fn test_compress_and_close_delegate_decompress() {
    let delegate = Keypair::new();
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        extensions: ALL_EXTENSIONS,
        delegate_config: Some((delegate, 500_000_000)),
        is_frozen: false,
        use_permanent_delegate_for_decompress: false,
        use_delegate_for_decompress: true,
    })
    .await
    .unwrap();
}

/// Test delegated amount with no extensions.
#[tokio::test]
#[serial]
async fn test_compress_and_close_with_delegated_amount_no_extensions() {
    let delegate = Keypair::new();
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        extensions: &[],
        delegate_config: Some((delegate, 500_000_000)),
        is_frozen: false,
        use_permanent_delegate_for_decompress: false,
        use_delegate_for_decompress: false,
    })
    .await
    .unwrap();
}

/// Test delegate decompress with no extensions.
#[tokio::test]
#[serial]
async fn test_compress_and_close_delegate_decompress_no_extensions() {
    let delegate = Keypair::new();
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        extensions: &[],
        delegate_config: Some((delegate, 500_000_000)),
        is_frozen: false,
        use_permanent_delegate_for_decompress: false,
        use_delegate_for_decompress: true,
    })
    .await
    .unwrap();
}

/// Test that orphan delegate (delegate set, delegated_amount = 0) is preserved
/// through compress -> decompress cycle.
///
/// Covers spec requirements:
/// - #12: Orphan delegate (delegate set, delegated_amount = 0)
/// - #17: Restores orphan delegate on decompress
/// - #26: Full round-trip orphan delegate state preserved
#[tokio::test]
#[serial]
async fn test_compress_and_close_preserves_orphan_delegate() {
    let delegate = Keypair::new();
    // delegate_config with delegated_amount = 0 creates an orphan delegate
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        extensions: ALL_EXTENSIONS,
        delegate_config: Some((delegate, 0)), // delegated_amount = 0 but delegate is set
        is_frozen: false,
        use_permanent_delegate_for_decompress: false,
        use_delegate_for_decompress: false,
    })
    .await
    .unwrap();
}

/// Test orphan delegate with no extensions.
#[tokio::test]
#[serial]
async fn test_compress_and_close_orphan_delegate_no_extensions() {
    let delegate = Keypair::new();
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        extensions: &[],
        delegate_config: Some((delegate, 0)),
        is_frozen: false,
        use_permanent_delegate_for_decompress: false,
        use_delegate_for_decompress: false,
    })
    .await
    .unwrap();
}

/// Test that orphan delegate can still decompress (delegate has authority even with 0 amount).
#[tokio::test]
#[serial]
async fn test_orphan_delegate_can_decompress() {
    let delegate = Keypair::new();
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        extensions: ALL_EXTENSIONS,
        delegate_config: Some((delegate, 0)),
        is_frozen: false,
        use_permanent_delegate_for_decompress: false,
        use_delegate_for_decompress: true, // delegate signs for decompress
    })
    .await
    .unwrap();
}

/// Test that decompressing to an existing account with the same delegate
/// accumulates the delegated_amount.
///
/// Scenario:
/// 1. Create destination ctoken with delegate D and delegated_amount = 300
/// 2. Create compressed token with delegate D and delegated_amount = 200
/// 3. Decompress to existing destination
/// 4. Verify destination delegated_amount = 500 (300 + 200)
#[tokio::test]
#[serial]
async fn test_decompress_accumulates_delegated_amount() {
    use super::shared::{set_ctoken_account_state, setup_extensions_test};
    use borsh::BorshDeserialize;
    use light_client::indexer::Indexer;
    use light_compressed_token_sdk::spl_interface::find_spl_interface_pda_with_index;
    use light_program_test::program_test::TestRpc;
    use light_test_utils::{
        actions::legacy::instructions::transfer2::{
            create_generic_transfer2_instruction, DecompressInput, Transfer2InstructionType,
        },
        mint_2022::{create_token_22_account, mint_spl_tokens_22, RESTRICTED_EXTENSIONS},
        Rpc,
    };
    use light_token::instruction::{CompressibleParams, CreateTokenAccount, TransferFromSpl};
    use light_token_interface::{
        instructions::extensions::{
            CompressedOnlyExtensionInstructionData, ExtensionInstructionData,
        },
        state::{Token, TokenDataVersion},
    };
    use solana_sdk::signer::Signer;

    let mut context = setup_extensions_test(ALL_EXTENSIONS).await.unwrap();
    let has_restricted_extensions = ALL_EXTENSIONS
        .iter()
        .any(|ext| RESTRICTED_EXTENSIONS.contains(ext));
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    // Create delegate keypair
    let delegate = Keypair::new();
    let existing_delegated_amount = 300_000_000u64;
    let compressed_delegated_amount = 200_000_000u64;
    let expected_total_delegated_amount = existing_delegated_amount + compressed_delegated_amount;

    // 1. Create SPL Token-2022 account and mint tokens
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

    // 2. Create Light Token account to be compressed (source)
    let owner = Keypair::new();
    let source_keypair = Keypair::new();
    let source_account = source_keypair.pubkey();

    let create_source_ix =
        CreateTokenAccount::new(payer.pubkey(), source_account, mint_pubkey, owner.pubkey())
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
                pre_pay_num_epochs: 0, // immediately compressible
                lamports_per_write: Some(100),
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: has_restricted_extensions,
            })
            .instruction()
            .unwrap();

    context
        .rpc
        .create_and_send_transaction(
            &[create_source_ix],
            &payer.pubkey(),
            &[&payer, &source_keypair],
        )
        .await
        .unwrap();

    // 3. Transfer tokens to source Light Token
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, has_restricted_extensions);
    let transfer_ix = TransferFromSpl {
        amount: mint_amount,
        spl_interface_pda_bump,
        decimals: 9,
        source_spl_token_account: spl_account,
        destination: source_account,
        authority: payer.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: spl_token_2022::ID,
    }
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // 4. Set delegate on source account before compression
    set_ctoken_account_state(
        &mut context.rpc,
        source_account,
        Some(delegate.pubkey()),
        compressed_delegated_amount,
        false, // not frozen
    )
    .await
    .unwrap();

    // 5. Create DESTINATION Light Token account that already has a delegate
    let dest_keypair = Keypair::new();
    let dest_account = dest_keypair.pubkey();

    let create_dest_ix =
        CreateTokenAccount::new(payer.pubkey(), dest_account, mint_pubkey, owner.pubkey())
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
                compression_only: has_restricted_extensions,
            })
            .instruction()
            .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[create_dest_ix], &payer.pubkey(), &[&payer, &dest_keypair])
        .await
        .unwrap();

    // Give destination 10 SOL so it won't be compressed by forester
    context
        .rpc
        .airdrop_lamports(&dest_account, 10_000_000_000)
        .await
        .unwrap();

    // 6. Set the SAME delegate on destination with existing delegated_amount
    set_ctoken_account_state(
        &mut context.rpc,
        dest_account,
        Some(delegate.pubkey()),
        existing_delegated_amount,
        false, // not frozen
    )
    .await
    .unwrap();

    // 7. Warp epoch to trigger forester compression of source account
    context.rpc.warp_epoch_forward(30).await.unwrap();

    // 8. Verify source account is compressed
    let source_after = context.rpc.get_account(source_account).await.unwrap();
    assert!(
        source_after.is_none() || source_after.unwrap().lamports == 0,
        "Source account should be closed after compression"
    );

    // 9. Get the compressed token account
    let compressed_accounts = context
        .rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(
        compressed_accounts.len(),
        1,
        "Should have exactly 1 compressed token account"
    );

    // 10. Decompress to existing destination with matching delegate
    let in_tlv = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: compressed_delegated_amount,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 0,
            is_ata: false,
            bump: 0,
            owner_index: 0,
        },
    )]];

    let decompress_ix = create_generic_transfer2_instruction(
        &mut context.rpc,
        vec![Transfer2InstructionType::Decompress(DecompressInput {
            compressed_token_account: vec![compressed_accounts[0].clone()],
            decompress_amount: mint_amount,
            solana_token_account: dest_account,
            amount: mint_amount,
            pool_index: None,
            decimals: 9,
            in_tlv: Some(in_tlv),
        })],
        payer.pubkey(),
        true,
    )
    .await
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer, &owner])
        .await
        .unwrap();

    // 11. Verify delegated_amount was accumulated
    let dest_account_data = context
        .rpc
        .get_account(dest_account)
        .await
        .unwrap()
        .expect("Destination account should exist");

    let dest_ctoken = Token::deserialize(&mut &dest_account_data.data[..])
        .expect("Failed to deserialize Token");

    assert_eq!(
        dest_ctoken.delegate,
        Some(delegate.pubkey().to_bytes().into()),
        "Delegate should be preserved"
    );

    assert_eq!(
        dest_ctoken.delegated_amount, expected_total_delegated_amount,
        "Delegated amount should be accumulated: {} + {} = {}",
        existing_delegated_amount, compressed_delegated_amount, expected_total_delegated_amount
    );

    println!(
        "Successfully accumulated delegated_amount: {} + {} = {}",
        existing_delegated_amount, compressed_delegated_amount, dest_ctoken.delegated_amount
    );
}

/// Test that decompressing to an existing account with a DIFFERENT delegate
/// does not accumulate the delegated_amount (delegates must match).
///
/// Scenario:
/// 1. Create destination ctoken with delegate D1 and delegated_amount = 300
/// 2. Create compressed token with delegate D2 and delegated_amount = 200
/// 3. Decompress to existing destination
/// 4. Verify destination delegated_amount remains 300 (no accumulation)
#[tokio::test]
#[serial]
async fn test_decompress_skips_accumulation_when_delegate_mismatch() {
    use super::shared::{set_ctoken_account_state, setup_extensions_test};
    use borsh::BorshDeserialize;
    use light_client::indexer::Indexer;
    use light_compressed_token_sdk::spl_interface::find_spl_interface_pda_with_index;
    use light_program_test::program_test::TestRpc;
    use light_test_utils::{
        actions::legacy::instructions::transfer2::{
            create_generic_transfer2_instruction, DecompressInput, Transfer2InstructionType,
        },
        mint_2022::{create_token_22_account, mint_spl_tokens_22, RESTRICTED_EXTENSIONS},
        Rpc,
    };
    use light_token::instruction::{CompressibleParams, CreateTokenAccount, TransferFromSpl};
    use light_token_interface::{
        instructions::extensions::{
            CompressedOnlyExtensionInstructionData, ExtensionInstructionData,
        },
        state::{Token, TokenDataVersion},
    };
    use solana_sdk::signer::Signer;

    let mut context = setup_extensions_test(ALL_EXTENSIONS).await.unwrap();
    let has_restricted_extensions = ALL_EXTENSIONS
        .iter()
        .any(|ext| RESTRICTED_EXTENSIONS.contains(ext));
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    // Create TWO DIFFERENT delegate keypairs
    let delegate_compressed = Keypair::new(); // delegate in compressed token
    let delegate_destination = Keypair::new(); // delegate in destination account
    let existing_delegated_amount = 300_000_000u64;
    let compressed_delegated_amount = 200_000_000u64;

    // 1. Create SPL Token-2022 account and mint tokens
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

    // 2. Create Light Token account to be compressed (source)
    let owner = Keypair::new();
    let source_keypair = Keypair::new();
    let source_account = source_keypair.pubkey();

    let create_source_ix =
        CreateTokenAccount::new(payer.pubkey(), source_account, mint_pubkey, owner.pubkey())
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
                pre_pay_num_epochs: 0, // immediately compressible
                lamports_per_write: Some(100),
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: has_restricted_extensions,
            })
            .instruction()
            .unwrap();

    context
        .rpc
        .create_and_send_transaction(
            &[create_source_ix],
            &payer.pubkey(),
            &[&payer, &source_keypair],
        )
        .await
        .unwrap();

    // 3. Transfer tokens to source Light Token
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, has_restricted_extensions);
    let transfer_ix = TransferFromSpl {
        amount: mint_amount,
        spl_interface_pda_bump,
        decimals: 9,
        source_spl_token_account: spl_account,
        destination: source_account,
        authority: payer.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: spl_token_2022::ID,
    }
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // 4. Set delegate on source account before compression (delegate_compressed)
    set_ctoken_account_state(
        &mut context.rpc,
        source_account,
        Some(delegate_compressed.pubkey()),
        compressed_delegated_amount,
        false, // not frozen
    )
    .await
    .unwrap();

    // 5. Create DESTINATION Light Token account with a DIFFERENT delegate
    let dest_keypair = Keypair::new();
    let dest_account = dest_keypair.pubkey();

    let create_dest_ix =
        CreateTokenAccount::new(payer.pubkey(), dest_account, mint_pubkey, owner.pubkey())
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
                compression_only: has_restricted_extensions,
            })
            .instruction()
            .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[create_dest_ix], &payer.pubkey(), &[&payer, &dest_keypair])
        .await
        .unwrap();

    // Give destination 10 SOL so it won't be compressed by forester
    context
        .rpc
        .airdrop_lamports(&dest_account, 10_000_000_000)
        .await
        .unwrap();

    // 6. Set a DIFFERENT delegate on destination
    set_ctoken_account_state(
        &mut context.rpc,
        dest_account,
        Some(delegate_destination.pubkey()),
        existing_delegated_amount,
        false, // not frozen
    )
    .await
    .unwrap();

    // 7. Warp epoch to trigger forester compression of source account
    context.rpc.warp_epoch_forward(30).await.unwrap();

    // 8. Verify source account is compressed
    let source_after = context.rpc.get_account(source_account).await.unwrap();
    assert!(
        source_after.is_none() || source_after.unwrap().lamports == 0,
        "Source account should be closed after compression"
    );

    // 9. Get the compressed token account
    let compressed_accounts = context
        .rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(
        compressed_accounts.len(),
        1,
        "Should have exactly 1 compressed token account"
    );

    // 10. Decompress to existing destination with DIFFERENT delegate
    let in_tlv = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: compressed_delegated_amount,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 0,
            is_ata: false,
            bump: 0,
            owner_index: 0,
        },
    )]];

    let decompress_ix = create_generic_transfer2_instruction(
        &mut context.rpc,
        vec![Transfer2InstructionType::Decompress(DecompressInput {
            compressed_token_account: vec![compressed_accounts[0].clone()],
            decompress_amount: mint_amount,
            solana_token_account: dest_account,
            amount: mint_amount,
            pool_index: None,
            decimals: 9,
            in_tlv: Some(in_tlv),
        })],
        payer.pubkey(),
        true,
    )
    .await
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer, &owner])
        .await
        .unwrap();

    // 11. Verify delegated_amount was NOT accumulated (delegates don't match)
    let dest_account_data = context
        .rpc
        .get_account(dest_account)
        .await
        .unwrap()
        .expect("Destination account should exist");

    let dest_ctoken = Token::deserialize(&mut &dest_account_data.data[..])
        .expect("Failed to deserialize Token");

    // Delegate should remain as the destination's original delegate
    assert_eq!(
        dest_ctoken.delegate,
        Some(delegate_destination.pubkey().to_bytes().into()),
        "Delegate should remain as destination's original delegate"
    );

    // Delegated amount should NOT be accumulated (delegates don't match)
    assert_eq!(
        dest_ctoken.delegated_amount, existing_delegated_amount,
        "Delegated amount should NOT be accumulated when delegates don't match: expected {}, got {}",
        existing_delegated_amount, dest_ctoken.delegated_amount
    );

    println!(
        "Successfully skipped accumulation when delegates don't match: destination delegated_amount remains {}",
        dest_ctoken.delegated_amount
    );
}

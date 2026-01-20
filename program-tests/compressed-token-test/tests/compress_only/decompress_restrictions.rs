//! Tests for CompressedOnly decompress restrictions.
//!
//! This module tests:
//! - Spec #13: CompressedOnly inputs can only decompress to Light Token, not SPL
//! - Spec #14: CompressedOnly inputs must decompress complete account (no change output)

use light_client::indexer::Indexer;
use light_program_test::{
    program_test::{LightProgramTest, TestRpc},
    utils::assert::assert_rpc_error,
    ProgramTestConfig, Rpc,
};
use light_test_utils::mint_2022::{
    create_mint_22_with_extension_types, create_token_22_account, mint_spl_tokens_22,
    RESTRICTED_EXTENSIONS,
};
use light_token::{
    spl_interface::find_spl_interface_pda_with_index,
    token::{CompressibleParams, CreateTokenAccount, TransferFromSpl},
};
use light_token_client::instructions::transfer2::{
    create_generic_transfer2_instruction, DecompressInput, Transfer2InstructionType,
};
use light_token_interface::{
    instructions::extensions::{CompressedOnlyExtensionInstructionData, ExtensionInstructionData},
    state::TokenDataVersion,
};
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use spl_token_2022::extension::ExtensionType;

/// Expected error code for CompressedOnlyRequiresCTokenDecompress
const COMPRESSED_ONLY_REQUIRES_CTOKEN_DECOMPRESS: u32 = 6149;

/// Expected error code for CompressedOnlyBlocksTransfer
const COMPRESSED_ONLY_BLOCKS_TRANSFER: u32 = 18048;

/// Helper to set up a compressed token with CompressedOnly extension for decompress testing
async fn setup_compressed_token_for_decompress(
    extensions: &[ExtensionType],
) -> (
    LightProgramTest,
    Keypair,                                       // payer
    Pubkey,                                        // mint
    Keypair,                                       // owner
    light_client::indexer::CompressedTokenAccount, // compressed account
    u64,                                           // amount
) {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create mint with extensions
    let (mint_keypair, _) =
        create_mint_22_with_extension_types(&mut rpc, &payer, 9, extensions).await;
    let mint_pubkey = mint_keypair.pubkey();

    // Create SPL Token-2022 account and mint tokens
    let spl_account =
        create_token_22_account(&mut rpc, &payer, &mint_pubkey, &payer.pubkey()).await;
    let mint_amount = 1_000_000_000u64;
    mint_spl_tokens_22(&mut rpc, &payer, &mint_pubkey, &spl_account, mint_amount).await;

    // Create Light Token account with compression_only=true
    let owner = Keypair::new();
    let account_keypair = Keypair::new();
    let ctoken_account = account_keypair.pubkey();

    let create_ix =
        CreateTokenAccount::new(payer.pubkey(), ctoken_account, mint_pubkey, owner.pubkey())
            .with_compressible(CompressibleParams {
                compressible_config: rpc
                    .test_accounts
                    .funding_pool_config
                    .compressible_config_pda,
                rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
                pre_pay_num_epochs: 0,
                lamports_per_write: Some(100),
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: true,
            })
            .instruction()
            .unwrap();

    rpc.create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer, &account_keypair])
        .await
        .unwrap();

    // Transfer tokens to Light Token
    let has_restricted = extensions
        .iter()
        .any(|ext| RESTRICTED_EXTENSIONS.contains(ext));
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, has_restricted);
    let transfer_ix = TransferFromSpl {
        amount: mint_amount,
        spl_interface_pda_bump,
        decimals: 9,
        source_spl_token_account: spl_account,
        destination: ctoken_account,
        authority: payer.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: spl_token_2022::ID,
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Warp epoch to trigger forester compression
    rpc.warp_epoch_forward(30).await.unwrap();

    // Get compressed token accounts
    let compressed_accounts = rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(
        compressed_accounts.len(),
        1,
        "Should have 1 compressed account"
    );

    (
        rpc,
        payer,
        mint_pubkey,
        owner,
        compressed_accounts[0].clone(),
        mint_amount,
    )
}

/// Test that CompressedOnly accounts cannot decompress to SPL Token-2022 accounts.
///
/// Covers spec requirement #13: Can only decompress to Light Token, not SPL account
#[tokio::test]
#[serial]
async fn test_decompress_compressed_only_rejects_spl_destination() {
    // Set up compressed token with CompressedOnly extension
    let (mut rpc, payer, mint_pubkey, owner, compressed_account, amount) =
        setup_compressed_token_for_decompress(&[ExtensionType::Pausable]).await;

    // Create SPL Token-2022 account (NOT Light Token) as destination
    let spl_destination =
        create_token_22_account(&mut rpc, &payer, &mint_pubkey, &owner.pubkey()).await;

    // Attempt to decompress to SPL account with CompressedOnly in_tlv
    let in_tlv = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 0,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 0,
            is_ata: false,
            bump: 0,
            owner_index: 0,
        },
    )]];

    let decompress_ix = create_generic_transfer2_instruction(
        &mut rpc,
        vec![Transfer2InstructionType::Decompress(DecompressInput {
            compressed_token_account: vec![compressed_account],
            decompress_amount: amount,
            solana_token_account: spl_destination,
            amount,
            pool_index: None,
            decimals: 9,
            in_tlv: Some(in_tlv),
        })],
        payer.pubkey(),
        true,
    )
    .await
    .unwrap();

    let result = rpc
        .create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail because CompressedOnly inputs must decompress to Light Token, not SPL
    assert_rpc_error(result, 0, COMPRESSED_ONLY_REQUIRES_CTOKEN_DECOMPRESS).unwrap();
}

/// Test that CompressedOnly accounts cannot do partial decompress (would create change output).
///
/// Covers spec requirement #14: Must decompress complete account (no change output)
#[tokio::test]
#[serial]
async fn test_decompress_compressed_only_rejects_partial_decompress() {
    // Set up compressed token with CompressedOnly extension
    let (mut rpc, payer, mint_pubkey, owner, compressed_account, amount) =
        setup_compressed_token_for_decompress(&[ExtensionType::Pausable]).await;

    // Create destination Light Token account
    let dest_keypair = Keypair::new();
    let destination_pubkey = dest_keypair.pubkey();

    let create_dest_ix = CreateTokenAccount::new(
        payer.pubkey(),
        destination_pubkey,
        mint_pubkey,
        owner.pubkey(),
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
        compression_only: true,
    })
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[create_dest_ix], &payer.pubkey(), &[&payer, &dest_keypair])
        .await
        .unwrap();

    // Attempt partial decompress (half the amount)
    // This would create a change output with the remaining tokens
    let partial_amount = amount / 2;

    let in_tlv = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 0,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 0,
            is_ata: false,
            bump: 0,
            owner_index: 0,
        },
    )]];

    let decompress_ix = create_generic_transfer2_instruction(
        &mut rpc,
        vec![Transfer2InstructionType::Decompress(DecompressInput {
            compressed_token_account: vec![compressed_account],
            decompress_amount: partial_amount, // Only decompress half
            solana_token_account: destination_pubkey,
            amount, // Full input amount
            pool_index: None,
            decimals: 9,
            in_tlv: Some(in_tlv),
        })],
        payer.pubkey(),
        true,
    )
    .await
    .unwrap();

    let result = rpc
        .create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail because partial decompress would create a change output (compressed output)
    // and CompressedOnly inputs cannot have compressed outputs
    assert_rpc_error(result, 0, COMPRESSED_ONLY_BLOCKS_TRANSFER).unwrap();
}

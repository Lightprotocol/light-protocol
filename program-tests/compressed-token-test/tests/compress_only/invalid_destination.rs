//! Tests for decompress destination validation.
//!
//! These tests verify:
//! 1. Decompression fails with DecompressDestinationMismatch when owner doesn't match
//! 2. Decompression SUCCEEDS when destination has existing state (amount, delegate, etc.)
//!    - Amount is ADDED to existing balance
//!    - Existing delegate is PRESERVED (not overwritten)

use anchor_spl::token_2022::spl_token_2022;
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
    instruction::{CompressibleParams, CreateTokenAccount, TransferFromSpl},
    spl_interface::find_spl_interface_pda_with_index,
};
use light_token_client::instructions::transfer2::{
    create_generic_transfer2_instruction, DecompressInput, Transfer2InstructionType,
};
use light_token_interface::{
    instructions::extensions::{CompressedOnlyExtensionInstructionData, ExtensionInstructionData},
    state::TokenDataVersion,
};
use serial_test::serial;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey, signature::Keypair, signer::Signer};

use super::shared::ExtensionType;

/// Expected error code for DecompressDestinationMismatch (owner or ATA mismatch)
const DECOMPRESS_DESTINATION_MISMATCH: u32 = 18057;

/// Helper to modify Light Token account to have invalid state
async fn set_invalid_destination_state(
    rpc: &mut LightProgramTest,
    account_pubkey: Pubkey,
    amount: Option<u64>,
    delegate: Option<Pubkey>,
    delegated_amount: Option<u64>,
    close_authority: Option<Pubkey>,
) {
    use anchor_spl::token_2022::spl_token_2022;
    use solana_sdk::{program_option::COption, program_pack::Pack};

    let mut account_info = rpc.get_account(account_pubkey).await.unwrap().unwrap();

    let mut spl_account =
        spl_token_2022::state::Account::unpack_unchecked(&account_info.data[..165]).unwrap();

    if let Some(amt) = amount {
        spl_account.amount = amt;
    }
    if let Some(d) = delegate {
        spl_account.delegate = COption::Some(d);
    }
    if let Some(da) = delegated_amount {
        spl_account.delegated_amount = da;
    }
    if let Some(ca) = close_authority {
        spl_account.close_authority = COption::Some(ca);
    }

    spl_token_2022::state::Account::pack(spl_account, &mut account_info.data[..165]).unwrap();
    rpc.set_account(account_pubkey, account_info);
}

/// Helper to set up a compressed token with CompressedOnly extension for decompress testing
async fn setup_compressed_token_for_decompress(
    extensions: &[super::shared::ExtensionType],
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
    use spl_token_2022::ID as SPL_TOKEN_2022_ID;
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
        spl_token_program: SPL_TOKEN_2022_ID,
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

#[tokio::test]
#[serial]
async fn test_decompress_owner_mismatch() {
    // Set up compressed token - owner is the actual owner of the compressed account
    let (mut rpc, payer, mint_pubkey, owner, compressed_account, amount) =
        setup_compressed_token_for_decompress(&[ExtensionType::Pausable]).await;

    // Create destination with DIFFERENT owner (not matching compressed account owner)
    let different_owner = Keypair::new();
    let dest_keypair = Keypair::new();
    let destination_pubkey = dest_keypair.pubkey();

    let create_dest_ix = CreateTokenAccount::new(
        payer.pubkey(),
        destination_pubkey,
        mint_pubkey,
        different_owner.pubkey(), // Different owner - doesn't match compressed account owner!
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
            solana_token_account: destination_pubkey,
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

    // Sign with payer and actual owner (of the compressed account)
    // The validation should fail because destination.owner != compressed_account.owner
    let result = rpc
        .create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail because destination owner doesn't match input owner
    assert_rpc_error(result, 0, DECOMPRESS_DESTINATION_MISMATCH).unwrap();
}

/// Test that decompression to an account with existing balance SUCCEEDS
/// and the amount is ADDED to the existing balance.
#[tokio::test]
#[serial]
async fn test_decompress_non_zero_amount() {
    // Set up compressed token
    let (mut rpc, payer, mint_pubkey, owner, compressed_account, amount) =
        setup_compressed_token_for_decompress(&[ExtensionType::Pausable]).await;

    // Create destination with correct owner
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

    // Set non-zero amount on destination (existing balance of 1000)
    let existing_balance = 1000u64;
    set_invalid_destination_state(
        &mut rpc,
        destination_pubkey,
        Some(existing_balance),
        None,
        None,
        None,
    )
    .await;

    // Build decompress input with CompressedOnly extension data
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
            solana_token_account: destination_pubkey,
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

    // Execute decompress - should succeed
    rpc.create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer, &owner])
        .await
        .unwrap();

    // Verify the amount was ADDED to existing balance
    let account_data = rpc.get_account(destination_pubkey).await.unwrap().unwrap();
    let token_account =
        spl_token_2022::state::Account::unpack_unchecked(&account_data.data[..165]).unwrap();
    assert_eq!(
        token_account.amount,
        existing_balance + amount,
        "Amount should be added to existing balance"
    );
}

/// Test that decompression to an account with existing delegate SUCCEEDS
/// and the existing delegate is PRESERVED (not overwritten).
#[tokio::test]
#[serial]
async fn test_decompress_has_delegate() {
    // Set up compressed token
    let (mut rpc, payer, mint_pubkey, owner, compressed_account, amount) =
        setup_compressed_token_for_decompress(&[ExtensionType::Pausable]).await;

    // Create destination with correct owner
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

    // Set delegate on destination (existing delegate that should be preserved)
    let existing_delegate = Keypair::new();
    let existing_delegated_amount = 500u64;
    set_invalid_destination_state(
        &mut rpc,
        destination_pubkey,
        None,
        Some(existing_delegate.pubkey()),
        Some(existing_delegated_amount),
        None,
    )
    .await;

    // Build decompress input with CompressedOnly extension data
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
            solana_token_account: destination_pubkey,
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

    // Execute decompress - should succeed
    rpc.create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer, &owner])
        .await
        .unwrap();

    // Verify the existing delegate was preserved (not overwritten)
    let account_data = rpc.get_account(destination_pubkey).await.unwrap().unwrap();
    let token_account =
        spl_token_2022::state::Account::unpack_unchecked(&account_data.data[..165]).unwrap();
    assert_eq!(
        token_account.delegate,
        solana_sdk::program_option::COption::Some(existing_delegate.pubkey()),
        "Existing delegate should be preserved"
    );
    assert_eq!(
        token_account.delegated_amount, existing_delegated_amount,
        "Existing delegated_amount should be preserved"
    );
}

/// Test that decompression to an account with existing delegated_amount SUCCEEDS.
/// This is covered by test_decompress_has_delegate but kept for explicit coverage.
#[tokio::test]
#[serial]
async fn test_decompress_non_zero_delegated_amount() {
    // Set up compressed token
    let (mut rpc, payer, mint_pubkey, owner, compressed_account, amount) =
        setup_compressed_token_for_decompress(&[ExtensionType::Pausable]).await;

    // Create destination with correct owner
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

    // Set non-zero delegated_amount on destination
    let delegate = Keypair::new();
    set_invalid_destination_state(
        &mut rpc,
        destination_pubkey,
        None,
        Some(delegate.pubkey()), // Need delegate for delegated_amount
        Some(500),               // Non-zero delegated_amount
        None,
    )
    .await;

    // Build decompress input with CompressedOnly extension data
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
            solana_token_account: destination_pubkey,
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

    // Execute decompress - should succeed
    rpc.create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer, &owner])
        .await
        .unwrap();
}

/// Test that decompression to an account with close_authority SUCCEEDS.
/// Close authority is no longer checked during decompress.
#[tokio::test]
#[serial]
async fn test_decompress_has_close_authority() {
    // Set up compressed token
    let (mut rpc, payer, mint_pubkey, owner, compressed_account, amount) =
        setup_compressed_token_for_decompress(&[ExtensionType::Pausable]).await;

    // Create destination with correct owner
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

    // Set close_authority on destination
    let close_authority = Keypair::new();
    set_invalid_destination_state(
        &mut rpc,
        destination_pubkey,
        None,
        None,
        None,
        Some(close_authority.pubkey()), // Has close_authority
    )
    .await;

    // Build decompress input with CompressedOnly extension data
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
            solana_token_account: destination_pubkey,
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

    // Execute decompress - should succeed (close_authority is not checked)
    rpc.create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer, &owner])
        .await
        .unwrap();

    // Verify the close_authority was preserved
    let account_data = rpc.get_account(destination_pubkey).await.unwrap().unwrap();
    let token_account =
        spl_token_2022::state::Account::unpack_unchecked(&account_data.data[..165]).unwrap();
    assert_eq!(
        token_account.close_authority,
        solana_sdk::program_option::COption::Some(close_authority.pubkey()),
        "Existing close_authority should be preserved"
    );
}

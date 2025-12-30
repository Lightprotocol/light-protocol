//! Tests for ATA CompressOnly decompress security.
//!
//! These tests verify that ATAs with CompressOnly extension can only be
//! decompressed to the exact same ATA pubkey that was originally compressed.

use light_client::indexer::Indexer;
use light_ctoken_interface::{
    instructions::extensions::{CompressedOnlyExtensionInstructionData, ExtensionInstructionData},
    state::{ExtensionStruct, TokenDataVersion},
};
use light_ctoken_sdk::{
    ctoken::{
        derive_ctoken_ata, CompressibleParams, CreateAssociatedCTokenAccount, CreateCTokenAccount,
        TransferSplToCtoken,
    },
    spl_interface::find_spl_interface_pda_with_index,
};
use light_program_test::{
    program_test::TestRpc, utils::assert::assert_rpc_error, LightProgramTest, ProgramTestConfig,
};
use light_test_utils::{
    mint_2022::{
        create_mint_22_with_extension_types, create_token_22_account, mint_spl_tokens_22,
        RESTRICTED_EXTENSIONS,
    },
    Rpc, RpcError,
};
use light_token_client::instructions::transfer2::{
    create_generic_transfer2_instruction, DecompressInput, Transfer2InstructionType,
};
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use spl_token_2022::extension::ExtensionType;

use super::shared::{set_ctoken_account_state, setup_extensions_test};

/// Expected error code for DecompressDestinationMismatch
const DECOMPRESS_DESTINATION_MISMATCH: u32 = 18057;
/// Expected error code for MintMismatch
const MINT_MISMATCH: u32 = 18058;

/// Setup context for ATA CompressOnly tests
struct AtaCompressedTokenContext {
    rpc: LightProgramTest,
    payer: Keypair,
    mint_pubkey: Pubkey,
    owner: Keypair,
    compressed_account: light_client::indexer::CompressedTokenAccount,
    amount: u64,
    ata_pubkey: Pubkey,
    ata_bump: u8,
}

/// Helper to set up a compressed token from an ATA with CompressOnly extension.
/// Creates an ATA with compression_only=true, funds it, and waits for compression.
async fn setup_ata_compressed_token(
    extensions: &[ExtensionType],
    with_delegate: Option<(&Keypair, u64)>, // delegate, delegated_amount
    is_frozen: bool,
) -> Result<AtaCompressedTokenContext, RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
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

    // Create ATA with compression_only=true
    let owner = Keypair::new();
    let (ata_pubkey, ata_bump) = derive_ctoken_ata(&owner.pubkey(), &mint_pubkey);

    let create_ata_ix =
        CreateAssociatedCTokenAccount::new(payer.pubkey(), owner.pubkey(), mint_pubkey)
            .with_compressible(CompressibleParams {
                compressible_config: rpc
                    .test_accounts
                    .funding_pool_config
                    .compressible_config_pda,
                rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
                pre_pay_num_epochs: 0, // Immediately compressible
                lamports_per_write: Some(100),
                compress_to_account_pubkey: None, // Auto-set for ATAs with compression_only
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: true,
            })
            .instruction()
            .map_err(|e| {
                RpcError::CustomError(format!("Failed to create ATA instruction: {:?}", e))
            })?;

    rpc.create_and_send_transaction(&[create_ata_ix], &payer.pubkey(), &[&payer])
        .await?;

    // Transfer tokens from SPL to ATA
    let has_restricted = extensions
        .iter()
        .any(|ext| RESTRICTED_EXTENSIONS.contains(ext));
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, has_restricted);

    let transfer_ix = TransferSplToCtoken {
        amount: mint_amount,
        spl_interface_pda_bump,
        decimals: 9,
        source_spl_token_account: spl_account,
        destination_ctoken_account: ata_pubkey,
        authority: payer.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: spl_token_2022::ID,
    }
    .instruction()
    .map_err(|e| {
        RpcError::CustomError(format!("Failed to create transfer instruction: {:?}", e))
    })?;

    rpc.create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer])
        .await?;

    // Optionally modify delegate/frozen state
    let delegate_pubkey = with_delegate.map(|(kp, _)| kp.pubkey());
    let delegated_amount = with_delegate.map(|(_, a)| a).unwrap_or(0);

    if with_delegate.is_some() || is_frozen {
        set_ctoken_account_state(
            &mut rpc,
            ata_pubkey,
            delegate_pubkey,
            delegated_amount,
            is_frozen,
        )
        .await?;
    }

    // Warp epoch to trigger forester compression
    rpc.warp_epoch_forward(30).await?;

    // Get compressed token accounts
    // For ATAs with compression_only=true, the compressed account owner is the ATA pubkey
    // The is_ata flag in the CompressedOnlyExtension enables ATA derivation verification during decompress
    let compressed_accounts = rpc
        .get_compressed_token_accounts_by_owner(&ata_pubkey, None, None)
        .await?
        .value
        .items;

    assert_eq!(
        compressed_accounts.len(),
        1,
        "Should have 1 compressed account (owner=ATA pubkey)"
    );

    Ok(AtaCompressedTokenContext {
        rpc,
        payer,
        mint_pubkey,
        owner,
        compressed_account: compressed_accounts[0].clone(),
        amount: mint_amount,
        ata_pubkey,
        ata_bump,
    })
}

/// Helper to attempt decompress with specific in_tlv settings
async fn attempt_decompress_with_tlv(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    owner: &Keypair,
    compressed_account: light_client::indexer::CompressedTokenAccount,
    amount: u64,
    destination_pubkey: Pubkey,
    in_tlv: Vec<Vec<ExtensionInstructionData>>,
) -> Result<solana_sdk::signature::Signature, RpcError> {
    let decompress_ix = create_generic_transfer2_instruction(
        rpc,
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
    .map_err(|e| {
        RpcError::CustomError(format!("Failed to create decompress instruction: {:?}", e))
    })?;

    rpc.create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[payer, owner])
        .await
}

/// Test that CompressAndClose for an ATA stores is_ata=1 and correct bump in CompressedOnlyExtension.
#[tokio::test]
#[serial]
async fn test_ata_compress_and_close_stores_is_ata() {
    let context = setup_ata_compressed_token(&[ExtensionType::Pausable], None, false)
        .await
        .unwrap();

    let token_data = &context.compressed_account.token;

    // Check tlv has CompressedOnlyExtension with is_ata=1
    let has_compressed_only_with_is_ata = token_data
        .tlv
        .as_ref()
        .map(|tlv| {
            tlv.iter()
                .any(|ext| matches!(ext, ExtensionStruct::CompressedOnly(e) if e.is_ata == 1))
        })
        .unwrap_or(false);

    assert!(
        has_compressed_only_with_is_ata,
        "CompressedOnlyExtension should have is_ata=1 for ATA"
    );

    // Check owner is ATA pubkey (not wallet owner) due to compress_to_pubkey behavior
    let owner_bytes: [u8; 32] = token_data.owner.to_bytes();
    assert_eq!(
        owner_bytes,
        context.ata_pubkey.to_bytes(),
        "Compressed account owner should be ATA pubkey (compress_to_pubkey)"
    );
}

/// Test that decompress to the correct ATA succeeds.
#[tokio::test]
#[serial]
async fn test_ata_decompress_to_correct_ata_succeeds() {
    let mut context = setup_ata_compressed_token(&[ExtensionType::Pausable], None, false)
        .await
        .unwrap();

    // Create destination ATA (idempotent - same address)
    let create_dest_ix = CreateAssociatedCTokenAccount::new(
        context.payer.pubkey(),
        context.owner.pubkey(),
        context.mint_pubkey,
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
    .idempotent()
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(
            &[create_dest_ix],
            &context.payer.pubkey(),
            &[&context.payer],
        )
        .await
        .unwrap();

    // Build decompress instruction with correct is_ata=true and bump
    let in_tlv = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 0,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 0,
            is_ata: true,
            bump: context.ata_bump,
            owner_index: 0,
        },
    )]];

    let result = attempt_decompress_with_tlv(
        &mut context.rpc,
        &context.payer,
        &context.owner,
        context.compressed_account.clone(),
        context.amount,
        context.ata_pubkey,
        in_tlv,
    )
    .await;
    println!("Decompress result: {:?}", result);
    assert!(result.is_ok(), "Decompress to correct ATA should succeed");

    // Verify ATA has tokens restored
    use borsh::BorshDeserialize;
    use light_ctoken_interface::state::CToken;
    let dest_account = context
        .rpc
        .get_account(context.ata_pubkey)
        .await
        .unwrap()
        .unwrap();
    let dest_ctoken = CToken::deserialize(&mut &dest_account.data[..]).unwrap();
    assert_eq!(
        dest_ctoken.amount, context.amount,
        "Decompressed amount should match"
    );
}

/// Test that decompress to a different ATA (with same owner) fails.
#[tokio::test]
#[serial]
async fn test_ata_decompress_to_different_ata_fails() {
    let mut context = setup_ata_compressed_token(&[ExtensionType::Pausable], None, false)
        .await
        .unwrap();

    // Create a second mint
    let (mint2_keypair, _) = create_mint_22_with_extension_types(
        &mut context.rpc,
        &context.payer,
        9,
        &[ExtensionType::Pausable],
    )
    .await;
    let mint2_pubkey = mint2_keypair.pubkey();

    // Create ATA for same owner but different mint
    let (ata2_pubkey, _ata2_bump) = derive_ctoken_ata(&context.owner.pubkey(), &mint2_pubkey);

    let create_ata2_ix = CreateAssociatedCTokenAccount::new(
        context.payer.pubkey(),
        context.owner.pubkey(),
        mint2_pubkey,
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
            &[create_ata2_ix],
            &context.payer.pubkey(),
            &[&context.payer],
        )
        .await
        .unwrap();

    // Attempt to decompress mint1's compressed account to ata2 (different mint's ATA)
    let in_tlv = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 0,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 0,
            is_ata: true,
            bump: context.ata_bump, // Using original bump
            owner_index: 0,
        },
    )]];

    let result = attempt_decompress_with_tlv(
        &mut context.rpc,
        &context.payer,
        &context.owner,
        context.compressed_account.clone(),
        context.amount,
        ata2_pubkey, // Wrong ATA (different mint)
        in_tlv,
    )
    .await;

    // Mint check fails before ATA derivation check
    assert_rpc_error(result, 0, MINT_MISMATCH).unwrap();
}

/// Test that decompress from ATA to non-ATA account fails.
#[tokio::test]
#[serial]
async fn test_ata_decompress_to_non_ata_fails() {
    let mut context = setup_ata_compressed_token(&[ExtensionType::Pausable], None, false)
        .await
        .unwrap();

    // Create a regular (non-ATA) CToken account with same owner
    let regular_account_keypair = Keypair::new();
    let create_regular_ix = CreateCTokenAccount::new(
        context.payer.pubkey(),
        regular_account_keypair.pubkey(),
        context.mint_pubkey,
        context.owner.pubkey(),
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
            &[create_regular_ix],
            &context.payer.pubkey(),
            &[&context.payer, &regular_account_keypair],
        )
        .await
        .unwrap();

    // Attempt decompress to non-ATA account with is_ata=true
    let in_tlv = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 0,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 0,
            is_ata: true,
            bump: context.ata_bump,
            owner_index: 0,
        },
    )]];

    let result = attempt_decompress_with_tlv(
        &mut context.rpc,
        &context.payer,
        &context.owner,
        context.compressed_account.clone(),
        context.amount,
        regular_account_keypair.pubkey(), // Non-ATA account
        in_tlv,
    )
    .await;

    assert_rpc_error(result, 0, DECOMPRESS_DESTINATION_MISMATCH).unwrap();
}

/// Test that decompress with wrong bump fails.
#[tokio::test]
#[serial]
async fn test_ata_decompress_with_wrong_bump_fails() {
    let mut context = setup_ata_compressed_token(&[ExtensionType::Pausable], None, false)
        .await
        .unwrap();

    // Create destination ATA
    let create_dest_ix = CreateAssociatedCTokenAccount::new(
        context.payer.pubkey(),
        context.owner.pubkey(),
        context.mint_pubkey,
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
    .idempotent()
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(
            &[create_dest_ix],
            &context.payer.pubkey(),
            &[&context.payer],
        )
        .await
        .unwrap();

    // Use wrong bump
    let wrong_bump = if context.ata_bump == 255 {
        context.ata_bump - 1
    } else {
        context.ata_bump + 1
    };

    let in_tlv = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 0,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 0,
            is_ata: true,
            bump: wrong_bump, // Wrong bump!
            owner_index: 0,
        },
    )]];

    let result = attempt_decompress_with_tlv(
        &mut context.rpc,
        &context.payer,
        &context.owner,
        context.compressed_account.clone(),
        context.amount,
        context.ata_pubkey,
        in_tlv,
    )
    .await;

    // Wrong bump causes ATA derivation to fail with InvalidSeeds
    result.expect_err("Decompress with wrong bump should fail");
}

/// Test that decompress to account with existing balance adds to it.
#[tokio::test]
#[serial]
async fn test_decompress_to_account_with_balance_adds() {
    let mut context = setup_ata_compressed_token(&[ExtensionType::Pausable], None, false)
        .await
        .unwrap();

    // Create destination ATA
    let create_dest_ix = CreateAssociatedCTokenAccount::new(
        context.payer.pubkey(),
        context.owner.pubkey(),
        context.mint_pubkey,
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
    .idempotent()
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(
            &[create_dest_ix],
            &context.payer.pubkey(),
            &[&context.payer],
        )
        .await
        .unwrap();

    // Pre-fund destination with some tokens by modifying account state
    let pre_existing_amount = 500_000_000u64;
    {
        use anchor_spl::token_2022::spl_token_2022;
        use solana_sdk::program_pack::Pack;

        let mut account_info = context
            .rpc
            .get_account(context.ata_pubkey)
            .await
            .unwrap()
            .unwrap();

        let mut spl_account =
            spl_token_2022::state::Account::unpack_unchecked(&account_info.data[..165]).unwrap();
        spl_account.amount = pre_existing_amount;
        spl_token_2022::state::Account::pack(spl_account, &mut account_info.data[..165]).unwrap();
        context.rpc.set_account(context.ata_pubkey, account_info);
    }

    // Decompress
    let in_tlv = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 0,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 0,
            is_ata: true,
            bump: context.ata_bump,
            owner_index: 0,
        },
    )]];

    let result = attempt_decompress_with_tlv(
        &mut context.rpc,
        &context.payer,
        &context.owner,
        context.compressed_account.clone(),
        context.amount,
        context.ata_pubkey,
        in_tlv,
    )
    .await;

    assert!(result.is_ok(), "Decompress should succeed");

    // Verify final balance is sum of existing + decompressed
    use borsh::BorshDeserialize;
    use light_ctoken_interface::state::CToken;
    let dest_account = context
        .rpc
        .get_account(context.ata_pubkey)
        .await
        .unwrap()
        .unwrap();
    let dest_ctoken = CToken::deserialize(&mut &dest_account.data[..]).unwrap();
    assert_eq!(
        dest_ctoken.amount,
        pre_existing_amount + context.amount,
        "Final balance should be sum of existing ({}) + decompressed ({})",
        pre_existing_amount,
        context.amount
    );
}

/// Test that decompress skips delegate restoration if destination already has delegate.
#[tokio::test]
#[serial]
async fn test_decompress_skips_delegate_if_destination_has_delegate() {
    // Create compressed token with delegate=Alice, delegated_amount=50
    let alice = Keypair::new();
    let mut context = setup_ata_compressed_token(
        &[ExtensionType::Pausable],
        Some((&alice, 50_000_000)),
        false,
    )
    .await
    .unwrap();

    // Create destination ATA
    let create_dest_ix = CreateAssociatedCTokenAccount::new(
        context.payer.pubkey(),
        context.owner.pubkey(),
        context.mint_pubkey,
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
    .idempotent()
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(
            &[create_dest_ix],
            &context.payer.pubkey(),
            &[&context.payer],
        )
        .await
        .unwrap();

    // Set destination delegate=Bob, delegated_amount=30
    let bob = Keypair::new();
    set_ctoken_account_state(
        &mut context.rpc,
        context.ata_pubkey,
        Some(bob.pubkey()),
        30_000_000,
        false,
    )
    .await
    .unwrap();

    // Decompress with Alice's delegate info
    let in_tlv = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 50_000_000,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 0,
            is_ata: true,
            bump: context.ata_bump,
            owner_index: 0,
        },
    )]];

    let result = attempt_decompress_with_tlv(
        &mut context.rpc,
        &context.payer,
        &context.owner,
        context.compressed_account.clone(),
        context.amount,
        context.ata_pubkey,
        in_tlv,
    )
    .await;

    assert!(result.is_ok(), "Decompress should succeed");

    // Verify destination delegate is still Bob (not Alice)
    use borsh::BorshDeserialize;
    use light_ctoken_interface::state::CToken;
    let dest_account = context
        .rpc
        .get_account(context.ata_pubkey)
        .await
        .unwrap()
        .unwrap();
    let dest_ctoken = CToken::deserialize(&mut &dest_account.data[..]).unwrap();

    assert_eq!(
        dest_ctoken.delegate,
        Some(bob.pubkey().to_bytes().into()),
        "Delegate should still be Bob, not restored to Alice"
    );
    assert_eq!(
        dest_ctoken.delegated_amount, 30_000_000,
        "Delegated amount should still be 30M, not restored to 50M"
    );
}

/// Test that non-ATA CompressOnly decompress keeps current owner-match behavior.
#[tokio::test]
#[serial]
async fn test_non_ata_compress_only_decompress() {
    // Setup using existing setup_extensions_test for regular accounts
    let mut context = setup_extensions_test(&[ExtensionType::Pausable])
        .await
        .unwrap();
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    // Create SPL Token-2022 account and mint tokens
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

    // Create regular (non-ATA) CToken account with compression_only=true
    let owner = Keypair::new();
    let account_keypair = Keypair::new();
    let ctoken_account = account_keypair.pubkey();

    let create_ix =
        CreateCTokenAccount::new(payer.pubkey(), ctoken_account, mint_pubkey, owner.pubkey())
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
                pre_pay_num_epochs: 0,
                lamports_per_write: Some(100),
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: true,
            })
            .instruction()
            .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer, &account_keypair])
        .await
        .unwrap();

    // Transfer tokens to CToken
    let has_restricted = [ExtensionType::Pausable]
        .iter()
        .any(|ext| RESTRICTED_EXTENSIONS.contains(ext));
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, has_restricted);

    let transfer_ix = TransferSplToCtoken {
        amount: mint_amount,
        spl_interface_pda_bump,
        decimals: 9,
        source_spl_token_account: spl_account,
        destination_ctoken_account: ctoken_account,
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

    // Warp epoch to trigger compression
    context.rpc.warp_epoch_forward(30).await.unwrap();

    // Verify compressed account has is_ata=0 and owner = wallet owner
    let compressed_accounts = context
        .rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(compressed_accounts.len(), 1);
    let token_data = &compressed_accounts[0].token;

    // Check is_ata=0
    let has_compressed_only_with_is_ata_0 = token_data
        .tlv
        .as_ref()
        .map(|tlv| {
            tlv.iter()
                .any(|ext| matches!(ext, ExtensionStruct::CompressedOnly(e) if e.is_ata == 0))
        })
        .unwrap_or(false);

    assert!(
        has_compressed_only_with_is_ata_0,
        "Non-ATA CompressedOnlyExtension should have is_ata=0"
    );

    // Check owner is wallet owner (not account pubkey)
    let owner_bytes: [u8; 32] = token_data.owner.to_bytes();
    assert_eq!(
        owner_bytes,
        owner.pubkey().to_bytes(),
        "Non-ATA compressed account owner should be wallet owner"
    );

    // Create new CToken account with SAME owner for decompress
    let new_account_keypair = Keypair::new();
    let create_new_ix = CreateCTokenAccount::new(
        payer.pubkey(),
        new_account_keypair.pubkey(),
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
            &[create_new_ix],
            &payer.pubkey(),
            &[&payer, &new_account_keypair],
        )
        .await
        .unwrap();

    // Decompress with is_ata=false
    let in_tlv = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 0,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 0,
            is_ata: false, // Non-ATA
            bump: 0,       // Not used for non-ATA
            owner_index: 0,
        },
    )]];

    let result = attempt_decompress_with_tlv(
        &mut context.rpc,
        &payer,
        &owner,
        compressed_accounts[0].clone(),
        mint_amount,
        new_account_keypair.pubkey(),
        in_tlv,
    )
    .await;

    assert!(
        result.is_ok(),
        "Non-ATA decompress to same-owner account should succeed"
    );

    // Verify tokens restored
    use borsh::BorshDeserialize;
    use light_ctoken_interface::state::CToken;
    let dest_account = context
        .rpc
        .get_account(new_account_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let dest_ctoken = CToken::deserialize(&mut &dest_account.data[..]).unwrap();
    assert_eq!(dest_ctoken.amount, mint_amount);
}

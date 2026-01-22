//! Tests for ATA CompressOnly decompress security.
//!
//! These tests verify that ATAs with CompressOnly extension can only be
//! decompressed to the exact same ATA pubkey that was originally compressed.

use light_client::indexer::Indexer;
use light_compressed_token_sdk::spl_interface::find_spl_interface_pda_with_index;
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
use light_token::instruction::{
    derive_token_ata, CompressibleParams, CreateAssociatedTokenAccount, CreateTokenAccount,
    TransferFromSpl,
};
use light_token_client::instructions::transfer2::{
    create_generic_transfer2_instruction, DecompressInput, Transfer2InstructionType,
};
use light_token_interface::{
    instructions::extensions::{CompressedOnlyExtensionInstructionData, ExtensionInstructionData},
    state::{ExtensionStruct, TokenDataVersion},
};
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use spl_token_2022::extension::ExtensionType;

use super::shared::{set_ctoken_account_state, setup_extensions_test};

/// Expected error code for DecompressDestinationMismatch
const DECOMPRESS_DESTINATION_MISMATCH: u32 = 18057;
/// Expected error code for MintMismatch
const MINT_MISMATCH: u32 = 18058;
/// Expected error code for DecompressAmountMismatch
const DECOMPRESS_AMOUNT_MISMATCH: u32 = 18064;

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
    let (ata_pubkey, ata_bump) = derive_token_ata(&owner.pubkey(), &mint_pubkey);

    let create_ata_ix =
        CreateAssociatedTokenAccount::new(payer.pubkey(), owner.pubkey(), mint_pubkey)
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

    let transfer_ix = TransferFromSpl {
        amount: mint_amount,
        spl_interface_pda_bump,
        decimals: 9,
        source_spl_token_account: spl_account,
        destination: ata_pubkey,
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
    let create_dest_ix = CreateAssociatedTokenAccount::new(
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
    use light_token_interface::state::Token;
    let dest_account = context
        .rpc
        .get_account(context.ata_pubkey)
        .await
        .unwrap()
        .unwrap();
    let dest_ctoken = Token::deserialize(&mut &dest_account.data[..]).unwrap();
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
    let (ata2_pubkey, _ata2_bump) = derive_token_ata(&context.owner.pubkey(), &mint2_pubkey);

    let create_ata2_ix = CreateAssociatedTokenAccount::new(
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

    // Create a regular (non-ATA) Light Token account with same owner
    let regular_account_keypair = Keypair::new();
    let create_regular_ix = CreateTokenAccount::new(
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
    let create_dest_ix = CreateAssociatedTokenAccount::new(
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
    let create_dest_ix = CreateAssociatedTokenAccount::new(
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
    use light_token_interface::state::Token;
    let dest_account = context
        .rpc
        .get_account(context.ata_pubkey)
        .await
        .unwrap()
        .unwrap();
    let dest_ctoken = Token::deserialize(&mut &dest_account.data[..]).unwrap();
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
    let create_dest_ix = CreateAssociatedTokenAccount::new(
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
    use light_token_interface::state::Token;
    let dest_account = context
        .rpc
        .get_account(context.ata_pubkey)
        .await
        .unwrap()
        .unwrap();
    let dest_ctoken = Token::deserialize(&mut &dest_account.data[..]).unwrap();

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

/// Test that decompress with mismatched amount fails for ATA.
/// The compression_amount in the instruction must match the input token data amount.
#[tokio::test]
#[serial]
async fn test_ata_decompress_with_mismatched_amount_fails() {
    use borsh::BorshSerialize;
    use light_compressed_account::compressed_account::PackedMerkleContext;
    use light_compressed_token_sdk::compressed_token::transfer2::account_metas::{
        get_transfer2_instruction_account_metas, Transfer2AccountsMetaConfig,
    };
    use light_sdk::instruction::PackedAccounts;
    use light_token_interface::{
        instructions::transfer2::{
            CompressedTokenInstructionDataTransfer2, Compression, CompressionMode,
            MultiInputTokenDataWithContext,
        },
        TRANSFER2,
    };
    use solana_sdk::instruction::Instruction;

    let mut context = setup_ata_compressed_token(&[ExtensionType::Pausable], None, false)
        .await
        .unwrap();

    // Create destination ATA
    let create_dest_ix = CreateAssociatedTokenAccount::new(
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

    // Build instruction data directly to control compressions without SDK adding change outputs
    let compressed_account = &context.compressed_account;
    let mut packed_accounts = PackedAccounts::default();

    // Add merkle tree and output queue
    let merkle_tree = compressed_account.account.tree_info.tree;
    let queue = compressed_account.account.tree_info.queue;
    let tree_index = packed_accounts.insert_or_get(merkle_tree);
    let queue_index = packed_accounts.insert_or_get(queue);

    // Add mint and wallet owner (for signing and TLV owner_index)
    let mint_index = packed_accounts.insert_or_get_read_only(compressed_account.token.mint);
    let wallet_owner_index =
        packed_accounts.insert_or_get_config(context.owner.pubkey(), true, false);

    // Add Light Token ATA recipient account - this is also the compressed token owner for ATAs
    let ctoken_ata_index = packed_accounts.insert_or_get_config(context.ata_pubkey, false, true);

    // Create input token data with FULL amount (what merkle proof verifies)
    // For ATA compressed tokens, owner is the ATA pubkey (not wallet)
    let has_delegate = compressed_account.token.delegate.is_some();
    let delegate_index = if has_delegate {
        packed_accounts
            .insert_or_get_read_only(compressed_account.token.delegate.unwrap_or_default())
    } else {
        0
    };

    let input_token_data = vec![MultiInputTokenDataWithContext {
        owner: ctoken_ata_index, // ATA pubkey is the compressed token owner
        amount: compressed_account.token.amount, // Full amount for merkle proof
        has_delegate,
        delegate: delegate_index,
        mint: mint_index,
        version: 3, // ShaFlat
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: tree_index,
            queue_pubkey_index: queue_index,
            leaf_index: compressed_account.account.leaf_index,
            prove_by_index: true,
        },
        root_index: 0,
    }];

    // Create compression with WRONG amount (mismatch!)
    // Input has full amount but compression claims only half
    let wrong_decompress_amount = context.amount / 2;
    let compressions = vec![
        Compression {
            mode: CompressionMode::Decompress,
            amount: wrong_decompress_amount, // WRONG: doesn't match input amount
            mint: mint_index,
            source_or_recipient: ctoken_ata_index,
            authority: 0,
            pool_account_index: 0,
            pool_index: 0,
            bump: 0,
            decimals: 9,
        },
        Compression {
            mode: CompressionMode::Decompress,
            amount: wrong_decompress_amount, // WRONG: doesn't match input amount
            mint: mint_index,
            source_or_recipient: ctoken_ata_index,
            authority: 0,
            pool_account_index: 0,
            pool_index: 0,
            bump: 0,
            decimals: 9,
        },
    ];

    // Build in_tlv for CompressedOnly extension
    // owner_index in TLV is the wallet owner (who can sign), not the ATA
    let in_tlv = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 0,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 0,
            is_ata: true,
            bump: context.ata_bump,
            owner_index: wallet_owner_index,
        },
    )]];

    // Build instruction data directly
    let instruction_data = CompressedTokenInstructionDataTransfer2 {
        with_transaction_hash: false,
        with_lamports_change_account_merkle_tree_index: false,
        lamports_change_account_merkle_tree_index: 0,
        lamports_change_account_owner_index: 0,
        output_queue: queue_index,
        proof:
            light_compressed_account::instruction_data::compressed_proof::ValidityProof::default()
                .into(),
        in_token_data: input_token_data,
        out_token_data: vec![], // No compressed outputs
        in_lamports: None,
        out_lamports: None,
        in_tlv: Some(in_tlv),
        out_tlv: None,
        compressions: Some(compressions),
        cpi_context: None,
        max_top_up: 0,
    };

    // Serialize instruction data
    let serialized = instruction_data.try_to_vec().unwrap();
    let mut data = Vec::with_capacity(1 + serialized.len());
    data.push(TRANSFER2);
    data.extend(serialized);

    // Get account metas
    let (account_metas, _, _) = packed_accounts.to_account_metas();
    let meta_config = Transfer2AccountsMetaConfig::new(context.payer.pubkey(), account_metas);
    let instruction_account_metas = get_transfer2_instruction_account_metas(meta_config);

    let decompress_ix = Instruction {
        program_id: light_token_interface::LIGHT_TOKEN_PROGRAM_ID.into(),
        accounts: instruction_account_metas,
        data,
    };

    let result = context
        .rpc
        .create_and_send_transaction(
            &[decompress_ix],
            &context.payer.pubkey(),
            &[&context.payer, &context.owner],
        )
        .await;

    assert_rpc_error(result, 0, DECOMPRESS_AMOUNT_MISMATCH).unwrap();
}

/// Test that multiple compress-decompress cycles work correctly for the same ATA.
/// Creates the same ATA twice, each time compressing it, then decompresses both
/// compressed accounts back to the ATA in a single Transfer2 instruction.
#[tokio::test]
#[serial]
async fn test_ata_multiple_compress_decompress_cycles() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create mint with Pausable extension (restricted, requires compression_only)
    let extensions = &[ExtensionType::Pausable];
    let (mint_keypair, _) =
        create_mint_22_with_extension_types(&mut rpc, &payer, 9, extensions).await;
    let mint_pubkey = mint_keypair.pubkey();

    // Create SPL Token-2022 account and mint tokens for funding
    let spl_account =
        create_token_22_account(&mut rpc, &payer, &mint_pubkey, &payer.pubkey()).await;
    let total_mint_amount = 10_000_000_000u64;
    mint_spl_tokens_22(
        &mut rpc,
        &payer,
        &mint_pubkey,
        &spl_account,
        total_mint_amount,
    )
    .await;

    // Setup wallet owner and derive ATA
    let wallet = Keypair::new();
    let (ata_pubkey, ata_bump) = derive_token_ata(&wallet.pubkey(), &mint_pubkey);

    let amount1 = 100_000_000u64;
    let amount2 = 200_000_000u64;

    // ========== CYCLE 1 ==========
    println!("=== Cycle 1: Create ATA, fund, compress ===");

    // Create ATA with compression_only=true
    let create_ata_ix =
        CreateAssociatedTokenAccount::new(payer.pubkey(), wallet.pubkey(), mint_pubkey)
            .with_compressible(CompressibleParams {
                compressible_config: rpc
                    .test_accounts
                    .funding_pool_config
                    .compressible_config_pda,
                rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
                pre_pay_num_epochs: 0, // Immediately compressible
                lamports_per_write: Some(100),
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: true,
            })
            .instruction()
            .unwrap();

    rpc.create_and_send_transaction(&[create_ata_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Transfer tokens from SPL to ATA
    let has_restricted = extensions
        .iter()
        .any(|ext| RESTRICTED_EXTENSIONS.contains(ext));
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, has_restricted);

    let transfer_ix1 = TransferFromSpl {
        amount: amount1,
        spl_interface_pda_bump,
        decimals: 9,
        source_spl_token_account: spl_account,
        destination: ata_pubkey,
        authority: payer.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: spl_token_2022::ID,
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[transfer_ix1], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Warp to trigger compression
    rpc.warp_epoch_forward(30).await.unwrap();

    // Verify ATA is closed
    let ata_after_cycle1 = rpc.get_account(ata_pubkey).await.unwrap();
    assert!(
        ata_after_cycle1.is_none(),
        "ATA should be closed after cycle 1 compression"
    );

    // ========== CYCLE 2 ==========
    println!("=== Cycle 2: Create ATA again, fund, compress ===");

    // Create ATA again (same address)
    let create_ata_ix2 =
        CreateAssociatedTokenAccount::new(payer.pubkey(), wallet.pubkey(), mint_pubkey)
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

    rpc.create_and_send_transaction(&[create_ata_ix2], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Transfer tokens from SPL to ATA
    let transfer_ix2 = TransferFromSpl {
        amount: amount2,
        spl_interface_pda_bump,
        decimals: 9,
        source_spl_token_account: spl_account,
        destination: ata_pubkey,
        authority: payer.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: spl_token_2022::ID,
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[transfer_ix2], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Warp to trigger compression
    rpc.warp_epoch_forward(30).await.unwrap();

    // Verify ATA is closed again
    let ata_after_cycle2 = rpc.get_account(ata_pubkey).await.unwrap();
    assert!(
        ata_after_cycle2.is_none(),
        "ATA should be closed after cycle 2 compression"
    );

    // ========== VERIFY COMPRESSED ACCOUNTS ==========
    println!("=== Verifying compressed accounts ===");

    // For ATAs with compression_only=true, the compressed account owner is the ATA pubkey
    let compressed_accounts = rpc
        .get_compressed_token_accounts_by_owner(&ata_pubkey, None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(
        compressed_accounts.len(),
        2,
        "Should have 2 compressed token accounts (one from each cycle)"
    );

    // Verify both have CompressedOnly extension with is_ata=1
    for (i, account) in compressed_accounts.iter().enumerate() {
        let has_compressed_only_with_is_ata = account
            .token
            .tlv
            .as_ref()
            .map(|tlv| {
                tlv.iter()
                    .any(|ext| matches!(ext, ExtensionStruct::CompressedOnly(e) if e.is_ata == 1))
            })
            .unwrap_or(false);

        assert!(
            has_compressed_only_with_is_ata,
            "Compressed account {} should have CompressedOnly extension with is_ata=1",
            i
        );

        // Verify owner is ATA pubkey
        let owner_bytes: [u8; 32] = account.token.owner.to_bytes();
        assert_eq!(
            owner_bytes,
            ata_pubkey.to_bytes(),
            "Compressed account {} owner should be ATA pubkey",
            i
        );
    }

    // Verify amounts
    let amounts: Vec<u64> = compressed_accounts.iter().map(|a| a.token.amount).collect();
    assert!(
        amounts.contains(&amount1) && amounts.contains(&amount2),
        "Should have compressed accounts with amounts {} and {}, got {:?}",
        amount1,
        amount2,
        amounts
    );

    // ========== DECOMPRESS BOTH ==========
    println!("=== Decompressing both to same ATA ===");

    // Create ATA again (destination for decompress)
    let create_ata_ix3 =
        CreateAssociatedTokenAccount::new(payer.pubkey(), wallet.pubkey(), mint_pubkey)
            .with_compressible(CompressibleParams {
                compressible_config: rpc
                    .test_accounts
                    .funding_pool_config
                    .compressible_config_pda,
                rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
                pre_pay_num_epochs: 2, // More epochs so it won't be compressed immediately
                lamports_per_write: Some(100),
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: true,
            })
            .idempotent()
            .instruction()
            .unwrap();

    rpc.create_and_send_transaction(&[create_ata_ix3], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Build Transfer2 with TWO Decompress operations to the same ATA
    // Each decompress needs a unique compression_index
    let in_tlv1 = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 0,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 0, // First decompress
            is_ata: true,
            bump: ata_bump,
            owner_index: 0, // Will be updated by create_generic_transfer2_instruction
        },
    )]];

    let in_tlv2 = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 0,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 1, // Second decompress - different index
            is_ata: true,
            bump: ata_bump,
            owner_index: 0, // Will be updated by create_generic_transfer2_instruction
        },
    )]];

    let decompress_ix = create_generic_transfer2_instruction(
        &mut rpc,
        vec![
            Transfer2InstructionType::Decompress(DecompressInput {
                compressed_token_account: vec![compressed_accounts[0].clone()],
                decompress_amount: compressed_accounts[0].token.amount,
                solana_token_account: ata_pubkey,
                amount: compressed_accounts[0].token.amount,
                pool_index: None,
                decimals: 9,
                in_tlv: Some(in_tlv1),
            }),
            Transfer2InstructionType::Decompress(DecompressInput {
                compressed_token_account: vec![compressed_accounts[1].clone()],
                decompress_amount: compressed_accounts[1].token.amount,
                solana_token_account: ata_pubkey,
                amount: compressed_accounts[1].token.amount,
                pool_index: None,
                decimals: 9,
                in_tlv: Some(in_tlv2),
            }),
        ],
        payer.pubkey(),
        true,
    )
    .await
    .unwrap();

    // For ATA decompress, wallet owner signs (not ATA pubkey)
    rpc.create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer, &wallet])
        .await
        .unwrap();

    // ========== VERIFY FINAL STATE ==========
    println!("=== Verifying final state ===");

    // Verify ATA has combined balance
    use borsh::BorshDeserialize;
    use light_token_interface::state::Token;

    let ata_account = rpc.get_account(ata_pubkey).await.unwrap().unwrap();
    let ata_ctoken = Token::deserialize(&mut &ata_account.data[..]).unwrap();

    assert_eq!(
        ata_ctoken.amount,
        amount1 + amount2,
        "ATA should have combined balance of {} + {} = {}, got {}",
        amount1,
        amount2,
        amount1 + amount2,
        ata_ctoken.amount
    );

    // Verify no more compressed token accounts
    let remaining = rpc
        .get_compressed_token_accounts_by_owner(&ata_pubkey, None, None)
        .await
        .unwrap()
        .value
        .items;

    assert!(
        remaining.is_empty(),
        "All compressed accounts should be consumed, got {} remaining",
        remaining.len()
    );

    println!(
        "Successfully completed ATA multiple compress-decompress cycles test. Final balance: {}",
        ata_ctoken.amount
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

    // Create regular (non-ATA) Light Token account with compression_only=true
    let owner = Keypair::new();
    let account_keypair = Keypair::new();
    let ctoken_account = account_keypair.pubkey();

    let create_ix =
        CreateTokenAccount::new(payer.pubkey(), ctoken_account, mint_pubkey, owner.pubkey())
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

    // Transfer tokens to Light Token
    let has_restricted = [ExtensionType::Pausable]
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

    // Create new Light Token account with SAME owner for decompress
    let new_account_keypair = Keypair::new();
    let create_new_ix = CreateTokenAccount::new(
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
    use light_token_interface::state::Token;
    let dest_account = context
        .rpc
        .get_account(new_account_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let dest_ctoken = Token::deserialize(&mut &dest_account.data[..]).unwrap();
    assert_eq!(dest_ctoken.amount, mint_amount);
}

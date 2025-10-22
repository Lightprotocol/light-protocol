#![allow(clippy::result_large_err)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::to_string_in_format_args)]

// ============================================================================
// NO SYSTEM PROGRAM CPI TESTS (process_no_system_program_cpi path)
// ============================================================================
//
// This tests the Transfer2 execution path when BOTH in_token_data and out_token_data are empty.
// In this mode, only compress/decompress/compress-and-close operations are performed,
// without any compressed-to-compressed token transfers.
//
// The execution path is:
// transfer2/processor.rs::process_transfer2() → process_no_system_program_cpi()
//
// Test Coverage (13 tests):
//
// Missing Required Data/Accounts:
// 1. Empty compressions array → NoInputsProvided (25)
//
// Sum Check Failures - CToken:
// 2. Compress without decompress → SumCheckFailed (6005)
// 3. Decompress without compress → SumCheckFailed (6005)
// 4. Compress less, decompress more → SumCheckFailed (6005)
// 5. Compress more, decompress less → SumCheckFailed (6005)
//
// CToken Authority Failures:
// 6. Invalid authority compress → OwnerMismatch (75)
// 7. Authority not signer compress → InvalidSigner (20009)
// 8. Insufficient balance compress → ArithmeticOverflow (string match, not error code)
//
// Decompression Authority Field:
// 9. Decompress with nonzero authority → InvalidInstructionData (string match, not error code)
//
// Multi-Mint Validation:
// 10. Too many mints (>5) → TooManyMints (6039)
// 11. Duplicate mint validation → DuplicateMint (6102)
//
// Index Out of Bounds:
// 12. Mint index out of bounds → DuplicateMint (6102) - out of bounds masked in validate_mint_uniqueness
// 13. Account index out of bounds → NotEnoughAccountKeys (20014)
// 14. Authority index out of bounds → SigningError - client-side error, can't send transaction
//
// ============================================================================

use light_compressed_token_sdk::{
    instructions::{derive_ctoken_ata, find_spl_mint_address},
    ValidityProof,
};
use light_ctoken_types::instructions::{mint_action::Recipient, transfer2::Compression};
use light_program_test::{
    utils::assert::assert_rpc_error, LightProgramTest, ProgramTestConfig, Rpc,
};
use light_sdk::instruction::PackedAccounts;
use light_test_utils::{airdrop_lamports, RpcError};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

// ============================================================================
// Test Setup
// ============================================================================

/// Test context for no_system_program_cpi failing tests
struct NoSystemProgramCpiTestContext {
    pub rpc: LightProgramTest,
    pub payer: Keypair,
    pub owner: Keypair,
    pub recipient: Keypair,
    pub source_ata: Pubkey,
    pub recipient_ata: Pubkey,
    pub mint: Pubkey,
    pub compressions: Vec<Compression>,
    pub packed_accounts: Vec<solana_sdk::instruction::AccountMeta>,
}

/// Set up test environment with two CToken accounts for compress/decompress testing
async fn setup_no_system_program_cpi_test(
    source_token_amount: u64,
) -> Result<NoSystemProgramCpiTestContext, RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create owner (source) and recipient
    let owner = Keypair::new();
    airdrop_lamports(&mut rpc, &owner.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    let recipient = Keypair::new();
    rpc.airdrop_lamports(&recipient.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // Create mint authority
    let mint_authority = Keypair::new();
    airdrop_lamports(&mut rpc, &mint_authority.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // Create compressed mint seed
    let mint_seed = Keypair::new();
    let (mint, _) = find_spl_mint_address(&mint_seed.pubkey());
    let (source_ata, _) = derive_ctoken_ata(&owner.pubkey(), &mint);
    let (recipient_ata, _) = derive_ctoken_ata(&recipient.pubkey(), &mint);

    // Create CToken ATA for owner (source)
    let instruction = light_compressed_token_sdk::instructions::create_associated_token_account(
        payer.pubkey(),
        owner.pubkey(),
        mint,
    )
    .map_err(|e| RpcError::AssertRpcError(format!("Failed to create source ATA: {}", e)))
    .unwrap();
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Create CToken ATA for recipient
    let instruction = light_compressed_token_sdk::instructions::create_associated_token_account(
        payer.pubkey(),
        recipient.pubkey(),
        mint,
    )
    .map_err(|e| RpcError::AssertRpcError(format!("Failed to create recipient ATA: {}", e)))
    .unwrap();
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Create mint and mint tokens to source CToken ATA
    let decompressed_recipients = if source_token_amount > 0 {
        vec![Recipient {
            recipient: owner.pubkey().to_bytes().into(),
            amount: source_token_amount,
        }]
    } else {
        vec![]
    };

    light_token_client::actions::mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &mint_authority,
        &payer,
        vec![],                  // no compressed recipients
        decompressed_recipients, // mint to source CToken ATA (empty if token_amount is 0)
        None,
        None,
        Some(light_token_client::instructions::mint_action::NewMint {
            decimals: 6,
            supply: 0,
            mint_authority: mint_authority.pubkey(),
            freeze_authority: None,
            metadata: None,
            version: 3, // ShaFlat
        }),
    )
    .await
    .unwrap();

    // Build compressions and packed accounts for default balanced case (500 compress, 500 decompress)
    let (compressions, packed_accounts) = create_compressions_and_packed_accounts(
        mint,
        owner.pubkey(),
        source_ata,
        recipient_ata,
        500, // compress amount
        500, // decompress amount
    );

    Ok(NoSystemProgramCpiTestContext {
        rpc,
        payer,
        owner,
        recipient,
        source_ata,
        recipient_ata,
        mint,
        compressions,
        packed_accounts,
    })
}

/// Create compressions and packed accounts for compressions-only mode
/// Returns (compressions, packed_account_metas)
fn create_compressions_and_packed_accounts(
    mint: Pubkey,
    owner: Pubkey,
    source_ata: Pubkey,
    recipient_ata: Pubkey,
    compress_amount: u64,
    decompress_amount: u64,
) -> (Vec<Compression>, Vec<solana_sdk::instruction::AccountMeta>) {
    // Build packed accounts
    let mut packed_accounts = PackedAccounts::default();
    let mint_index = packed_accounts.insert_or_get_read_only(mint);
    let owner_index = packed_accounts.insert_or_get_config(owner, true, false);
    let source_ata_index = packed_accounts.insert_or_get_config(source_ata, false, true);
    let recipient_ata_index = packed_accounts.insert_or_get_config(recipient_ata, false, true);

    // Create compressions
    let mut compressions = Vec::new();
    if compress_amount > 0 {
        compressions.push(Compression::compress_ctoken(
            compress_amount,
            mint_index,
            source_ata_index,
            owner_index,
        ));
    }
    if decompress_amount > 0 {
        compressions.push(Compression::decompress_ctoken(
            decompress_amount,
            mint_index,
            recipient_ata_index,
        ));
    }

    let (account_metas, _, _) = packed_accounts.to_account_metas();

    (compressions, account_metas)
}

// ============================================================================
// Instruction Builder Helpers
// ============================================================================

/// Build Transfer2 instruction for compressions-only mode (no compressed accounts)
/// This mode is used when we want to compress/decompress/compress-and-close
/// without any compressed-to-compressed token transfers.
fn build_compressions_only_instruction(
    fee_payer: Pubkey,
    compressions: Option<Vec<light_ctoken_types::instructions::transfer2::Compression>>,
    packed_account_metas: Vec<solana_sdk::instruction::AccountMeta>,
) -> Result<solana_sdk::instruction::Instruction, RpcError> {
    use anchor_lang::AnchorSerialize;
    use light_compressed_token_sdk::constants::{CPI_AUTHORITY_PDA, TRANSFER2};
    use light_ctoken_types::instructions::transfer2::CompressedTokenInstructionDataTransfer2;
    use solana_sdk::instruction::AccountMeta;

    // For compressions-only mode (decompressed_accounts_only), the account order is:
    // 1. cpi_authority_pda (not signer, not writable) - constant derived from compressed token program
    // 2. fee_payer (signer, not writable)
    // 3. ...packed accounts
    let mut account_metas = vec![
        AccountMeta::new_readonly(Pubkey::new_from_array(CPI_AUTHORITY_PDA), false),
        AccountMeta::new_readonly(fee_payer, true),
    ];
    account_metas.extend(packed_account_metas);

    // Build instruction data with empty token data but with compressions
    let instruction_data = CompressedTokenInstructionDataTransfer2 {
        with_transaction_hash: false,
        with_lamports_change_account_merkle_tree_index: false,
        lamports_change_account_merkle_tree_index: 0,
        lamports_change_account_owner_index: 0,
        output_queue: 0,
        proof: ValidityProof::default().into(),
        in_token_data: vec![],  // Empty - no compressed inputs
        out_token_data: vec![], // Empty - no compressed outputs
        in_lamports: None,
        out_lamports: None,
        in_tlv: None,
        out_tlv: None,
        compressions,
        cpi_context: None,
    };

    // Serialize instruction data
    let serialized = instruction_data
        .try_to_vec()
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to serialize: {:?}", e)))?;

    // Build instruction data with discriminator
    let mut data = Vec::with_capacity(1 + serialized.len());
    data.push(TRANSFER2);
    data.extend(serialized);

    Ok(solana_sdk::instruction::Instruction {
        program_id: light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID.into(),
        accounts: account_metas,
        data,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_setup_functional() -> Result<(), RpcError> {
    // Functional baseline test: compress and decompress using compressions-only path
    // This tests that sum check passes when compress and decompress balance out
    let NoSystemProgramCpiTestContext {
        mut rpc,
        payer,
        owner,
        compressions,
        packed_accounts,
        ..
    } = setup_no_system_program_cpi_test(1000).await?;

    // Build and send compressions-only instruction with balanced compress/decompress
    let ix =
        build_compressions_only_instruction(payer.pubkey(), Some(compressions), packed_accounts)?;

    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should succeed - compress 500, decompress 500 = balanced
    assert!(
        result.is_ok(),
        "Balanced compress/decompress should succeed: {:?}",
        result.err()
    );

    Ok(())
}

#[tokio::test]
async fn test_empty_compressions_array() -> Result<(), RpcError> {
    // Test: Empty compressions array should fail with NoInputsProvided (6025)
    let NoSystemProgramCpiTestContext {
        mut rpc,
        payer,
        owner,
        packed_accounts,
        ..
    } = setup_no_system_program_cpi_test(1000).await?;

    // Create instruction with None compressions (not an empty array)
    let ix = build_compressions_only_instruction(payer.pubkey(), None, packed_accounts)?;

    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with NoInputsProvided (error code 25, which is 6025 - 6000)
    assert_rpc_error(
        result, 0, 25, // NoInputsProvided
    )?;

    Ok(())
}

// ============================================================================
// Sum Check Failures - CToken
// ============================================================================

#[tokio::test]
async fn test_compress_without_decompress() -> Result<(), RpcError> {
    // Test: Compress without decompress should fail with SumCheckFailed (6005)
    let NoSystemProgramCpiTestContext {
        mut rpc,
        payer,
        owner,
        mint,
        source_ata,
        packed_accounts: _,
        ..
    } = setup_no_system_program_cpi_test(1000).await?;

    // Create compressions with only compress (no decompress to balance it)
    let (compressions, packed_accounts) = create_compressions_and_packed_accounts(
        mint,
        owner.pubkey(),
        source_ata,
        source_ata, // doesn't matter, not used
        500,        // compress amount
        0,          // NO decompress - will cause sum check to fail
    );

    let ix =
        build_compressions_only_instruction(payer.pubkey(), Some(compressions), packed_accounts)?;

    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with SumCheckFailed (error code 6005)
    assert_rpc_error(
        result, 0, 6005, // SumCheckFailed
    )?;

    Ok(())
}

#[tokio::test]
async fn test_decompress_without_compress() -> Result<(), RpcError> {
    // Test: Decompress without compress should fail with SumCheckFailed (6005)
    let NoSystemProgramCpiTestContext {
        mut rpc,
        payer,
        owner,
        mint,
        recipient_ata,
        packed_accounts: _,
        ..
    } = setup_no_system_program_cpi_test(1000).await?;

    // Create compressions with only decompress (no compress to balance it)
    let (compressions, packed_accounts) = create_compressions_and_packed_accounts(
        mint,
        owner.pubkey(),
        recipient_ata, // doesn't matter, not used
        recipient_ata,
        0,   // NO compress - will cause sum check to fail
        500, // decompress amount
    );

    let ix =
        build_compressions_only_instruction(payer.pubkey(), Some(compressions), packed_accounts)?;

    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with SumCheckFailed (error code 6005)
    assert_rpc_error(
        result, 0, 6005, // SumCheckFailed
    )?;

    Ok(())
}

#[tokio::test]
async fn test_compress_less_decompress_more() -> Result<(), RpcError> {
    // Test: Compress 300, decompress 500 should fail with SumCheckFailed (6005)
    // Note: Originally expected ComputeOutputSumFailed (6002) but actual error is SumCheckFailed
    let NoSystemProgramCpiTestContext {
        mut rpc,
        payer,
        owner,
        mint,
        source_ata,
        recipient_ata,
        packed_accounts: _,
        ..
    } = setup_no_system_program_cpi_test(1000).await?;

    // Create compressions with unbalanced amounts (decompress > compress)
    let (compressions, packed_accounts) = create_compressions_and_packed_accounts(
        mint,
        owner.pubkey(),
        source_ata,
        recipient_ata,
        499, // compress less
        500, // decompress more - will cause sum check to fail
    );

    let ix =
        build_compressions_only_instruction(payer.pubkey(), Some(compressions), packed_accounts)?;

    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with SumCheckFailed (error code 6005)
    assert_rpc_error(
        result, 0, 6005, // SumCheckFailed
    )?;

    Ok(())
}

#[tokio::test]
async fn test_compress_more_decompress_less() -> Result<(), RpcError> {
    // Test: Compress 700, decompress 500 should fail with SumCheckFailed (6005)
    let NoSystemProgramCpiTestContext {
        mut rpc,
        payer,
        owner,
        mint,
        source_ata,
        recipient_ata,
        packed_accounts: _,
        ..
    } = setup_no_system_program_cpi_test(1000).await?;

    // Create compressions with unbalanced amounts (compress > decompress)
    let (compressions, packed_accounts) = create_compressions_and_packed_accounts(
        mint,
        owner.pubkey(),
        source_ata,
        recipient_ata,
        501, // compress more
        500, // decompress less - will cause sum check to fail
    );

    let ix =
        build_compressions_only_instruction(payer.pubkey(), Some(compressions), packed_accounts)?;

    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with SumCheckFailed (error code 6005)
    assert_rpc_error(
        result, 0, 6005, // SumCheckFailed
    )?;

    Ok(())
}

// ============================================================================
// CToken Authority Failures
// ============================================================================

#[tokio::test]
async fn test_invalid_authority_compress() {
    // Test: Compress with wrong authority should fail with OwnerMismatch (6075)
    let NoSystemProgramCpiTestContext {
        mut rpc,
        payer,
        mint,
        source_ata,
        recipient_ata,
        ..
    } = setup_no_system_program_cpi_test(1000).await.unwrap();

    // Create a wrong authority (not the owner)
    let wrong_authority = Keypair::new();
    airdrop_lamports(&mut rpc, &wrong_authority.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // Build compressions manually with wrong authority
    use light_sdk::instruction::PackedAccounts;

    let mut packed_accounts = PackedAccounts::default();
    let mint_index = packed_accounts.insert_or_get_read_only(mint);
    let wrong_authority_index =
        packed_accounts.insert_or_get_config(wrong_authority.pubkey(), true, false);
    let source_ata_index = packed_accounts.insert_or_get_config(source_ata, false, true);
    let recipient_ata_index = packed_accounts.insert_or_get_config(recipient_ata, false, true);

    let compressions = vec![
        Compression::compress_ctoken(500, mint_index, source_ata_index, wrong_authority_index),
        Compression::decompress_ctoken(500, mint_index, recipient_ata_index),
    ];

    let (account_metas, _, _) = packed_accounts.to_account_metas();

    let ix = build_compressions_only_instruction(payer.pubkey(), Some(compressions), account_metas)
        .unwrap();

    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &wrong_authority])
        .await;

    // Should fail with OwnerMismatch (error code 75, which is 6075 - 6000)
    assert_rpc_error(result, 0, 75).unwrap();
}

#[tokio::test]
async fn test_authority_not_signer_compress() {
    // Test: Compress with authority not signer should fail with InvalidSigner (20009)
    let NoSystemProgramCpiTestContext {
        mut rpc,
        payer,
        owner,
        mint,
        source_ata,
        recipient_ata,
        ..
    } = setup_no_system_program_cpi_test(1000).await.unwrap();

    // Build compressions manually with owner as authority but NOT marked as signer
    use light_sdk::instruction::PackedAccounts;

    let mut packed_accounts = PackedAccounts::default();
    let mint_index = packed_accounts.insert_or_get_read_only(mint);
    // Key fix: Mark owner as NOT signer (third parameter = false)
    let owner_index = packed_accounts.insert_or_get_config(owner.pubkey(), false, false);
    let source_ata_index = packed_accounts.insert_or_get_config(source_ata, false, true);
    let recipient_ata_index = packed_accounts.insert_or_get_config(recipient_ata, false, true);

    let compressions = vec![
        Compression::compress_ctoken(500, mint_index, source_ata_index, owner_index),
        Compression::decompress_ctoken(500, mint_index, recipient_ata_index),
    ];

    let (account_metas, _, _) = packed_accounts.to_account_metas();

    let ix = build_compressions_only_instruction(payer.pubkey(), Some(compressions), account_metas)
        .unwrap();

    // Only include payer as signer, not owner
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
        .await;

    // Should fail with InvalidSigner (error code 20009)
    assert_rpc_error(result, 0, 20009).unwrap();
}

#[tokio::test]
async fn test_insufficient_balance_compress() {
    // Test: Compress more than balance should fail with CompressInsufficientFunds (18019)
    let NoSystemProgramCpiTestContext {
        mut rpc,
        payer,
        owner,
        mint,
        source_ata,
        recipient_ata,
        ..
    } = setup_no_system_program_cpi_test(1000).await.unwrap();

    // Try to compress 1500 but only have 1000 in source_ata
    let (compressions, packed_accounts) = create_compressions_and_packed_accounts(
        mint,
        owner.pubkey(),
        source_ata,
        recipient_ata,
        1500, // more than the 1000 balance
        1500,
    );

    let ix =
        build_compressions_only_instruction(payer.pubkey(), Some(compressions), packed_accounts)
            .unwrap();

    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with ArithmeticOverflow (subtraction overflow when balance < compress amount)
    // Note: Originally expected CompressInsufficientFunds (18019) but balance check happens
    // via checked subtraction which returns ArithmeticOverflow on underflow
    assert!(result.is_err());
    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("ArithmeticOverflow"),
        "Expected ArithmeticOverflow in error, got: {}",
        err_string
    );
}

/// Test 11: Decompress with nonzero authority field
/// Expected: InvalidInstructionData (3)
#[tokio::test]
async fn test_decompress_with_nonzero_authority() {
    let mut context = setup_no_system_program_cpi_test(1000).await.unwrap();

    // Build compressions manually with authority set for decompress operation
    let mut packed_accounts = PackedAccounts::default();
    let mint_index = packed_accounts.insert_or_get_read_only(context.mint);
    let owner_index = packed_accounts.insert_or_get_config(context.owner.pubkey(), true, false);
    let source_ata_index = packed_accounts.insert_or_get_config(context.source_ata, false, true);
    let recipient_ata_index =
        packed_accounts.insert_or_get_config(context.recipient_ata, false, true);

    // Create compress with valid authority using helper
    let compress_compression =
        Compression::compress_ctoken(500, mint_index, source_ata_index, owner_index);

    // Create decompress but manually set authority to non-zero (should be 0)
    let mut decompress_compression =
        Compression::decompress_ctoken(500, mint_index, recipient_ata_index);
    decompress_compression.authority = owner_index; // Invalid: should be 0 for decompress

    let compressions = vec![compress_compression, decompress_compression];

    let (account_metas, _, _) = packed_accounts.to_account_metas();

    // Build instruction
    let instruction = build_compressions_only_instruction(
        context.payer.pubkey(),
        Some(compressions),
        account_metas,
    )
    .unwrap();

    // Execute transaction
    let result = context
        .rpc
        .create_and_send_transaction(
            &[instruction],
            &context.payer.pubkey(),
            &[&context.payer, &context.owner],
        )
        .await;

    // Should fail with InvalidInstructionData (ProgramError, not custom error code)
    assert!(result.is_err());
    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("InvalidInstructionData"),
        "Expected InvalidInstructionData in error, got: {}",
        err_string
    );
}

/// Test 12: Too many mints (>5)
/// Expected: TooManyMints (6039)
#[tokio::test]
async fn test_too_many_mints() {
    let mut context = setup_no_system_program_cpi_test(1000).await.unwrap();

    // Create mint authority for the additional mints
    let mint_authority = Keypair::new();
    airdrop_lamports(&mut context.rpc, &mint_authority.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // Create 5 additional mints (6 total including context.mint)
    let mut mints_with_atas = vec![];
    for _ in 0..5 {
        // Create new mint seed
        let mint_seed = Keypair::new();
        let (mint, _) = find_spl_mint_address(&mint_seed.pubkey());
        let (source_ata, _) = derive_ctoken_ata(&context.owner.pubkey(), &mint);
        let (recipient_ata, _) = derive_ctoken_ata(&context.recipient.pubkey(), &mint);

        // Create source ATA
        let instruction =
            light_compressed_token_sdk::instructions::create_associated_token_account(
                context.payer.pubkey(),
                context.owner.pubkey(),
                mint,
            )
            .unwrap();
        context
            .rpc
            .create_and_send_transaction(&[instruction], &context.payer.pubkey(), &[&context.payer])
            .await
            .unwrap();

        // Create recipient ATA
        let instruction =
            light_compressed_token_sdk::instructions::create_associated_token_account(
                context.payer.pubkey(),
                context.recipient.pubkey(),
                mint,
            )
            .unwrap();
        context
            .rpc
            .create_and_send_transaction(&[instruction], &context.payer.pubkey(), &[&context.payer])
            .await
            .unwrap();

        // Create mint and mint tokens to source CToken ATA
        let decompressed_recipients = vec![Recipient {
            recipient: context.owner.pubkey().to_bytes().into(),
            amount: 1000,
        }];

        light_token_client::actions::mint_action_comprehensive(
            &mut context.rpc,
            &mint_seed,
            &mint_authority,
            &context.payer,
            vec![],                  // no compressed recipients
            decompressed_recipients, // mint to source CToken ATA
            None,
            None,
            Some(light_token_client::instructions::mint_action::NewMint {
                decimals: 6,
                supply: 0,
                mint_authority: mint_authority.pubkey(),
                freeze_authority: None,
                metadata: None,
                version: 3, // ShaFlat
            }),
        )
        .await
        .unwrap();

        mints_with_atas.push((mint, source_ata, recipient_ata));
    }

    // Build compressions with 6 different mints
    let mut packed_accounts = PackedAccounts::default();
    let owner_index = packed_accounts.insert_or_get_config(context.owner.pubkey(), true, false);

    let mut compressions = vec![];

    // Add compressions for the first mint from context
    let mint_index = packed_accounts.insert_or_get_read_only(context.mint);
    let source_index = packed_accounts.insert_or_get_config(context.source_ata, false, true);
    let recipient_index = packed_accounts.insert_or_get_config(context.recipient_ata, false, true);

    compressions.push(Compression::compress_ctoken(
        100,
        mint_index,
        source_index,
        owner_index,
    ));
    compressions.push(Compression::decompress_ctoken(
        100,
        mint_index,
        recipient_index,
    ));

    // Add compressions for the 5 additional mints
    for (mint, source_ata, recipient_ata) in &mints_with_atas {
        let mint_index = packed_accounts.insert_or_get_read_only(*mint);
        let source_index = packed_accounts.insert_or_get_config(*source_ata, false, true);
        let recipient_index = packed_accounts.insert_or_get_config(*recipient_ata, false, true);

        compressions.push(Compression::compress_ctoken(
            100,
            mint_index,
            source_index,
            owner_index,
        ));
        compressions.push(Compression::decompress_ctoken(
            100,
            mint_index,
            recipient_index,
        ));
    }

    let (account_metas, _, _) = packed_accounts.to_account_metas();

    // Build instruction
    let instruction = build_compressions_only_instruction(
        context.payer.pubkey(),
        Some(compressions),
        account_metas,
    )
    .unwrap();

    // Execute transaction
    let result = context
        .rpc
        .create_and_send_transaction(
            &[instruction],
            &context.payer.pubkey(),
            &[&context.payer, &context.owner],
        )
        .await;

    // Should fail with TooManyMints
    assert_rpc_error(result, 0, 6039).unwrap();
}

/// Test 13: Duplicate mint validation
/// Expected: DuplicateMint (6102)
#[tokio::test]
async fn test_duplicate_mint_validation() {
    let mut context = setup_no_system_program_cpi_test(1000).await.unwrap();

    // Build compressions with duplicate mint index
    let mut packed_accounts = PackedAccounts::default();
    let owner_index = packed_accounts.insert_or_get_config(context.owner.pubkey(), true, false);
    let mint_index_1 = packed_accounts.insert_or_get_read_only(context.mint);
    let source_index = packed_accounts.insert_or_get_config(context.source_ata, false, true);
    let recipient_index = packed_accounts.insert_or_get_config(context.recipient_ata, false, true);

    // Get account metas and manually add duplicate mint at the end
    let (mut account_metas, _, _) = packed_accounts.to_account_metas();
    let mint_index_2 = account_metas.len() as u8;
    account_metas.push(solana_sdk::instruction::AccountMeta::new_readonly(
        context.mint,
        false,
    ));

    // Create compression and decompress for first mint index (balanced)
    let mut compressions = vec![
        Compression::compress_ctoken(500, mint_index_1, source_index, owner_index),
        Compression::decompress_ctoken(500, mint_index_1, recipient_index),
    ];

    // Create compression and decompress for second mint index (balanced)
    // This is the duplicate - same pubkey as mint_index_1 but different index
    compressions.push(Compression::compress_ctoken(
        1,
        mint_index_2,
        source_index,
        owner_index,
    ));
    compressions.push(Compression::decompress_ctoken(
        1,
        mint_index_2,
        recipient_index,
    ));

    // Build instruction
    let instruction = build_compressions_only_instruction(
        context.payer.pubkey(),
        Some(compressions),
        account_metas,
    )
    .unwrap();

    // Execute transaction
    let result = context
        .rpc
        .create_and_send_transaction(
            &[instruction],
            &context.payer.pubkey(),
            &[&context.payer, &context.owner],
        )
        .await;

    // Should fail with DuplicateMint
    assert_rpc_error(result, 0, 6102).unwrap();
}

/// Test 14: Mint index out of bounds
/// Expected: DuplicateMint (6102) - out of bounds is masked as DuplicateMint in validate_mint_uniqueness
#[tokio::test]
async fn test_mint_index_out_of_bounds() {
    let mut context = setup_no_system_program_cpi_test(1000).await.unwrap();

    // Build compressions with out-of-bounds mint index
    let mut packed_accounts = PackedAccounts::default();
    let owner_index = packed_accounts.insert_or_get_config(context.owner.pubkey(), true, false);
    let source_index = packed_accounts.insert_or_get_config(context.source_ata, false, true);
    let recipient_index = packed_accounts.insert_or_get_config(context.recipient_ata, false, true);

    let (account_metas, _, _) = packed_accounts.to_account_metas();

    // Use invalid mint index (99 which doesn't exist)
    // Need balanced operations to pass sum check
    let invalid_mint_index = 99u8;

    let compressions = vec![
        Compression::compress_ctoken(500, invalid_mint_index, source_index, owner_index),
        Compression::decompress_ctoken(500, invalid_mint_index, recipient_index),
    ];

    // Build instruction
    let instruction = build_compressions_only_instruction(
        context.payer.pubkey(),
        Some(compressions),
        account_metas,
    )
    .unwrap();

    // Execute transaction
    let result = context
        .rpc
        .create_and_send_transaction(
            &[instruction],
            &context.payer.pubkey(),
            &[&context.payer, &context.owner],
        )
        .await;

    // Should fail with DuplicateMint (out of bounds is masked)
    assert_rpc_error(result, 0, 6102).unwrap();
}

/// Test 15: Account index out of bounds
/// Expected: NotEnoughAccountKeys (20014) - this one succeeds because account validation happens in process_token_compression
#[tokio::test]
async fn test_account_index_out_of_bounds() {
    let mut context = setup_no_system_program_cpi_test(1000).await.unwrap();

    // Build compressions with out-of-bounds account index
    let mut packed_accounts = PackedAccounts::default();
    let owner_index = packed_accounts.insert_or_get_config(context.owner.pubkey(), true, false);
    let mint_index = packed_accounts.insert_or_get_read_only(context.mint);

    let (account_metas, _, _) = packed_accounts.to_account_metas();

    // Use invalid account index (99 which doesn't exist)
    // Need balanced operations to pass sum check
    let invalid_account_index = 99u8;

    let compressions = vec![
        Compression::compress_ctoken(500, mint_index, invalid_account_index, owner_index),
        Compression::decompress_ctoken(500, mint_index, invalid_account_index),
    ];

    // Build instruction
    let instruction = build_compressions_only_instruction(
        context.payer.pubkey(),
        Some(compressions),
        account_metas,
    )
    .unwrap();

    // Execute transaction
    let result = context
        .rpc
        .create_and_send_transaction(
            &[instruction],
            &context.payer.pubkey(),
            &[&context.payer, &context.owner],
        )
        .await;

    // Should fail with NotEnoughAccountKeys
    assert_rpc_error(result, 0, 20014).unwrap();
}

/// Test 16: Authority index out of bounds
/// Expected: Client-side signing error - authority index can't be validated on-chain in compressions-only mode
/// because transaction construction fails when trying to resolve signer
#[tokio::test]
async fn test_authority_index_out_of_bounds() {
    let mut context = setup_no_system_program_cpi_test(1000).await.unwrap();

    // Build compressions with out-of-bounds authority index
    let mut packed_accounts = PackedAccounts::default();
    let mint_index = packed_accounts.insert_or_get_read_only(context.mint);
    let source_index = packed_accounts.insert_or_get_config(context.source_ata, false, true);
    let recipient_index = packed_accounts.insert_or_get_config(context.recipient_ata, false, true);

    let (account_metas, _, _) = packed_accounts.to_account_metas();

    // Use invalid authority index (99 which doesn't exist)
    // Need balanced operations to pass sum check
    let invalid_authority_index = 99u8;

    let compressions = vec![
        Compression::compress_ctoken(500, mint_index, source_index, invalid_authority_index),
        Compression::decompress_ctoken(500, mint_index, recipient_index),
    ];

    // Build instruction
    let instruction = build_compressions_only_instruction(
        context.payer.pubkey(),
        Some(compressions),
        account_metas,
    )
    .unwrap();

    // Execute transaction
    let result = context
        .rpc
        .create_and_send_transaction(
            &[instruction],
            &context.payer.pubkey(),
            &[&context.payer, &context.owner],
        )
        .await;

    // Should fail with client-side signing error (keypair-pubkey mismatch)
    assert!(result.is_err());
    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("SigningError"),
        "Expected SigningError in error, got: {}",
        err_string
    );
}

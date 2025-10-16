#![allow(clippy::result_large_err)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::to_string_in_format_args)]

// ============================================================================
// COMPRESS TESTS (Solana account → compressed)
// ============================================================================
//
// Sum Check Failures:
// 1. amount more than output (should fail with output sum check)
// 2. amount less than output (should fail with input sum check)
//
// CToken Compression Authority Validation:
// 3. ctoken compression
//  3.1 invalid authority has signed
//  3.2 authority is valid but not signer
//  3.3 insufficient balance in ctoken account → CompressInsufficientFunds (18019)
//  3.4 mint mismatch (token account mint != compression mint)
//
//
// Output Out of Bounds:
// 5.1. authority out of bounds
// 5.2. mint out of bounds
// 5.3. recipient out of bounds
//
// has_delegate Flag Mismatch:
// 6.1. Output: has_delegate=true but delegate=0
// 6.2. Output: has_delegate=false but delegate!=0
//
// ============================================================================
// TEST SETUP REQUIREMENTS
// ============================================================================
//
// Test setup for Compress ctoken:
// 1. create and mint to one ctoken compressed account
//

use light_compressed_token_sdk::{
    account2::CTokenAccount2,
    instructions::{
        create_associated_token_account::create_compressible_associated_token_account,
        derive_ctoken_ata, find_spl_mint_address,
        transfer2::{
            account_metas::Transfer2AccountsMetaConfig, create_transfer2_instruction,
            Transfer2Config, Transfer2Inputs,
        },
        CreateCompressibleAssociatedTokenAccountInputs,
    },
    ValidityProof,
};
use light_ctoken_types::{instructions::mint_action::Recipient, state::TokenDataVersion};
use light_program_test::{LightProgramTest, ProgramTestConfig, Rpc};
use light_sdk::instruction::PackedAccounts;
use light_test_utils::RpcError;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
// ============================================================================
// Test Setup
// ============================================================================

/// Test context for compression failing tests
struct CompressionTestContext {
    pub rpc: LightProgramTest,
    pub payer: Keypair,

    pub owner: Keypair,
    pub compression_inputs: Transfer2Inputs,
    pub system_accounts_offset: usize, // Offset to add to packed account indices to get instruction account indices
}

/// Set up test environment with compressed mint and one CToken account with tokens
async fn setup_compression_test(token_amount: u64) -> Result<CompressionTestContext, RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();

    // Create owner and airdrop lamports
    let owner = Keypair::new();
    rpc.airdrop_lamports(&owner.pubkey(), 1_000_000_000).await?;

    // Create mint authority
    let mint_authority = Keypair::new();
    rpc.airdrop_lamports(&mint_authority.pubkey(), 1_000_000_000)
        .await?;

    // Create compressed mint seed
    let mint_seed = Keypair::new();

    // Derive mint and ATA addresses
    let (mint, _) = find_spl_mint_address(&mint_seed.pubkey());
    let (ctoken_ata, _) = derive_ctoken_ata(&owner.pubkey(), &mint);

    // Create compressible CToken ATA for owner
    let create_ata_instruction = create_compressible_associated_token_account(
        CreateCompressibleAssociatedTokenAccountInputs {
            payer: payer.pubkey(),
            owner: owner.pubkey(),
            mint,
            rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
            pre_pay_num_epochs: 2,
            lamports_per_write: Some(1000),
            compressible_config: rpc
                .test_accounts
                .funding_pool_config
                .compressible_config_pda,
            token_account_version: TokenDataVersion::ShaFlat,
        },
    )
    .map_err(|e| RpcError::AssertRpcError(format!("Failed to create ATA: {:?}", e)))?;

    rpc.create_and_send_transaction(&[create_ata_instruction], &payer.pubkey(), &[&payer])
        .await?;

    // Use mint_action_comprehensive to create mint AND mint to decompressed CToken ATA
    let decompressed_recipients = vec![Recipient {
        recipient: owner.pubkey().to_bytes().into(),
        amount: token_amount,
    }];

    light_token_client::actions::mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &mint_authority,
        &payer,
        vec![],                  // no compressed recipients
        decompressed_recipients, // mint to decompressed CToken ATA
        None,                    // no mint authority update
        None,                    // no freeze authority update
        Some(light_token_client::instructions::mint_action::NewMint {
            decimals: 6,
            supply: 0,
            mint_authority: mint_authority.pubkey(),
            freeze_authority: None,
            metadata: None,
            version: 3, // ShaFlat for compressible accounts
        }),
    )
    .await?;

    // Get output queue for compression
    let output_queue = rpc
        .get_random_state_tree_info()
        .unwrap()
        .get_output_pubkey()
        .unwrap();

    // Build compression Transfer2Inputs
    let compression_inputs = create_compression_inputs(
        ctoken_ata,
        mint,
        owner.pubkey(),
        owner.pubkey(), // compress to owner
        token_amount,
        payer.pubkey(),
        output_queue,
        0, // output_merkle_tree_index (output queue is at index 0)
    )?;

    // Calculate system accounts offset by creating a test instruction
    // and finding where the first packed account appears
    let test_ix = create_transfer2_instruction(compression_inputs.clone())
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Find the first packed account (merkle tree at packed index 0)
    let first_packed_account = compression_inputs
        .meta_config
        .packed_accounts
        .as_ref()
        .unwrap()[0]
        .pubkey;
    let system_accounts_offset = test_ix
        .accounts
        .iter()
        .position(|acc| acc.pubkey == first_packed_account)
        .expect("First packed account should be in instruction");

    Ok(CompressionTestContext {
        rpc,
        payer,

        owner,
        compression_inputs,
        system_accounts_offset,
    })
}

// ============================================================================
// Instruction Builder Helpers
// ============================================================================

/// Build Transfer2Inputs for compression (CToken ATA -> compressed)
/// This uses the low-level SDK abstractions for maximum control in failing tests
/// Returns Transfer2Inputs so tests can modify it before creating the instruction
fn create_compression_inputs(
    ctoken_ata: Pubkey,
    mint: Pubkey,
    authority: Pubkey,
    recipient: Pubkey,
    compress_amount: u64,
    fee_payer: Pubkey,
    output_queue: Pubkey,
    output_merkle_tree_index: u8,
) -> Result<Transfer2Inputs, RpcError> {
    // Use PackedAccounts to manage account packing
    let mut packed_accounts = PackedAccounts::default();

    // For compression (0 inputs, 1 output), add the output queue
    packed_accounts.insert_or_get(output_queue);

    // Add mint, authority (owner of CToken ATA), recipient
    let mint_index = packed_accounts.insert_or_get_read_only(mint);
    let authority_index = packed_accounts.insert_or_get_config(authority, true, false); // is_signer, not writable
    let recipient_index = packed_accounts.insert_or_get_read_only(recipient);

    // Add CToken ATA account
    let ctoken_ata_index = packed_accounts.insert_or_get_config(ctoken_ata, false, true); // not signer, is writable

    // Create CTokenAccount2 for compression (0 inputs, 1 output)
    // Use new_empty since we have no compressed input accounts
    let mut compression_account = CTokenAccount2::new_empty(recipient_index, mint_index);

    // Compress tokens from CToken ATA
    compression_account
        .compress_ctoken(compress_amount, ctoken_ata_index, authority_index)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to compress: {:?}", e)))?;

    // Get account metas from PackedAccounts
    let (account_metas, _, _) = packed_accounts.to_account_metas();

    // Build and return Transfer2Inputs
    // token_accounts contains single account with compression output
    Ok(Transfer2Inputs {
        token_accounts: vec![compression_account],
        validity_proof: ValidityProof::default(),
        transfer_config: Transfer2Config::default().filter_zero_amount_outputs(),
        meta_config: Transfer2AccountsMetaConfig::new(fee_payer, account_metas),
        in_lamports: None,
        out_lamports: None,
        output_queue: output_merkle_tree_index,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_ctoken_compression_functional() -> Result<(), RpcError> {
    // Baseline test: valid compression should succeed
    let CompressionTestContext {
        mut rpc,
        payer,
        owner,
        compression_inputs,
        system_accounts_offset: _,
    } = setup_compression_test(1000).await?;

    // Create instruction from Transfer2Inputs
    let ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Send transaction with owner as signer (owner of CToken ATA)
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should succeed
    assert!(
        result.is_ok(),
        "Valid compression should succeed: {:?}",
        result.err()
    );

    Ok(())
}

#[tokio::test]
async fn test_compression_amount_less_than_output() -> Result<(), RpcError> {
    // Test: Compression amount less than output (input sum check should fail)
    // Compress 1000 tokens from CToken ATA but output shows 1001 tokens
    let CompressionTestContext {
        mut rpc,
        payer,

        owner,
        mut compression_inputs,
        system_accounts_offset: _,
    } = setup_compression_test(1000).await?;

    // Increase output amount by 1 (compression amount is 1000, but output is 1001)
    // This breaks the sum check: input sum (from compressions) < output sum
    compression_inputs.token_accounts[0].output.amount += 1;

    // Create instruction from modified Transfer2Inputs
    let ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Send transaction with owner as signer
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with ComputeInputSumFailed (6002)
    light_program_test::utils::assert::assert_rpc_error(result, 0, 6002).unwrap();

    Ok(())
}

#[tokio::test]
async fn test_compression_amount_more_than_output() -> Result<(), RpcError> {
    // Test: Compression amount more than output (output sum check should fail)
    // Compress 1000 tokens from CToken ATA but output shows 999 tokens
    let CompressionTestContext {
        mut rpc,
        payer,

        owner,
        mut compression_inputs,
        system_accounts_offset: _,
    } = setup_compression_test(1000).await?;

    // Decrease output amount by 1 (compression amount is 1000, but output is 999)
    // This breaks the sum check: input sum (from compressions) > output sum
    compression_inputs.token_accounts[0].output.amount -= 1;

    // Create instruction from modified Transfer2Inputs
    let ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Send transaction with owner as signer
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with SumCheckFailed (6005)
    light_program_test::utils::assert::assert_rpc_error(result, 0, 6005).unwrap();

    Ok(())
}

#[tokio::test]
async fn test_compression_invalid_authority_signed() -> Result<(), RpcError> {
    // Test: Invalid authority has signed (not the CToken ATA owner)
    let CompressionTestContext {
        mut rpc,
        payer,

        owner: _,
        compression_inputs,
        system_accounts_offset,
    } = setup_compression_test(1000).await?;

    // Create an invalid authority keypair
    let invalid_authority = Keypair::new();
    rpc.airdrop_lamports(&invalid_authority.pubkey(), 1_000_000_000)
        .await?;

    // Get authority packed index from compression inputs
    let authority_packed_index = compression_inputs.token_accounts[0]
        .compression
        .as_ref()
        .unwrap()
        .authority;

    // Create instruction from Transfer2Inputs
    let mut ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Replace authority account with invalid authority (using system_accounts_offset)
    ix.accounts[system_accounts_offset + authority_packed_index as usize].pubkey =
        invalid_authority.pubkey();

    // Send transaction with invalid authority as signer
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &invalid_authority])
        .await;

    // Should fail with OwnerMismatch (custom program error 0x4b = 75) - Authority doesn't match account owner or delegate
    assert!(
        result
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("custom program error: 0x4b"),
        "Expected custom program error 0x4b, got: {}",
        result.unwrap_err().to_string()
    );

    Ok(())
}

#[tokio::test]
async fn test_compression_authority_not_signer() -> Result<(), RpcError> {
    // Test: Authority is valid but not signer
    let CompressionTestContext {
        mut rpc,
        payer,
        owner: _,
        compression_inputs,
        system_accounts_offset,
    } = setup_compression_test(1000).await?;

    // Get authority packed index from compression inputs
    let authority_packed_index = compression_inputs.token_accounts[0]
        .compression
        .as_ref()
        .unwrap()
        .authority;

    // Create instruction from Transfer2Inputs
    let mut ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Set authority as non-signer (using system_accounts_offset)
    ix.accounts[system_accounts_offset + authority_packed_index as usize].is_signer = false;

    // Send transaction without authority as signer (only payer)
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
        .await;

    // Should fail with InvalidSigner (20009) - Required signer is not signing
    light_program_test::utils::assert::assert_rpc_error(result, 0, 20009).unwrap();

    Ok(())
}

#[tokio::test]
async fn test_compression_invalid_mint() -> Result<(), RpcError> {
    // Test: Invalid mint in output - mint index points to wrong account
    // This will cause sum check to fail because mints don't match
    let CompressionTestContext {
        mut rpc,
        payer,
        owner,
        mut compression_inputs,
        system_accounts_offset: _,
    } = setup_compression_test(1000).await?;

    // Get recipient packed index and use it as fake mint index
    // This keeps sum check balanced (both compression and output use same mint index)
    // but the actual mint pubkey will be wrong
    let recipient_packed_index = compression_inputs.token_accounts[0].output.owner;

    // Change mint index in both compression and output to point to recipient account
    compression_inputs.token_accounts[0]
        .compression
        .as_mut()
        .unwrap()
        .mint = recipient_packed_index;
    compression_inputs.token_accounts[0].output.mint = recipient_packed_index;

    // Create instruction from modified Transfer2Inputs
    let ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Send transaction
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with InvalidAccountData - mint mismatch detected during CToken account validation
    assert!(
        result
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("invalid account data for instruction"),
        "Expected InvalidAccountData error, got: {}",
        result.unwrap_err().to_string()
    );

    Ok(())
}

#[tokio::test]
async fn test_compression_authority_out_of_bounds() -> Result<(), RpcError> {
    // Test: Authority index out of bounds
    let CompressionTestContext {
        mut rpc,
        payer,
        owner,
        mut compression_inputs,
        system_accounts_offset: _,
    } = setup_compression_test(1000).await?;

    // Get the number of packed accounts
    let num_packed_accounts = compression_inputs
        .meta_config
        .packed_accounts
        .as_ref()
        .unwrap()
        .len();

    // Set authority index to out of bounds value
    compression_inputs.token_accounts[0]
        .compression
        .as_mut()
        .unwrap()
        .authority = num_packed_accounts as u8;

    // Create instruction from modified Transfer2Inputs
    let ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Send transaction
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with NotEnoughAccountKeys (20014)
    light_program_test::utils::assert::assert_rpc_error(result, 0, 20014).unwrap();

    Ok(())
}

#[tokio::test]
async fn test_compression_mint_out_of_bounds() -> Result<(), RpcError> {
    // Test: Mint index out of bounds in output
    let CompressionTestContext {
        mut rpc,
        payer,
        owner,
        mut compression_inputs,
        system_accounts_offset: _,
    } = setup_compression_test(1000).await?;

    // Get the number of packed accounts
    let num_packed_accounts = compression_inputs
        .meta_config
        .packed_accounts
        .as_ref()
        .unwrap()
        .len();

    // Set mint index to out of bounds value in output
    compression_inputs.token_accounts[0].output.mint = num_packed_accounts as u8;

    // Create instruction from modified Transfer2Inputs
    let ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Send transaction
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with NotEnoughAccountKeys (20014)
    light_program_test::utils::assert::assert_rpc_error(result, 0, 20014).unwrap();

    Ok(())
}

#[tokio::test]
async fn test_compression_recipient_out_of_bounds() -> Result<(), RpcError> {
    // Test: Recipient (owner) index out of bounds in output
    let CompressionTestContext {
        mut rpc,
        payer,
        owner,
        mut compression_inputs,
        system_accounts_offset: _,
    } = setup_compression_test(1000).await?;

    // Get the number of packed accounts
    let num_packed_accounts = compression_inputs
        .meta_config
        .packed_accounts
        .as_ref()
        .unwrap()
        .len();

    // Set recipient (owner) index to out of bounds value in output
    compression_inputs.token_accounts[0].output.owner = num_packed_accounts as u8;

    // Create instruction from modified Transfer2Inputs
    let ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Send transaction
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with NotEnoughAccountKeys (20014)
    light_program_test::utils::assert::assert_rpc_error(result, 0, 20014).unwrap();

    Ok(())
}

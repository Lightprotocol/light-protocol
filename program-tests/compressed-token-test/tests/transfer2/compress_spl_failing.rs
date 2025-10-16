#![allow(clippy::result_large_err)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::to_string_in_format_args)]

// ============================================================================
// COMPRESS SPL TESTS (SPL token account → compressed)
// ============================================================================
//
// Sum Check Failures:
// 1. amount more than output (should fail with output sum check)
// 2. amount less than output (should fail with input sum check)
//
// SPL Token Compression Authority Validation:
// 3. spl token compression
//  3.1 invalid authority has signed
//  3.2 authority is valid but not signer
//
// SPL Token Compression Pool Validation:
// 4. spl token compression
//  4.1 invalid pool account (invalid derivation seed, valid pool index, valid bump)
//  4.2 invalid pool account (valid derivation seed, valid pool index, invalid bump)
//  4.3 invalid pool account (valid derivation seed, invalid pool index, valid bump)
//  4.4 pool account out of bounds
//  4.5 pool index 6 (higher than max 5)
//
// Output Out of Bounds:
// 5.1. authority out of bounds
// 5.2. mint out of bounds
// 5.3. recipient out of bounds

use anchor_spl::token_2022::spl_token_2022;
use light_compressed_token_sdk::{
    account2::CTokenAccount2,
    instructions::{
        create_associated_token_account::derive_ctoken_ata,
        transfer2::{
            account_metas::Transfer2AccountsMetaConfig, create_transfer2_instruction,
            Transfer2Config, Transfer2Inputs,
        },
    },
    token_pool::find_token_pool_pda_with_index,
    ValidityProof,
};
use light_program_test::{utils::assert::assert_rpc_error, LightProgramTest, ProgramTestConfig};
use light_sdk::instruction::PackedAccounts;
use light_test_utils::{
    airdrop_lamports,
    spl::{create_mint_helper, create_token_2022_account, mint_spl_tokens},
    Rpc, RpcError,
};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use spl_pod::bytemuck::pod_from_bytes;
use spl_token_2022::{error::TokenError, pod::PodAccount};

// ============================================================================
// Test Setup
// ============================================================================

/// Test context for SPL compression failing tests
struct SplCompressionTestContext {
    pub rpc: LightProgramTest,
    pub payer: Keypair,
    pub mint: Pubkey,
    pub sender: Keypair,
    pub spl_token_account: Keypair,
    pub compression_inputs: Transfer2Inputs,
    pub system_accounts_offset: usize,
}

/// Set up test environment with SPL token account and CToken ATA
async fn setup_spl_compression_test(
    token_amount: u64,
) -> Result<SplCompressionTestContext, RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(true, None)).await?;
    let payer = rpc.get_payer().insecure_clone();

    // Create sender and airdrop lamports
    let sender = Keypair::new();
    airdrop_lamports(&mut rpc, &sender.pubkey(), 1_000_000_000).await?;

    // Create mint
    let mint = create_mint_helper(&mut rpc, &payer).await;

    // Create SPL token account and mint tokens
    let spl_token_account_keypair = Keypair::new();
    create_token_2022_account(&mut rpc, &mint, &spl_token_account_keypair, &sender, false).await?;
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &spl_token_account_keypair.pubkey(),
        &payer.pubkey(),
        &payer,
        token_amount,
        false,
    )
    .await?;

    // Create recipient and airdrop lamports
    let recipient = Keypair::new();
    airdrop_lamports(&mut rpc, &recipient.pubkey(), 1_000_000_000).await?;

    // Create compressed token ATA for recipient
    let instruction = light_compressed_token_sdk::instructions::create_associated_token_account(
        payer.pubkey(),
        recipient.pubkey(),
        mint,
    )
    .map_err(|e| RpcError::AssertRpcError(format!("Failed to create ATA instruction: {}", e)))?;

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await?;

    let ctoken_ata = derive_ctoken_ata(&recipient.pubkey(), &mint).0;

    // Get output queue for compression (for system_accounts_offset calculation only)
    let output_queue = rpc
        .get_random_state_tree_info()
        .unwrap()
        .get_output_pubkey()
        .unwrap();

    // Build SPL compression Transfer2Inputs
    let compression_inputs = create_spl_compression_inputs(
        spl_token_account_keypair.pubkey(),
        mint,
        sender.pubkey(),
        ctoken_ata, // Pass CToken ATA, not recipient pubkey
        token_amount,
        payer.pubkey(),
        output_queue,
        0, // output_merkle_tree_index (unused but kept for signature compatibility)
    )?;

    // Calculate system accounts offset
    let test_ix = create_transfer2_instruction(compression_inputs.clone())
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

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

    Ok(SplCompressionTestContext {
        rpc,
        payer,
        mint,
        sender,
        spl_token_account: spl_token_account_keypair,
        compression_inputs,
        system_accounts_offset,
    })
}

// ============================================================================
// Instruction Builder Helpers
// ============================================================================

/// Build Transfer2Inputs for SPL token compression
/// Follows the pattern from light-token-client transfer2.rs lines 257-290
fn create_spl_compression_inputs(
    spl_token_account: Pubkey,
    mint: Pubkey,
    authority: Pubkey,
    ctoken_ata: Pubkey,
    compress_amount: u64,
    fee_payer: Pubkey,
    output_queue: Pubkey,
    _output_merkle_tree_index: u8,
) -> Result<Transfer2Inputs, RpcError> {
    let mut packed_tree_accounts = PackedAccounts::default();

    // For compressions with no compressed inputs, we need the output queue
    let shared_output_queue = packed_tree_accounts.insert_or_get(output_queue);

    // Create empty token account with recipient/mint
    let to_index = packed_tree_accounts.insert_or_get(ctoken_ata);
    let mint_index = packed_tree_accounts.insert_or_get(mint);
    let mut token_account = CTokenAccount2::new_empty(to_index, mint_index);

    // Add source SPL account and authority
    let source_index = packed_tree_accounts.insert_or_get(spl_token_account);
    let authority_index = packed_tree_accounts.insert_or_get_config(authority, true, false);

    // Add SPL token program (spl_token::ID is the owner of SPL token accounts)
    let _token_program_index = packed_tree_accounts.insert_or_get_read_only(spl_token::ID);

    // Derive token pool PDA using SDK function
    let pool_index = 0u8;
    let (token_pool_pda, bump) = find_token_pool_pda_with_index(&mint, pool_index);
    let pool_account_index = packed_tree_accounts.insert_or_get(token_pool_pda);

    // Compress from SPL token account
    token_account
        .compress_spl(
            compress_amount,
            source_index,
            authority_index,
            pool_account_index,
            pool_index,
            bump,
        )
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to compress SPL: {:?}", e)))?;

    let packed_accounts = packed_tree_accounts.to_account_metas().0;

    Ok(Transfer2Inputs {
        token_accounts: vec![token_account],
        validity_proof: ValidityProof::default(),
        transfer_config: Transfer2Config::default().filter_zero_amount_outputs(),
        meta_config: Transfer2AccountsMetaConfig {
            fee_payer: Some(fee_payer),
            packed_accounts: Some(packed_accounts),
            ..Default::default()
        },
        in_lamports: None,
        out_lamports: None,
        output_queue: shared_output_queue,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_spl_compression_functional() -> Result<(), RpcError> {
    // Baseline test: valid SPL compression should succeed
    let SplCompressionTestContext {
        mut rpc,
        payer,
        mint: _,
        sender,
        spl_token_account,
        compression_inputs,
        system_accounts_offset: _,
    } = setup_spl_compression_test(1000).await?;

    // Create instruction from Transfer2Inputs
    let ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Send transaction with sender as signer (owner of SPL token account)
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &sender])
        .await;

    // Should succeed
    assert!(
        result.is_ok(),
        "Valid SPL compression should succeed: {:?}",
        result.err()
    );

    // Verify SPL token balance decreased
    let spl_account_data = rpc.get_account(spl_token_account.pubkey()).await?.unwrap();
    let spl_account = pod_from_bytes::<PodAccount>(&spl_account_data.data)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to parse SPL account: {}", e)))?;
    let final_spl_balance: u64 = spl_account.amount.into();
    assert_eq!(
        final_spl_balance, 0,
        "SPL balance should be 0 after compression"
    );

    Ok(())
}

// ============================================================================
// Sum Check Tests
// ============================================================================

#[tokio::test]
async fn test_spl_compression_amount_more_than_output() -> Result<(), RpcError> {
    // Test: compression amount (1000) > output amount (500) should fail
    let SplCompressionTestContext {
        mut rpc,
        payer,
        sender,
        mut compression_inputs,
        ..
    } = setup_spl_compression_test(1000).await?;

    // Modify the output amount to be less than the compression amount
    compression_inputs.token_accounts[0].output.amount = 500;

    let ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &sender])
        .await;

    // Should fail with output sum check error (6005)
    assert_rpc_error(result, 0, 6005)?;

    Ok(())
}

#[tokio::test]
async fn test_spl_compression_amount_less_than_output() -> Result<(), RpcError> {
    // Test: compression amount (500) < output amount (1000) should fail
    let SplCompressionTestContext {
        mut rpc,
        payer,
        sender,
        mut compression_inputs,
        ..
    } = setup_spl_compression_test(1000).await?;

    // Modify the compression amount to be less than output amount
    if let Some(compression) = &mut compression_inputs.token_accounts[0].compression {
        compression.amount = 500;
    }
    // Keep output amount at 1000 (from CTokenAccount2::new_empty + compress_spl)

    let ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &sender])
        .await;

    // Should fail with input sum check error (6002)
    assert_rpc_error(result, 0, 6002)?;

    Ok(())
}

// ============================================================================
// Authority Validation Tests
// ============================================================================

#[tokio::test]
async fn test_spl_compression_invalid_authority_signed() -> Result<(), RpcError> {
    // Test: Invalid authority (not the SPL token account owner) signs
    let SplCompressionTestContext {
        mut rpc,
        payer,
        sender: _,
        compression_inputs,
        system_accounts_offset,
        ..
    } = setup_spl_compression_test(1000).await?;

    // Create an invalid authority keypair
    let invalid_authority = Keypair::new();
    airdrop_lamports(&mut rpc, &invalid_authority.pubkey(), 1_000_000_000).await?;
    println!("compression_inputs {:?}", compression_inputs);
    // Replace the authority account in packed_accounts with invalid authority
    let mut ix = create_transfer2_instruction(compression_inputs.clone())
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Authority is at index 2 in packed_accounts (offset by system_accounts_offset)
    let authority_account_index = system_accounts_offset + 4;
    ix.accounts[authority_account_index].pubkey = invalid_authority.pubkey();

    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &invalid_authority])
        .await;

    // Should fail with TokenError::OwnerMismatch - SPL token program rejects because invalid_authority doesn't own the token account
    assert_rpc_error(result, 0, TokenError::OwnerMismatch as u32)?;

    Ok(())
}

#[tokio::test]
async fn test_spl_compression_authority_not_signer() -> Result<(), RpcError> {
    // Test: Valid authority but not marked as signer
    let SplCompressionTestContext {
        mut rpc,
        payer,
        sender: _,
        compression_inputs,
        system_accounts_offset,
        ..
    } = setup_spl_compression_test(1000).await?;
    println!("compression_inputs {:?}", compression_inputs);
    let mut ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Authority is at index 2 in packed_accounts (offset by system_accounts_offset)
    let authority_account_index = system_accounts_offset + 4;
    ix.accounts[authority_account_index].is_signer = false;

    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
        .await;

    // Should fail with MissingRequiredSignature - the SPL token program requires the authority to be a signer
    assert!(
        result
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("Cross-program invocation with unauthorized signer or writable account"),
        "Expected MissingRequiredSignature error, got: {}",
        result.unwrap_err().to_string()
    );

    Ok(())
}

// ============================================================================
// Pool Validation Tests
// ============================================================================

#[tokio::test]
async fn test_spl_compression_invalid_pool_derivation_seed() -> Result<(), RpcError> {
    // Test: Invalid pool PDA (wrong derivation seed, correct pool index and bump)
    let SplCompressionTestContext {
        mut rpc,
        payer,
        sender,
        compression_inputs,
        system_accounts_offset,
        ..
    } = setup_spl_compression_test(1000).await?;
    println!("compression_inputs {:?}", compression_inputs);
    let mut ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Token pool PDA is at index 6 in packed_accounts
    let pool_account_index = system_accounts_offset + 6;
    // Use a random pubkey as invalid pool PDA
    ix.accounts[pool_account_index].pubkey = Pubkey::new_unique();

    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &sender])
        .await;

    // Should fail with InvalidTokenPoolPda error (6023)
    assert_rpc_error(result, 0, 6023)?;

    Ok(())
}

#[tokio::test]
async fn test_spl_compression_invalid_pool_bump() -> Result<(), RpcError> {
    // Test: Invalid pool PDA bump
    let SplCompressionTestContext {
        mut rpc,
        payer,
        sender,
        mint,
        mut compression_inputs,
        ..
    } = setup_spl_compression_test(1000).await?;

    // Derive pool with correct seed but wrong bump
    let pool_index = 0u8;
    let (_, correct_bump) = find_token_pool_pda_with_index(&mint, pool_index);

    // Modify the bump in the compression data to an incorrect value
    if let Some(compression) = &mut compression_inputs.token_accounts[0].compression {
        compression.bump = correct_bump.wrapping_add(1); // Wrong bump
    }

    let ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &sender])
        .await;

    // Should fail with InvalidTokenPoolPda error (6023)
    assert_rpc_error(result, 0, 6023)?;

    Ok(())
}

#[tokio::test]
async fn test_spl_compression_invalid_pool_index() -> Result<(), RpcError> {
    // Test: Wrong pool index (use index 1 instead of 0)
    let SplCompressionTestContext {
        mut rpc,
        payer,
        sender,
        mint,
        mut compression_inputs,
        system_accounts_offset,
        ..
    } = setup_spl_compression_test(1000).await?;

    // Derive pool with index 1 instead of 0
    let wrong_pool_index = 1u8;
    let (wrong_pool_pda, wrong_bump) = find_token_pool_pda_with_index(&mint, wrong_pool_index);

    // Update the compression data with wrong pool index
    if let Some(compression) = &mut compression_inputs.token_accounts[0].compression {
        compression.bump = wrong_bump;
    }
    println!("compression_inputs {:?}", compression_inputs);
    let mut ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Update the pool account in the instruction
    let pool_account_index = system_accounts_offset + 6;
    ix.accounts[pool_account_index].pubkey = wrong_pool_pda;

    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &sender])
        .await;

    // Should fail with InvalidTokenPoolPda (6023) - pool derivation check fails because pool index doesn't match
    assert_rpc_error(result, 0, 6023)?;

    Ok(())
}

#[tokio::test]
async fn test_spl_compression_pool_account_out_of_bounds() -> Result<(), RpcError> {
    // Test: Pool account index out of bounds in packed accounts
    let SplCompressionTestContext {
        mut rpc,
        payer,
        sender,
        mut compression_inputs,
        ..
    } = setup_spl_compression_test(1000).await?;

    // Set pool account index to out of bounds value
    if let Some(compression) = &mut compression_inputs.token_accounts[0].compression {
        compression.pool_account_index = 100; // Way out of bounds
    }

    let ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &sender])
        .await;

    // Should fail with account out of bounds error (20014)
    assert_rpc_error(result, 0, 20014)?;

    Ok(())
}

#[tokio::test]
async fn test_spl_compression_pool_index_exceeds_max() -> Result<(), RpcError> {
    // Test: Pool index 6 (max is 5)
    let SplCompressionTestContext {
        mut rpc,
        payer,
        sender,
        mut compression_inputs,
        ..
    } = setup_spl_compression_test(1000).await?;

    // Set pool index to 6 (exceeds max of 5)
    if let Some(compression) = &mut compression_inputs.token_accounts[0].compression {
        compression.pool_index = 6;
    }

    let ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &sender])
        .await;

    // Should fail with InvalidTokenPoolPda (6023) - pool index 6 fails derivation check
    assert_rpc_error(result, 0, 6023)?;

    Ok(())
}

// ============================================================================
// Output Out of Bounds Tests
// ============================================================================

#[tokio::test]
async fn test_spl_compression_authority_out_of_bounds() -> Result<(), RpcError> {
    // Test: Authority index out of bounds
    let SplCompressionTestContext {
        mut rpc,
        payer,
        sender,
        mut compression_inputs,
        ..
    } = setup_spl_compression_test(1000).await?;

    // Set authority index to out of bounds
    if let Some(compression) = &mut compression_inputs.token_accounts[0].compression {
        compression.authority = 100; // Out of bounds
    }

    let ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &sender])
        .await;

    // Should fail with account out of bounds error (20014)
    assert_rpc_error(result, 0, 20014)?;

    Ok(())
}

#[tokio::test]
async fn test_spl_compression_mint_out_of_bounds() -> Result<(), RpcError> {
    // Test: Mint index out of bounds
    let SplCompressionTestContext {
        mut rpc,
        payer,
        sender,
        mut compression_inputs,
        ..
    } = setup_spl_compression_test(1000).await?;

    // Set mint index to out of bounds
    compression_inputs.token_accounts[0].output.mint = 100; // Out of bounds

    let ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &sender])
        .await;

    // Should fail with account out of bounds error (20014)
    assert_rpc_error(result, 0, 20014)?;

    Ok(())
}

#[tokio::test]
async fn test_spl_compression_recipient_out_of_bounds() -> Result<(), RpcError> {
    // Test: Recipient (CToken ATA owner) index out of bounds
    let SplCompressionTestContext {
        mut rpc,
        payer,
        sender,
        mut compression_inputs,
        ..
    } = setup_spl_compression_test(1000).await?;

    // Set recipient/owner index to out of bounds
    compression_inputs.token_accounts[0].output.owner = 100; // Out of bounds

    let ix = create_transfer2_instruction(compression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &sender])
        .await;

    // Should fail with account out of bounds error (20014)
    assert_rpc_error(result, 0, 20014)?;

    Ok(())
}

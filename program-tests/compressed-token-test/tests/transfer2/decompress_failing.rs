#![allow(clippy::result_large_err)]
#![allow(clippy::to_string_in_format_args)]
#![allow(clippy::unwrap_or_default)]

// ============================================================================
// DECOMPRESS TESTS (compressed → Solana account)
// ============================================================================
//
// Sum Check Failures:
// 1. amount more than output (should fail with output sum check)
// 2. amount less than output (should fail with input sum check)
//
// Authority Field Validation:
// 3. authority != 0 (MUST be 0 for decompress mode) → InvalidInstructionData
//   NOTE: Decompress doesn't use authority field, it must always be 0
//
// Input Out of Bounds:
// 4.1. mint out of bounds
// 4.2. recipient out of bounds
//
// SPL Token Decompression Pool Validation:
// 5. spl token decompression
//  5.1 invalid pool account (invalid derivation seed, valid pool index, valid bump)
//  5.2 invalid pool account (valid derivation seed, valid pool index, invalid bump)
//  5.3 invalid pool account (valid derivation seed, invalid pool index, valid bump)
//  5.4 pool account out of bounds
//  5.5 pool index 6 (higher than max 5)
//
// has_delegate Flag Mismatch:
// 6.1. Input: has_delegate=true but delegate=0
// 6.2. Input: has_delegate=false but delegate!=0
//

use light_client::indexer::{CompressedTokenAccount, Indexer};
use light_token_interface::{
    instructions::{mint_action::Recipient, transfer2::MultiInputTokenDataWithContext},
    state::TokenDataVersion,
};
use light_ctoken_sdk::{
    compressed_token::{
        create_compressed_mint::find_cmint_address,
        transfer2::{
            account_metas::Transfer2AccountsMetaConfig, create_transfer2_instruction,
            Transfer2Config, Transfer2Inputs,
        },
        CTokenAccount2,
    },
    ctoken::{derive_ctoken_ata, CompressibleParams, CreateAssociatedCTokenAccount},
    ValidityProof,
};
use light_program_test::{
    utils::assert::assert_rpc_error, LightProgramTest, ProgramTestConfig, Rpc,
};
use light_sdk::instruction::PackedAccounts;
use light_test_utils::RpcError;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

// ============================================================================
// Test Setup
// ============================================================================

/// Test context for decompression failing tests
struct DecompressionTestContext {
    pub rpc: LightProgramTest,
    pub payer: Keypair,
    pub owner: Keypair,
    pub decompression_inputs: Transfer2Inputs,
    pub system_accounts_offset: usize,
}

/// Set up test environment with compressed tokens and an empty CToken recipient account
async fn setup_decompression_test(
    compressed_amount: u64,
) -> Result<DecompressionTestContext, RpcError> {
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
    let (mint, _) = find_cmint_address(&mint_seed.pubkey());
    let (ctoken_ata, _) = derive_ctoken_ata(&owner.pubkey(), &mint);

    // Create compressible CToken ATA for owner (recipient of decompression)
    let compressible_params = CompressibleParams {
        compressible_config: rpc
            .test_accounts
            .funding_pool_config
            .compressible_config_pda,
        rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
        pre_pay_num_epochs: 2,
        lamports_per_write: Some(1000),
        compress_to_account_pubkey: None,
        token_account_version: TokenDataVersion::ShaFlat,
        compression_only: true, // ATAs require compression_only=true
    };

    let create_ata_instruction =
        CreateAssociatedCTokenAccount::new(payer.pubkey(), owner.pubkey(), mint)
            .with_compressible(compressible_params)
            .instruction()
            .map_err(|e| RpcError::AssertRpcError(format!("Failed to create ATA: {:?}", e)))?;

    rpc.create_and_send_transaction(&[create_ata_instruction], &payer.pubkey(), &[&payer])
        .await?;

    // Mint compressed tokens to owner and 1 token to decompressed CToken ATA
    let compressed_recipients = vec![Recipient::new(owner.pubkey(), compressed_amount)];
    let decompressed_recipients = vec![Recipient::new(owner.pubkey(), 0)];

    light_token_client::actions::mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &mint_authority,
        &payer,
        None,                    // no decompress mint
        false,                   // compress_and_close_cmint
        compressed_recipients,   // mint compressed tokens to owner
        decompressed_recipients, // mint 1 token to decompressed CToken ATA
        None,                    // no mint authority update
        None,                    // no freeze authority update
        Some(light_token_client::instructions::mint_action::NewMint {
            decimals: 6,
            supply: 0,
            mint_authority: mint_authority.pubkey(),
            freeze_authority: None,
            metadata: None,
            version: 3, // ShaFlat for mint hashing
        }),
    )
    .await?;

    // Get compressed token account from indexer
    let compressed_token_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;
    let compressed_token_account = compressed_token_accounts
        .iter()
        .find(|acc| acc.token.amount == compressed_amount)
        .expect("Should find compressed token account");

    // Build decompression Transfer2Inputs
    let decompression_inputs = create_decompression_inputs(
        compressed_token_account,
        ctoken_ata,
        compressed_amount,
        payer.pubkey(),
    )
    .await?;

    // Calculate system accounts offset
    let test_ix = create_transfer2_instruction(decompression_inputs.clone())
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    let first_packed_account = decompression_inputs
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

    Ok(DecompressionTestContext {
        rpc,
        payer,
        owner,
        decompression_inputs,
        system_accounts_offset,
    })
}

// ============================================================================
// Instruction Builder Helpers
// ============================================================================

/// Build Transfer2Inputs for decompression (compressed -> CToken ATA)
async fn create_decompression_inputs(
    compressed_token_account: &CompressedTokenAccount,
    ctoken_ata: Pubkey,
    decompress_amount: u64,
    fee_payer: Pubkey,
) -> Result<Transfer2Inputs, RpcError> {
    use light_compressed_account::compressed_account::PackedMerkleContext;

    let mut packed_accounts = PackedAccounts::default();

    // Add merkle tree and output queue (for outputs, even though we're decompressing)
    let merkle_tree = compressed_token_account.account.tree_info.tree;
    let queue = compressed_token_account.account.tree_info.queue;
    let tree_index = packed_accounts.insert_or_get(merkle_tree);
    let queue_index = packed_accounts.insert_or_get(queue);

    // Add mint and owner
    let mint_index = packed_accounts.insert_or_get_read_only(compressed_token_account.token.mint);
    let owner_index =
        packed_accounts.insert_or_get_config(compressed_token_account.token.owner, true, false); // is_signer, not writable

    // Add CToken ATA recipient account
    let ctoken_ata_index = packed_accounts.insert_or_get_config(ctoken_ata, false, true); // not signer, is writable
    println!("compressed_token_account: {:?}", compressed_token_account);
    // Manually create MultiInputTokenDataWithContext
    let has_delegate = compressed_token_account.token.delegate.is_some();
    let delegate_index = if has_delegate {
        packed_accounts.insert_or_get_read_only(
            compressed_token_account
                .token
                .delegate
                .unwrap_or(Pubkey::default()),
        )
    } else {
        0
    };

    let token_data = MultiInputTokenDataWithContext {
        owner: owner_index,
        amount: compressed_token_account.token.amount,
        has_delegate,
        delegate: delegate_index,
        mint: mint_index,
        version: 2, // Discriminator from the account data
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: tree_index,
            queue_pubkey_index: queue_index,
            leaf_index: compressed_token_account.account.leaf_index,
            prove_by_index: true, // Use proof by index
        },
        root_index: 0, // Not used when prove_by_index is true
    };

    // Create CTokenAccount2 with the multi-input token data
    let mut token_account = CTokenAccount2::new(vec![token_data]).map_err(|e| {
        RpcError::AssertRpcError(format!("Failed to create CTokenAccount2: {:?}", e))
    })?;

    // Add decompression
    token_account
        .decompress_token(decompress_amount, ctoken_ata_index)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to decompress: {:?}", e)))?;

    // Get account metas
    let (account_metas, _, _) = packed_accounts.to_account_metas();

    Ok(Transfer2Inputs {
        token_accounts: vec![token_account],
        validity_proof: ValidityProof::default(), // Use default proof for proof by index
        transfer_config: Transfer2Config::default().filter_zero_amount_outputs(),
        meta_config: Transfer2AccountsMetaConfig::new(fee_payer, account_metas),
        in_lamports: None,
        out_lamports: None,
        output_queue: queue_index,
        in_tlv: None,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_ctoken_decompression_functional() -> Result<(), RpcError> {
    // Baseline test: valid decompression should succeed
    let DecompressionTestContext {
        mut rpc,
        payer,
        owner,
        decompression_inputs,
        system_accounts_offset: _,
    } = setup_decompression_test(1000).await?;

    // Create instruction from Transfer2Inputs
    let ix = create_transfer2_instruction(decompression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Send transaction with owner as signer
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should succeed
    assert!(
        result.is_ok(),
        "Valid decompression should succeed: {:?}",
        result.err()
    );

    Ok(())
}

#[tokio::test]
async fn test_decompression_amount_more_than_output() -> Result<(), RpcError> {
    // Test: Decompression amount more than output (output sum check should fail)
    // Decompress 1000 tokens but we only have 1000 in input
    let DecompressionTestContext {
        mut rpc,
        payer,
        owner,
        mut decompression_inputs,
        system_accounts_offset: _,
    } = setup_decompression_test(1000).await?;

    // Increase decompression amount by 1 (input sum is 1000, but decompression is 1001)
    // This breaks the sum check: input sum < output sum (from decompressions)
    decompression_inputs.token_accounts[0]
        .compression
        .as_mut()
        .unwrap()
        .amount += 1;

    // Create instruction from modified Transfer2Inputs
    let ix = create_transfer2_instruction(decompression_inputs)
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
async fn test_decompression_amount_less_than_output() -> Result<(), RpcError> {
    // Test: Decompression amount less than output (input sum check should fail)
    // Decompress 999 tokens but we have 1000 in input
    let DecompressionTestContext {
        mut rpc,
        payer,
        owner,
        mut decompression_inputs,
        system_accounts_offset: _,
    } = setup_decompression_test(1000).await?;

    // Decrease decompression amount by 1 (input sum is 1000, but decompression is 999)
    // This breaks the sum check: input sum > output sum (from decompressions)
    decompression_inputs.token_accounts[0]
        .compression
        .as_mut()
        .unwrap()
        .amount -= 1;

    // Create instruction from modified Transfer2Inputs
    let ix = create_transfer2_instruction(decompression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Send transaction with owner as signer
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with SumcheckFailed (6005)
    light_program_test::utils::assert::assert_rpc_error(result, 0, 6005).unwrap();

    Ok(())
}

#[tokio::test]
async fn test_decompression_mint_out_of_bounds() -> Result<(), RpcError> {
    // Test: Mint index out of bounds in input
    let DecompressionTestContext {
        mut rpc,
        payer,
        owner,
        mut decompression_inputs,
        system_accounts_offset: _,
    } = setup_decompression_test(1000).await?;

    // Get the number of packed accounts
    let num_packed_accounts = decompression_inputs
        .meta_config
        .packed_accounts
        .as_ref()
        .unwrap()
        .len();

    // Set mint index to out of bounds value in input
    decompression_inputs.token_accounts[0].inputs[0].mint = num_packed_accounts as u8;

    // Create instruction from modified Transfer2Inputs
    let ix = create_transfer2_instruction(decompression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Send transaction with owner as signer
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with "insufficient account keys for instruction"
    assert_rpc_error(result, 0, 20014).unwrap();

    Ok(())
}

#[tokio::test]
async fn test_decompression_recipient_out_of_bounds() -> Result<(), RpcError> {
    // Test: Recipient (CToken ATA) index out of bounds in decompression
    let DecompressionTestContext {
        mut rpc,
        payer,
        owner,
        mut decompression_inputs,
        system_accounts_offset: _,
    } = setup_decompression_test(1000).await?;

    // Get the number of packed accounts
    let num_packed_accounts = decompression_inputs
        .meta_config
        .packed_accounts
        .as_ref()
        .unwrap()
        .len();

    // Set recipient (CToken ATA) index to out of bounds value in decompression
    decompression_inputs.token_accounts[0]
        .compression
        .as_mut()
        .unwrap()
        .source_or_recipient = num_packed_accounts as u8;

    // Create instruction from modified Transfer2Inputs
    let ix = create_transfer2_instruction(decompression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Send transaction with owner as signer
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with NotEnoughAccountKeys (20014)
    light_program_test::utils::assert::assert_rpc_error(result, 0, 20014).unwrap();

    Ok(())
}

#[tokio::test]
async fn test_decompression_has_delegate_true_but_delegate_zero() -> Result<(), RpcError> {
    // Test: Input has_delegate=true but delegate index is 0
    let DecompressionTestContext {
        mut rpc,
        payer,
        owner,
        mut decompression_inputs,
        system_accounts_offset: _,
    } = setup_decompression_test(1000).await?;

    // Set has_delegate to true but keep delegate index at 0
    decompression_inputs.token_accounts[0].inputs[0].has_delegate = true;
    decompression_inputs.token_accounts[0].inputs[0].delegate = 0;

    // Create instruction from modified Transfer2Inputs
    let ix = create_transfer2_instruction(decompression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Send transaction with owner as signer
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with hash mismatch (14307) because modifying has_delegate changes the token data hash
    // This is the expected behavior - the invalid delegate configuration is caught during hash validation
    light_program_test::utils::assert::assert_rpc_error(result, 0, 14307).unwrap();

    Ok(())
}

#[tokio::test]
async fn test_decompression_has_delegate_false_but_delegate_nonzero() -> Result<(), RpcError> {
    // Test: Input has_delegate=false but delegate index is non-zero, and we try to sign with delegate
    let DecompressionTestContext {
        mut rpc,
        payer,
        owner: _,
        mut decompression_inputs,
        system_accounts_offset,
    } = setup_decompression_test(1000).await?;

    // Create a fake delegate keypair
    let fake_delegate = Keypair::new();
    rpc.airdrop_lamports(&fake_delegate.pubkey(), 1_000_000_000)
        .await?;

    // Add the delegate to packed accounts and get its index
    let delegate_index = decompression_inputs
        .meta_config
        .packed_accounts
        .as_mut()
        .unwrap()
        .len() as u8;

    decompression_inputs
        .meta_config
        .packed_accounts
        .as_mut()
        .unwrap()
        .push(solana_sdk::instruction::AccountMeta::new_readonly(
            fake_delegate.pubkey(),
            false, // is_signer
        ));

    // Set has_delegate to false but set delegate index to the fake delegate
    decompression_inputs.token_accounts[0].inputs[0].has_delegate = false;
    decompression_inputs.token_accounts[0].inputs[0].delegate = delegate_index;
    // // Replace owner with fake delegate in the instruction accounts
    let owner_packed_index = decompression_inputs.token_accounts[0].inputs[0].owner;

    // Create instruction from modified Transfer2Inputs
    let mut ix = create_transfer2_instruction(decompression_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;
    ix.accounts[system_accounts_offset + owner_packed_index as usize].is_signer = false;

    // Send transaction with fake delegate as signer instead of owner
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
        .await;

    // Should fail with OwnerMismatch (6075 = 6000 + 75) since owner must sign
    light_program_test::utils::assert::assert_rpc_error(result, 0, 6075).unwrap();

    Ok(())
}

#![allow(clippy::result_large_err)]

// ============================================================================
// TRANSFER2 FAILING TESTS - COMPREHENSIVE COVERAGE
// ============================================================================

use light_client::indexer::{CompressedTokenAccount, Indexer};
use light_compressed_token_sdk::{
    account2::CTokenAccount2,
    instructions::transfer2::{
        account_metas::Transfer2AccountsMetaConfig, create_transfer2_instruction, Transfer2Config,
        Transfer2Inputs,
    },
    ValidityProof,
};
use light_ctoken_types::{
    instructions::{mint_action::Recipient, transfer2::MultiInputTokenDataWithContext},
    state::TokenDataVersion,
};
use light_program_test::{
    utils::assert::assert_rpc_error, LightProgramTest, ProgramTestConfig, Rpc,
};
use light_sdk::instruction::PackedAccounts;
use light_test_utils::{airdrop_lamports, RpcError};
use light_token_client::actions::{create_mint, mint_to_compressed, transfer2::approve};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
// ============================================================================
// Test Setup
// ============================================================================

/// Simple test context for transfer2 failing tests
struct TransferTestContext {
    pub rpc: LightProgramTest,
    pub payer: Keypair,
    pub owner: Keypair,
    pub recipient: Keypair,
    pub transfer2_inputs: Transfer2Inputs,
    pub system_accounts_offset: usize, // Offset to add to packed account indices to get instruction account indices
}

/// Set up a simple test environment with one compressed mint and one account with tokens
async fn setup_transfer_test(
    token_amount: u64,
    token_version: TokenDataVersion,
) -> Result<TransferTestContext, RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();

    // Create owner and airdrop lamports
    let owner = Keypair::new();
    airdrop_lamports(&mut rpc, &owner.pubkey(), 1_000_000_000).await?;

    // Create recipient and airdrop lamports
    let recipient = Keypair::new();
    airdrop_lamports(&mut rpc, &recipient.pubkey(), 1_000_000_000).await?;

    // Create mint authority
    let mint_authority = Keypair::new();

    // Create compressed mint
    let mint_seed = Keypair::new();
    create_mint(
        &mut rpc,
        &mint_seed,
        6, // decimals
        &mint_authority,
        None, // freeze authority
        None, // metadata
        &payer,
    )
    .await?;

    let mint =
        light_compressed_token_sdk::instructions::find_spl_mint_address(&mint_seed.pubkey()).0;

    // Mint tokens to owner if amount > 0
    if token_amount > 0 {
        let recipients = vec![Recipient {
            recipient: owner.pubkey().into(),
            amount: token_amount,
        }];

        mint_to_compressed(
            &mut rpc,
            mint,
            recipients,
            token_version,
            &mint_authority,
            &payer,
        )
        .await?;
    }

    // Fetch owner's compressed token accounts
    let owner_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await?
        .value
        .items;

    // Build Transfer2Inputs for a transfer of 500 tokens (half the balance)
    let transfer_amount = token_amount / 2;
    let transfer2_inputs = create_transfer2_inputs(
        &owner_accounts,
        recipient.pubkey(),
        transfer_amount,
        payer.pubkey(),
        1, // output_merkle_tree_index
    )?;

    // Calculate system accounts offset by creating a test instruction
    // and finding where the first packed account appears
    let test_ix = create_transfer2_instruction(transfer2_inputs.clone())
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Find the first packed account (merkle tree at packed index 0)
    let first_packed_account = transfer2_inputs
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

    Ok(TransferTestContext {
        rpc,
        payer,
        owner,
        recipient,
        transfer2_inputs,
        system_accounts_offset,
    })
}

// ============================================================================
// Instruction Builder Helpers
// ============================================================================

/// Build Transfer2Inputs from compressed token accounts
/// This uses the low-level SDK abstractions for maximum control in failing tests
/// Returns Transfer2Inputs so tests can modify it before creating the instruction
fn create_transfer2_inputs(
    compressed_accounts: &[CompressedTokenAccount],
    recipient: Pubkey,
    transfer_amount: u64,
    fee_payer: Pubkey,
    output_merkle_tree_index: u8,
) -> Result<Transfer2Inputs, RpcError> {
    assert_eq!(
        compressed_accounts.len(),
        1,
        "This helper only supports one input account"
    );
    let account = &compressed_accounts[0];

    // Use PackedAccounts to manage account packing
    let mut packed_accounts = PackedAccounts::default();

    // Add tree accounts first (merkle tree and queue)
    packed_accounts.insert_or_get(account.account.tree_info.tree);
    packed_accounts.insert_or_get(account.account.tree_info.queue);

    // Add mint, owner, recipient
    let mint_index = packed_accounts.insert_or_get_read_only(account.token.mint);
    let owner_index = packed_accounts.insert_or_get_config(account.token.owner, true, false);
    let recipient_index = packed_accounts.insert_or_get_read_only(recipient);

    // Handle delegate
    let (has_delegate, delegate_index) = if let Some(delegate) = account.token.delegate {
        (true, packed_accounts.insert_or_get_read_only(delegate))
    } else {
        (false, 0)
    };

    // Build PackedMerkleContext - tree and queue are at indices 0 and 1
    let packed_merkle_context = light_compressed_account::compressed_account::PackedMerkleContext {
        merkle_tree_pubkey_index: 0,
        queue_pubkey_index: 1,
        leaf_index: account.account.leaf_index,
        prove_by_index: true,
    };

    // Get token version
    let version =
        TokenDataVersion::from_discriminator(account.account.data.as_ref().unwrap().discriminator)
            .unwrap() as u8;

    // Create input token data
    let input_token_data = vec![MultiInputTokenDataWithContext {
        owner: owner_index,
        amount: account.token.amount,
        has_delegate,
        delegate: delegate_index,
        mint: mint_index,
        version,
        merkle_context: packed_merkle_context,
        root_index: 0,
    }];

    // Create CTokenAccount2 from input
    let mut sender_account = CTokenAccount2::new(input_token_data).map_err(|e| {
        RpcError::AssertRpcError(format!("Failed to create CTokenAccount2: {:?}", e))
    })?;

    // Transfer to recipient (creates recipient output account)
    let recipient_account = sender_account
        .transfer(recipient_index, transfer_amount)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to transfer: {:?}", e)))?;

    // Get account metas from PackedAccounts
    let (account_metas, _, _) = packed_accounts.to_account_metas();

    // Build and return Transfer2Inputs
    // token_accounts contains: [sender with change, recipient]
    Ok(Transfer2Inputs {
        token_accounts: vec![sender_account, recipient_account],
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
//
// NOTE: We don't test merkle tree validation or proof validation - that's
// handled by the system program. Focus is on transfer2-specific validation.
// 1. invalid owner has signed

#[tokio::test]
async fn test_invalid_owner_signed() -> Result<(), RpcError> {
    // Test: Invalid owner has signed
    let TransferTestContext {
        mut rpc,
        payer,
        owner: _,
        recipient: invalid_owner,
        transfer2_inputs,
        system_accounts_offset,
    } = setup_transfer_test(1000, TokenDataVersion::ShaFlat).await?;

    let owner_packed_index = transfer2_inputs.token_accounts[0].inputs[0].owner;

    // Create instruction from Transfer2Inputs
    let mut ix = create_transfer2_instruction(transfer2_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Replace owner account with invalid owner (using system_accounts_offset)
    ix.accounts[system_accounts_offset + owner_packed_index as usize].pubkey =
        invalid_owner.pubkey();

    // Send transaction with invalid owner as signer
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &invalid_owner])
        .await;

    // Should fail with InvalidHash because the hash is computed with owner as part of account data
    assert_rpc_error(result, 0, 14307).unwrap();

    Ok(())
}

#[tokio::test]
async fn test_owner_not_signer() -> Result<(), RpcError> {
    // Test: Owner is valid but not signer
    let TransferTestContext {
        mut rpc,
        payer,
        owner: _,
        recipient: _,
        transfer2_inputs,
        system_accounts_offset,
    } = setup_transfer_test(1000, TokenDataVersion::ShaFlat).await?;

    let owner_packed_index = transfer2_inputs.token_accounts[0].inputs[0].owner;

    // Create instruction from Transfer2Inputs
    let mut ix = create_transfer2_instruction(transfer2_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Replace owner account with invalid owner (using system_accounts_offset)
    ix.accounts[system_accounts_offset + owner_packed_index as usize].is_signer = false;

    // Send transaction with invalid owner as signer
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
        .await;

    // Should fail with InvalidSigner
    assert_rpc_error(result, 0, 20009).unwrap();

    Ok(())
}

#[tokio::test]
async fn test_unbalanced_transfer_too_little_inputs() -> Result<(), RpcError> {
    let TransferTestContext {
        mut rpc,
        payer,
        owner,
        recipient: _,
        mut transfer2_inputs,
        system_accounts_offset: _,
    } = setup_transfer_test(1000, TokenDataVersion::ShaFlat).await?;

    transfer2_inputs.token_accounts[0].inputs[0].amount -= 1;

    // Create instruction from Transfer2Inputs
    let ix = create_transfer2_instruction(transfer2_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Send transaction with invalid owner as signer
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with ComputeOutputSumFailed
    assert_rpc_error(result, 0, 6002).unwrap();

    Ok(())
}

#[tokio::test]
async fn test_unbalanced_transfer_too_many_inputs() -> Result<(), RpcError> {
    let TransferTestContext {
        mut rpc,
        payer,
        owner,
        recipient: _,
        mut transfer2_inputs,
        system_accounts_offset: _,
    } = setup_transfer_test(1000, TokenDataVersion::ShaFlat).await?;

    transfer2_inputs.token_accounts[0].inputs[0].amount += 1;

    // Create instruction from Transfer2Inputs
    let ix = create_transfer2_instruction(transfer2_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Send transaction with invalid owner as signer
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with SumCheckFailed
    assert_rpc_error(result, 0, 6005).unwrap();

    Ok(())
}

#[tokio::test]
async fn test_unbalanced_transfer_too_little_outputs() -> Result<(), RpcError> {
    let TransferTestContext {
        mut rpc,
        payer,
        owner,
        recipient: _,
        mut transfer2_inputs,
        system_accounts_offset: _,
    } = setup_transfer_test(1000, TokenDataVersion::ShaFlat).await?;

    transfer2_inputs.token_accounts[0].output.amount -= 1;

    // Create instruction from Transfer2Inputs
    let ix = create_transfer2_instruction(transfer2_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Send transaction with invalid owner as signer
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with ComputeOutputSumFailed
    assert_rpc_error(result, 0, 6005).unwrap();

    Ok(())
}

#[tokio::test]
async fn test_unbalanced_transfer_too_many_outputs() -> Result<(), RpcError> {
    let TransferTestContext {
        mut rpc,
        payer,
        owner,
        recipient: _,
        mut transfer2_inputs,
        system_accounts_offset: _,
    } = setup_transfer_test(1000, TokenDataVersion::ShaFlat).await?;

    transfer2_inputs.token_accounts[0].output.amount += 1;

    // Create instruction from Transfer2Inputs
    let ix = create_transfer2_instruction(transfer2_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Send transaction with invalid owner as signer
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with ComputeOutputSumFailed
    assert_rpc_error(result, 0, 6002).unwrap();

    Ok(())
}

#[tokio::test]
async fn test_invalid_mint() -> Result<(), RpcError> {
    // Test: Invalid mint in input (should fail with InvalidHash because mint is part of hash)
    // Keep sum check balanced by using same mint index for all accounts
    let TransferTestContext {
        mut rpc,
        payer,
        owner,
        recipient: _,
        mut transfer2_inputs,
        system_accounts_offset: _,
    } = setup_transfer_test(1000, TokenDataVersion::ShaFlat).await?;

    let recipient_packed_index = transfer2_inputs.token_accounts[1].output.owner;

    // Change mint index in both input and outputs to keep sum check balanced
    // but the input hash will be wrong because the actual compressed account has different mint
    transfer2_inputs.token_accounts[0].inputs[0].mint = recipient_packed_index;
    transfer2_inputs.token_accounts[0].output.mint = recipient_packed_index;
    transfer2_inputs.token_accounts[1].output.mint = recipient_packed_index;

    // Create instruction from Transfer2Inputs
    let ix = create_transfer2_instruction(transfer2_inputs)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    // Send transaction
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    // Should fail with InvalidHash (14307) because mint is part of the account hash
    assert_rpc_error(result, 0, 14307).unwrap();

    Ok(())
}

#[tokio::test]
async fn test_invalid_version() -> Result<(), RpcError> {
    // Test all combinations of (correct_version, wrong_version)
    let version_combinations = vec![
        (TokenDataVersion::V1, TokenDataVersion::V2),
        (TokenDataVersion::V1, TokenDataVersion::ShaFlat),
        (TokenDataVersion::V2, TokenDataVersion::V1),
        (TokenDataVersion::V2, TokenDataVersion::ShaFlat),
        (TokenDataVersion::ShaFlat, TokenDataVersion::V1),
        (TokenDataVersion::ShaFlat, TokenDataVersion::V2),
    ];

    for (correct_version, wrong_version) in version_combinations {
        // Test: Invalid version in input (should fail with InvalidHash because version affects hash)
        let TransferTestContext {
            mut rpc,
            payer,
            owner,
            recipient: _,
            mut transfer2_inputs,
            system_accounts_offset: _,
        } = setup_transfer_test(1000, correct_version).await?;

        // Change version to wrong version
        // The actual compressed account was created with correct_version, so hash will be wrong
        transfer2_inputs.token_accounts[0].inputs[0].version = wrong_version as u8;

        // Create instruction from Transfer2Inputs
        let ix = create_transfer2_instruction(transfer2_inputs).map_err(|e| {
            RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e))
        })?;

        // Send transaction
        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
            .await;

        // Should fail with InvalidHash (14307) because version affects how hash is computed
        assert_rpc_error(result, 0, 14307).unwrap();
    }

    Ok(())
}

#[tokio::test]
async fn test_input_out_of_bounds() -> Result<(), RpcError> {
    // Test: Input indices out of bounds (owner, delegate, mint)
    let TransferTestContext {
        mut rpc,
        payer,
        owner,
        recipient: _,
        transfer2_inputs,
        system_accounts_offset: _,
    } = setup_transfer_test(1000, TokenDataVersion::ShaFlat).await?;

    // Get the number of packed accounts
    let num_packed_accounts = transfer2_inputs
        .meta_config
        .packed_accounts
        .as_ref()
        .unwrap()
        .len();

    // Test owner out of bounds
    {
        let mut inputs = transfer2_inputs.clone();
        inputs.token_accounts[0].inputs[0].owner = num_packed_accounts as u8;

        let ix = create_transfer2_instruction(inputs).map_err(|e| {
            RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e))
        })?;

        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
            .await;

        // Should fail with AccountError::NotEnoughAccountKeys (20014)
        assert_rpc_error(result, 0, 20014).unwrap();
    }

    // Test delegate out of bounds (set has_delegate=true and delegate to out of bounds)
    {
        let mut inputs = transfer2_inputs.clone();
        inputs.token_accounts[0].inputs[0].has_delegate = true;
        inputs.token_accounts[0].inputs[0].delegate = num_packed_accounts as u8;

        let ix = create_transfer2_instruction(inputs).map_err(|e| {
            RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e))
        })?;

        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
            .await;

        // Should fail with AccountError::NotEnoughAccountKeys (20014)
        assert_rpc_error(result, 0, 20014).unwrap();
    }

    // Test mint out of bounds
    {
        let mut inputs = transfer2_inputs.clone();
        inputs.token_accounts[0].inputs[0].mint = num_packed_accounts as u8;

        let ix = create_transfer2_instruction(inputs).map_err(|e| {
            RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e))
        })?;

        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
            .await;

        // Should fail with AccountError::NotEnoughAccountKeys (20014)
        assert_rpc_error(result, 0, 20014).unwrap();
    }

    Ok(())
}

#[tokio::test]
async fn test_output_out_of_bounds() -> Result<(), RpcError> {
    // Test: Output indices out of bounds (owner, delegate, mint)
    let TransferTestContext {
        mut rpc,
        payer,
        owner,
        recipient: _,
        transfer2_inputs,
        system_accounts_offset: _,
    } = setup_transfer_test(1000, TokenDataVersion::ShaFlat).await?;

    // Get the number of packed accounts
    let num_packed_accounts = transfer2_inputs
        .meta_config
        .packed_accounts
        .as_ref()
        .unwrap()
        .len();

    // Test owner out of bounds in output
    {
        let mut inputs = transfer2_inputs.clone();
        inputs.token_accounts[1].output.owner = num_packed_accounts as u8;

        let ix = create_transfer2_instruction(inputs).map_err(|e| {
            RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e))
        })?;

        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
            .await;

        // Should fail with AccountError::NotEnoughAccountKeys (20014)
        assert_rpc_error(result, 0, 20014).unwrap();
    }

    // Test delegate out of bounds in output (set has_delegate=true and delegate to out of bounds)
    {
        let mut inputs = transfer2_inputs.clone();
        inputs.token_accounts[1].output.has_delegate = true;
        inputs.token_accounts[1].output.delegate = num_packed_accounts as u8;

        let ix = create_transfer2_instruction(inputs).map_err(|e| {
            RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e))
        })?;

        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
            .await;

        // Should fail with AccountError::NotEnoughAccountKeys (20014)
        assert_rpc_error(result, 0, 20014).unwrap();
    }

    // Test mint out of bounds in output
    {
        let mut inputs = transfer2_inputs.clone();
        inputs.token_accounts[1].output.mint = num_packed_accounts as u8;

        let ix = create_transfer2_instruction(inputs).map_err(|e| {
            RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e))
        })?;

        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
            .await;

        // Should fail with AccountError::NotEnoughAccountKeys (20014)
        assert_rpc_error(result, 0, 20014).unwrap();
    }

    Ok(())
}

/// Set up test environment with delegated token account
/// Delegates the full token amount to a delegate
async fn setup_transfer_test_with_delegate(
    token_amount: u64,
    token_version: TokenDataVersion,
) -> Result<(TransferTestContext, Keypair), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();

    // Create owner and airdrop lamports
    let owner = Keypair::new();
    airdrop_lamports(&mut rpc, &owner.pubkey(), 1_000_000_000).await?;

    // Create recipient and airdrop lamports
    let recipient = Keypair::new();
    airdrop_lamports(&mut rpc, &recipient.pubkey(), 1_000_000_000).await?;

    // Create delegate and airdrop lamports
    let delegate = Keypair::new();
    airdrop_lamports(&mut rpc, &delegate.pubkey(), 1_000_000_000).await?;

    // Create mint authority
    let mint_authority = Keypair::new();

    // Create compressed mint
    let mint_seed = Keypair::new();
    create_mint(
        &mut rpc,
        &mint_seed,
        6, // decimals
        &mint_authority,
        None, // freeze authority
        None, // metadata
        &payer,
    )
    .await?;

    let mint =
        light_compressed_token_sdk::instructions::find_spl_mint_address(&mint_seed.pubkey()).0;

    // Mint tokens to owner if amount > 0
    if token_amount > 0 {
        let recipients = vec![Recipient {
            recipient: owner.pubkey().into(),
            amount: token_amount,
        }];

        mint_to_compressed(
            &mut rpc,
            mint,
            recipients,
            token_version,
            &mint_authority,
            &payer,
        )
        .await?;
    }

    // Fetch owner's compressed token accounts
    let owner_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await?
        .value
        .items;
    println!("owner_accounts {:?}", owner_accounts);

    // Approve delegate for the full amount
    approve(
        &mut rpc,
        &owner_accounts,
        delegate.pubkey(),
        token_amount, // delegate full amount
        &owner,
        &payer,
    )
    .await?;

    // Fetch updated token accounts with delegate
    let owner_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await?
        .value
        .items;
    println!("owner_accounts {:?}", owner_accounts);
    // Build Transfer2Inputs for a transfer of half the tokens
    // This ensures we have both inputs and outputs to test has_delegate flags
    let transfer_amount = token_amount / 2;
    let transfer2_inputs = create_transfer2_inputs(
        &owner_accounts,
        recipient.pubkey(),
        transfer_amount,
        payer.pubkey(),
        1, // output_merkle_tree_index
    )?;

    // Calculate system accounts offset
    let test_ix = create_transfer2_instruction(transfer2_inputs.clone())
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e)))?;

    let first_packed_account = transfer2_inputs
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

    let context = TransferTestContext {
        rpc,
        payer,

        owner,
        recipient,
        transfer2_inputs,
        system_accounts_offset,
    };

    Ok((context, delegate))
}

#[tokio::test]
async fn test_has_delegate_flag_mismatch() -> Result<(), RpcError> {
    // Test all 4 has_delegate flag mismatch scenarios

    // 11.1. Input: has_delegate=true but delegate=0
    // Tests hash validation when has_delegate flag is set but delegate index is 0
    // The computed hash will be wrong because the actual compressed account has a non-zero delegate
    {
        let (
            TransferTestContext {
                mut rpc,
                payer,
                owner,
                recipient: _,
                mut transfer2_inputs,
                system_accounts_offset: _,
            },
            _,
        ) = setup_transfer_test_with_delegate(1000, TokenDataVersion::ShaFlat).await?;
        println!("transfer2_inputs {:?}", transfer2_inputs);

        // Set has_delegate=true but delegate index to 0 (mismatch with actual account)
        transfer2_inputs.token_accounts[0].inputs[0].has_delegate = true;
        transfer2_inputs.token_accounts[0].inputs[0].delegate = 0;

        let ix = create_transfer2_instruction(transfer2_inputs).map_err(|e| {
            RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e))
        })?;

        // Owner signs (input account owner must sign)
        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
            .await;

        // Should fail with InvalidHash (14307) because delegate field is part of hash
        assert_rpc_error(result, 0, 14307).unwrap();
    }

    // 11.2. Input: has_delegate=false but delegate!=0
    // Tests hash validation when has_delegate flag is false but delegate index is non-zero
    // The computed hash will be wrong because the actual compressed account has has_delegate=true
    {
        let (
            TransferTestContext {
                mut rpc,
                payer,
                owner,
                recipient: _,
                mut transfer2_inputs,
                system_accounts_offset: _,
            },
            _,
        ) = setup_transfer_test_with_delegate(1000, TokenDataVersion::ShaFlat).await?;

        // Set has_delegate=false but keep delegate index non-zero (mismatch with actual account)
        transfer2_inputs.token_accounts[0].inputs[0].has_delegate = false;
        // delegate_index is already non-zero from the delegated account

        let ix = create_transfer2_instruction(transfer2_inputs).map_err(|e| {
            RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e))
        })?;

        // Owner signs (input account owner must sign)
        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &owner])
            .await;

        // Should fail with InvalidHash (14307) because has_delegate flag is part of hash
        assert_rpc_error(result, 0, 14307).unwrap();
    }

    // 11.3. Invalid delegate signing
    // Tests that an invalid delegate (not the actual delegate) cannot sign for the input account
    // The delegate pubkey in the instruction is replaced with an invalid delegate
    {
        let (
            TransferTestContext {
                mut rpc,
                payer,
                owner: _,
                recipient: _,
                mut transfer2_inputs,
                system_accounts_offset,
            },
            _,
        ) = setup_transfer_test_with_delegate(1000, TokenDataVersion::ShaFlat).await?;

        // Create an invalid delegate (not the actual delegate from approve())
        let invalid_delegate = Keypair::new();
        airdrop_lamports(&mut rpc, &invalid_delegate.pubkey(), 1_000_000_000).await?;

        // Set output has_delegate=true but delegate index to 0 (for variety)
        transfer2_inputs.token_accounts[1].output.has_delegate = true;
        transfer2_inputs.token_accounts[1].output.delegate = 0;

        let owner_packed_index = transfer2_inputs.token_accounts[0].inputs[0].owner;
        let delegate_packed_index = transfer2_inputs.token_accounts[0].inputs[0].delegate;

        let mut ix = create_transfer2_instruction(transfer2_inputs).map_err(|e| {
            RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e))
        })?;

        // Modify instruction: owner not signer, invalid delegate is signer
        ix.accounts[system_accounts_offset + owner_packed_index as usize].is_signer = false;
        ix.accounts[system_accounts_offset + delegate_packed_index as usize].is_signer = true;
        ix.accounts[system_accounts_offset + delegate_packed_index as usize].pubkey =
            invalid_delegate.pubkey();

        // Sign with invalid delegate instead of the real delegate
        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &invalid_delegate])
            .await;

        // Should fail with InvalidHash (14307) because the delegate pubkey in instruction
        // doesn't match the delegate in the compressed account hash
        assert_rpc_error(result, 0, 14307).unwrap();
    }

    // 11.4. No signer (neither owner nor delegate signs)
    // Tests that the transaction fails when neither the owner nor delegate provides a signature
    {
        let (
            TransferTestContext {
                mut rpc,
                payer,
                owner: _,
                recipient: _,
                transfer2_inputs,
                system_accounts_offset,
            },
            _,
        ) = setup_transfer_test_with_delegate(1000, TokenDataVersion::ShaFlat).await?;

        let owner_packed_index = transfer2_inputs.token_accounts[0].inputs[0].owner;
        let delegate_packed_index = transfer2_inputs.token_accounts[0].inputs[0].delegate;

        let mut ix = create_transfer2_instruction(transfer2_inputs).map_err(|e| {
            RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e))
        })?;

        // Modify instruction: owner not signer, delegate not signer
        ix.accounts[system_accounts_offset + owner_packed_index as usize].is_signer = false;
        ix.accounts[system_accounts_offset + delegate_packed_index as usize].is_signer = false;

        // Don't provide owner or delegate as signers (only payer)
        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
            .await;

        // Should fail with InvalidSigner (20009) because no valid authority signed
        assert_rpc_error(result, 0, 20009).unwrap();
    }

    // 11.5. Valid delegate signing (should succeed)
    // Baseline test: valid delegate signing for a delegated input account should work
    {
        let (
            TransferTestContext {
                mut rpc,
                payer,
                owner: _,
                recipient: _,
                transfer2_inputs,
                system_accounts_offset,
            },
            delegate,
        ) = setup_transfer_test_with_delegate(1000, TokenDataVersion::ShaFlat).await?;

        let owner_packed_index = transfer2_inputs.token_accounts[0].inputs[0].owner;
        let delegate_packed_index = transfer2_inputs.token_accounts[0].inputs[0].delegate;

        let mut ix = create_transfer2_instruction(transfer2_inputs).map_err(|e| {
            RpcError::AssertRpcError(format!("Failed to create instruction: {:?}", e))
        })?;

        // Modify instruction: owner not signer, delegate is signer
        ix.accounts[system_accounts_offset + owner_packed_index as usize].is_signer = false;
        ix.accounts[system_accounts_offset + delegate_packed_index as usize].is_signer = true;

        // Sign with valid delegate (the one set via approve())
        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &delegate])
            .await;

        // Should succeed because the correct delegate is signing
        assert!(
            result.is_ok(),
            "Should succeed when valid delegate signs: {:?}",
            result.err()
        );
    }

    Ok(())
}

//
// ============================================================================
// TRANSFER TESTS (compressed-to-compressed transfers)
// ============================================================================
//
// Authority Validation:
// 1. invalid owner has signed
// 2. owner is valid but not signer
// 3. invalid delegate has signed
// 4. delegate is valid but not signer (owner hasn't signed either)
//
// Sum Check Failures:
// 5. unbalanced transfer (too little inputs) (should fail with input sum check)
// 6. unbalanced transfer (too little outputs) (should fail with output sum check)
//
// Hash Validation:
// 7. invalid mint (should fail with 14137 invalid hash)
// 8. invalid version (should fail with 14137 invalid hash)
//
// Input Out of Bounds:
// 9.1. owner out of bounds
// 9.2. delegate out of bounds
// 9.3. mint out of bounds
//
// Output Out of Bounds:
// 10.1. owner out of bounds
// 10.2. delegate out of bounds
// 10.3. mint out of bounds
//
// has_delegate Flag Mismatch:
// 11.1. Input: has_delegate=true but delegate=0 (flag set but no delegate)
// 11.2. Input: has_delegate=false but delegate!=0 (delegate set but flag off)
// 11.3. Output: has_delegate=true but delegate=0
// 11.4. Output: has_delegate=false but delegate!=0
//

// ============================================================================
// COMPRESS AND CLOSE TESTS (compress full balance + close account)
// ============================================================================
//
// Owner-Based Close:
// 1. wrong amount (not full balance) → CompressAndCloseAmountMismatch (6090)
// 2. account has delegate set → CompressAndCloseDelegateNotAllowed (6092)
// 3. invalid authority (not owner) → InvalidSigner
// 4. authority is not signer → InvalidSigner
// 5. missing destination → CompressAndCloseDestinationMissing (6087)
//
// Rent Authority Close (compressible accounts):
// 6. rent authority closes non-compressible account → OwnerMismatch
// 7. rent authority closes account that's not yet compressible (is_compressible() = false)
// 8. invalid rent_sponsor account (wrong PDA)
// 9. compressed output missing in out_token_data
// 10. compressed output amount mismatch (doesn't match full balance)
// 11. compressed output owner mismatch (when compress_to_pubkey=false)
// 12. compressed output owner not account pubkey (when compress_to_pubkey=true)
// 13. compressed output has delegate set (has_delegate=true or delegate!=0)
// 14. compressed output wrong version (not ShaFlat/version 3)
// 15. version mismatch with compressible extension's token_account_version
//
// Index Out of Bounds:
// 16. rent_sponsor_index out of bounds
// 17. compressed_account_index out of bounds (in pool_index field)
// 18. destination_index out of bounds (in bump field)
// 19. source account index out of bounds
// 20. mint index out of bounds
// 21. authority index out of bounds
//
// ============================================================================
// DELEGATE TESTS (separate category)
// ============================================================================
//
// NOTE: Delegates always have the complete account balance delegated to them
// (no partial delegation), so there are no "insufficient allowance" tests.
//
// Transfer with Delegate:
// 1. delegate transfer with valid delegate (should succeed - baseline)
// 2. delegate transfer with invalid delegate signature
// 3. delegate transfer when owner hasn't delegated
// 4. delegate transfer after delegation revoked
//
// Compress with Delegate:
// 5. delegate compress from ctoken account (should succeed)
// 6. delegate compress with invalid delegate
// 7. delegate compress when not delegated
//
// ============================================================================
// COMPRESSIONS-ONLY MODE TESTS (no compressed accounts, Path A)
// ============================================================================
//
// When in_token_data and out_token_data are BOTH empty:
// 1. no compressions provided → NoInputsProvided (6025)
// 2. missing fee payer → CompressionsOnlyMissingFeePayer (6026)
// 3. missing CPI authority PDA → CompressionsOnlyMissingCpiAuthority (6027)
// 4. compressions provided but sum doesn't balance
//
// ============================================================================
// UNIMPLEMENTED FEATURES TESTS
// ============================================================================
//
// These fields must be None - testing they properly reject Some values:
// 1. in_lamports = Some(vec![100]) → TokenDataTlvUnimplemented (18035)
// 2. out_lamports = Some(vec![100]) → TokenDataTlvUnimplemented (18035)
// 3. in_tlv = Some(vec![vec![1,2,3]]) → CompressedTokenAccountTlvUnimplemented (18021)
// 4. out_tlv = Some(vec![vec![1,2,3]]) → CompressedTokenAccountTlvUnimplemented (18021)
//
// ============================================================================
// CPI CONTEXT MODE TESTS
// ============================================================================
//
// Write Mode Restrictions:
// 1. write mode with compressions → InvalidInstructionData (18001)
// 2. write mode with wrong account count → Transfer2CpiContextWriteInvalidAccess (6082)
// 3. write mode with SOL pool → Transfer2CpiContextWriteWithSolPool (6083)
//
// Execute Mode:
// 4. CPI context required but not provided → CpiContextExpected (6085)
//
// ============================================================================
// EDGE CASES
// ============================================================================
//
// Compression Transfer Limits:
// 1. more than 40 unique accounts needing lamport top-up → TooManyCompressionTransfers (6106)
//
// Multi-Mint Validation:
// (TODO: verify if these are already covered by system program)
// 2. 6+ different mints (TooManyMints - 6055)
// 3. input mint indices not in ascending order (InputsOutOfOrder - 6054)
// 4. output mint not present in inputs/compressions (ComputeOutputSumFailed - 6002)
//
// ============================================================================
// TEST SETUP REQUIREMENTS
// ============================================================================
//
// Test setup for Transfer:
// 1. create and mint to one compressed token account (undelegated)
//  - add option to delegate the entire balance
//
// Test setup for Compress ctoken:
// 1. create and mint to one ctoken compressed account
//
// Test setup for Compress spl token:
// 1. create spl token mint and mint to one spl token account

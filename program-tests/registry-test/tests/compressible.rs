#![allow(clippy::result_large_err)]
use std::str::FromStr;

// TODO: refactor into dir
use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use light_compressed_token_sdk::instructions::derive_ctoken_ata;
use light_compressible::{
    config::CompressibleConfig, error::CompressibleError, rent::SLOTS_PER_EPOCH,
};
use light_program_test::{
    forester::claim_forester, program_test::TestRpc, utils::assert::assert_rpc_error,
    LightProgramTest, ProgramTestConfig,
};
use light_registry::accounts::{
    UpdateCompressibleConfig as UpdateCompressibleConfigAccounts,
    WithdrawFundingPool as WithdrawFundingPoolAccounts,
};
use light_test_utils::{
    airdrop_lamports, assert_claim::assert_claim, spl::create_mint_helper, Rpc, RpcError,
};
use light_token_client::actions::{
    create_compressible_token_account, CreateCompressibleTokenAccountInputs,
};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};

/// Withdraw funds from the compressed token pool via the registry program
/// This function invokes the registry program's withdraw_funding_pool instruction,
/// which then CPIs to the compressed token program with the compression_authority PDA as signer.
async fn withdraw_funding_pool_via_registry<R: Rpc>(
    rpc: &mut R,
    withdrawal_authority: &Keypair,
    destination: Pubkey,
    amount: u64,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    // Registry and compressed token program IDs
    let registry_program_id =
        Pubkey::from_str("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX").unwrap();
    let compressed_token_program_id =
        Pubkey::from_str("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m").unwrap();
    let config = CompressibleConfig::ctoken_v1(Default::default(), Default::default());
    let compression_authority = config.compression_authority;
    let rent_sponsor = config.rent_sponsor;
    let compressible_config = CompressibleConfig::ctoken_v1_config_pda();

    // Build accounts using Anchor's account abstraction
    let withdraw_accounts = WithdrawFundingPoolAccounts {
        fee_payer: payer.pubkey(),
        withdrawal_authority: withdrawal_authority.pubkey(),
        compressible_config,
        rent_sponsor,
        compression_authority,
        destination,
        system_program: solana_sdk::system_program::id(),
        compressed_token_program: compressed_token_program_id,
    };

    // Build the instruction
    let instruction = Instruction {
        program_id: registry_program_id,
        accounts: withdraw_accounts.to_account_metas(None),
        data: light_registry::instruction::WithdrawFundingPool { amount }.data(),
    };

    // Send transaction
    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[payer, withdrawal_authority],
        blockhash,
    );

    rpc.process_transaction(transaction).await
}

#[tokio::test]
async fn test_claim_rent_for_completed_epochs() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let _payer_pubkey = payer.pubkey();
    let mint = Pubkey::new_unique();

    let compressible_owner_keypair = Keypair::new();
    let compressible_owner_pubkey = compressible_owner_keypair.pubkey();

    // Create compressible token account with 2 epochs of rent prepaid
    let prepaid_epochs = 2;
    let lamports_per_write = Some(100);

    // Use the new action to create the compressible token account
    let token_account_pubkey = create_compressible_token_account(
        &mut rpc,
        CreateCompressibleTokenAccountInputs {
            owner: compressible_owner_pubkey,
            mint,
            num_prepaid_epochs: prepaid_epochs,
            payer: &payer,
            token_account_keypair: None,
            lamports_per_write,
            token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
        },
    )
    .await
    .unwrap();

    // Warp forward one epoch
    let current_slot = rpc.get_slot().await.unwrap();
    let target_slot = current_slot + SLOTS_PER_EPOCH;
    rpc.warp_to_slot(target_slot).unwrap();

    // Get the forester keypair from test accounts
    let forester_keypair = rpc.test_accounts.protocol.forester.insecure_clone();

    // Use the claim_forester function to claim via registry program
    claim_forester(&mut rpc, &[token_account_pubkey], &forester_keypair, &payer)
        .await
        .unwrap();

    // Verify the claim using the assert function
    // We warped forward 1 epoch, so we expect to claim 1 epoch of rent
    let config = rpc.test_accounts.funding_pool_config;

    assert_claim(
        &mut rpc,
        &[token_account_pubkey],
        config.rent_sponsor_pda,
        config.compression_authority_pda,
    )
    .await;

    Ok(())
}

#[tokio::test]
async fn test_claim_multiple_accounts_different_epochs() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let mint = create_mint_helper(&mut rpc, &payer).await;

    // Create 10 token accounts with varying prepaid epochs (1 to 10)
    let mut token_accounts = Vec::new();
    let mut owners = Vec::new();

    for i in 2..=11 {
        let owner_keypair = Keypair::new();
        let owner_pubkey = owner_keypair.pubkey();
        owners.push(owner_keypair);
        let token_account_pubkey = create_compressible_token_account(
            &mut rpc,
            CreateCompressibleTokenAccountInputs {
                owner: owner_pubkey,
                mint,
                num_prepaid_epochs: i as u8,
                payer: &payer,
                token_account_keypair: None,
                lamports_per_write: Some(100),
                token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            },
        )
        .await
        .unwrap();

        token_accounts.push(token_account_pubkey);

        println!("Created token account {} with {} epochs prepaid", i, i);
    }

    // Store initial lamports for each account
    let mut initial_lamports = Vec::new();
    for account in &token_accounts {
        let account_data = rpc.get_account(*account).await.unwrap().unwrap();
        initial_lamports.push(account_data.lamports);
    }
    // Warp forward 10 epochs using the new wrapper method
    rpc.warp_epoch_forward(11).await.unwrap();

    // assert all token accounts are closed
    for token_account in token_accounts.iter() {
        let account = rpc.get_account(*token_account).await.unwrap();
        if let Some(account) = account {
            assert_eq!(account.lamports, 0);
        }
    }
}

#[tokio::test]
async fn test_withdraw_funding_pool() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // The withdrawal authority is the payer (as configured in the CompressibleConfig)
    let withdrawal_authority = payer.insecure_clone();

    // Get the rent_sponsor PDA from funding pool config
    let rent_sponsor = rpc.test_accounts.funding_pool_config.rent_sponsor_pda;

    // Fund the pool with 5 SOL
    let initial_pool_balance = 5_000_000_000u64;
    airdrop_lamports(&mut rpc, &rent_sponsor, initial_pool_balance)
        .await
        .unwrap();

    // Create a destination account for withdrawal
    let destination_keypair = Keypair::new();
    let destination_pubkey = destination_keypair.pubkey();

    // Fund destination with minimum rent exemption
    airdrop_lamports(&mut rpc, &destination_pubkey, 1_000_000)
        .await
        .unwrap();

    // Get initial balances
    let initial_destination_balance = rpc.get_account(destination_pubkey).await?.unwrap().lamports;
    let pool_balance_before = rpc.get_account(rent_sponsor).await?.unwrap().lamports;

    // Withdraw 1 SOL from pool to destination using registry program
    let withdraw_amount = 1_000_000_000u64;
    withdraw_funding_pool_via_registry(
        &mut rpc,
        &withdrawal_authority,
        destination_pubkey,
        withdraw_amount,
        &payer,
    )
    .await
    .unwrap();

    // Verify balances after withdrawal
    let pool_balance_after = rpc.get_account(rent_sponsor).await?.unwrap().lamports;
    let destination_balance_after = rpc.get_account(destination_pubkey).await?.unwrap().lamports;

    assert_eq!(
        pool_balance_after,
        pool_balance_before - withdraw_amount,
        "Pool balance should decrease by withdrawn amount"
    );

    assert_eq!(
        destination_balance_after,
        initial_destination_balance + withdraw_amount,
        "Destination balance should increase by withdrawn amount"
    );

    // Test: Try to withdraw with wrong authority (should fail)
    let wrong_authority = Keypair::new();
    airdrop_lamports(&mut rpc, &wrong_authority.pubkey(), 1_000_000)
        .await
        .unwrap();
    let result = withdraw_funding_pool_via_registry(
        &mut rpc,
        &wrong_authority,
        destination_pubkey,
        withdraw_amount,
        &payer,
    )
    .await;

    assert!(
        result.is_err(),
        "Should fail when withdrawing with wrong authority"
    );

    // Test: Try to withdraw more than available (should fail)
    let remaining_balance = rpc.get_account(rent_sponsor).await?.unwrap().lamports;
    let excessive_amount = remaining_balance + 1;
    let result = withdraw_funding_pool_via_registry(
        &mut rpc,
        &withdrawal_authority,
        destination_pubkey,
        excessive_amount,
        &payer,
    )
    .await;

    assert!(
        result.is_err(),
        "Should fail when withdrawing more than available balance"
    );

    // Withdraw everything
    withdraw_funding_pool_via_registry(
        &mut rpc,
        &withdrawal_authority,
        destination_pubkey,
        remaining_balance,
        &payer,
    )
    .await
    .unwrap();
    let pool_balance_after = rpc.get_account(rent_sponsor).await.unwrap();
    assert!(pool_balance_after.is_none(), "Pool balance should be 0");

    Ok(())
}

/// Helper function to pause a compressible config
async fn pause_compressible_config<R: Rpc>(
    rpc: &mut R,
    update_authority: &Keypair,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    let registry_program_id =
        Pubkey::from_str("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX").unwrap();
    let compressible_config = CompressibleConfig::ctoken_v1_config_pda();

    let accounts = UpdateCompressibleConfigAccounts {
        fee_payer: payer.pubkey(),
        update_authority: update_authority.pubkey(),
        compressible_config,
        new_update_authority: None,
        new_withdrawal_authority: None,
    };

    let instruction = Instruction {
        program_id: registry_program_id,
        accounts: accounts.to_account_metas(None),
        data: light_registry::instruction::PauseCompressibleConfig {}.data(),
    };

    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[payer, update_authority],
        blockhash,
    );

    rpc.process_transaction(transaction).await
}

/// Helper function to unpause a compressible config
async fn unpause_compressible_config<R: Rpc>(
    rpc: &mut R,
    update_authority: &Keypair,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    let registry_program_id =
        Pubkey::from_str("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX").unwrap();
    let compressible_config = CompressibleConfig::ctoken_v1_config_pda();

    let accounts = UpdateCompressibleConfigAccounts {
        fee_payer: payer.pubkey(),
        update_authority: update_authority.pubkey(),
        compressible_config,
        new_update_authority: None,
        new_withdrawal_authority: None,
    };

    let instruction = Instruction {
        program_id: registry_program_id,
        accounts: accounts.to_account_metas(None),
        data: light_registry::instruction::UnpauseCompressibleConfig {}.data(),
    };

    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[payer, update_authority],
        blockhash,
    );

    rpc.process_transaction(transaction).await
}

/// Helper function to deprecate a compressible config
async fn deprecate_compressible_config<R: Rpc>(
    rpc: &mut R,
    update_authority: &Keypair,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    let registry_program_id =
        Pubkey::from_str("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX").unwrap();
    let compressible_config = CompressibleConfig::ctoken_v1_config_pda();

    let accounts = UpdateCompressibleConfigAccounts {
        fee_payer: payer.pubkey(),
        update_authority: update_authority.pubkey(),
        compressible_config,
        new_update_authority: None,
        new_withdrawal_authority: None,
    };

    let instruction = Instruction {
        program_id: registry_program_id,
        accounts: accounts.to_account_metas(None),
        data: light_registry::instruction::DeprecateCompressibleConfig {}.data(),
    };

    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[payer, update_authority],
        blockhash,
    );

    rpc.process_transaction(transaction).await
}

/// Helper function to update compressible config authorities
async fn update_compressible_config_authorities<R: Rpc>(
    rpc: &mut R,
    update_authority: &Keypair,
    new_update_authority: Option<&Keypair>,
    new_withdrawal_authority: Option<&Keypair>,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    let compressible_config = CompressibleConfig::ctoken_v1_config_pda();

    let accounts = UpdateCompressibleConfigAccounts {
        fee_payer: payer.pubkey(),
        update_authority: update_authority.pubkey(),
        compressible_config,
        new_update_authority: new_update_authority.map(|k| k.pubkey()),
        new_withdrawal_authority: new_withdrawal_authority.map(|k| k.pubkey()),
    };

    let instruction = Instruction {
        program_id: light_registry::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: light_registry::instruction::UpdateCompressibleConfig {}.data(),
    };

    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();

    // Collect signers
    let mut signers: Vec<&Keypair> = vec![payer, update_authority];
    if let Some(new_auth) = new_update_authority {
        signers.push(new_auth);
    }
    if let Some(new_auth) = new_withdrawal_authority {
        signers.push(new_auth);
    }

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &signers,
        blockhash,
    );

    rpc.process_transaction(transaction).await
}

#[tokio::test]
async fn test_pause_compressible_config_with_valid_authority() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Pause the config with valid authority
    pause_compressible_config(&mut rpc, &payer, &payer)
        .await
        .unwrap();

    // Verify the config state is paused (state = 0)
    let compressible_config_pda = CompressibleConfig::ctoken_v1_config_pda();
    let account_data = rpc
        .get_account(compressible_config_pda)
        .await?
        .expect("CompressibleConfig account should exist");

    let config = CompressibleConfig::try_from_slice(&account_data.data[8..])
        .expect("Failed to deserialize CompressibleConfig");

    assert_eq!(config.state, 0, "Config state should be paused (0)");

    // Test 1: Cannot create new token accounts with paused config

    let compressible_instruction = light_compressed_token_sdk::instructions::create_compressible_associated_token_account(
        light_compressed_token_sdk::instructions::CreateCompressibleAssociatedTokenAccountInputs {
            payer: payer.pubkey(),
            owner: payer.pubkey(),
            mint: Pubkey::new_unique(),
            compressible_config: rpc.test_accounts.funding_pool_config.compressible_config_pda,
            rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
            pre_pay_num_epochs: 2,
            lamports_per_write: None,
            token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
        }
    ).map_err(|e| RpcError::AssertRpcError(format!("Failed to create compressible ATA instruction: {}", e))).unwrap();

    let result = rpc
        .create_and_send_transaction(&[compressible_instruction], &payer.pubkey(), &[&payer])
        .await;
    assert_rpc_error(result, 0, CompressibleError::InvalidState(1).into()).unwrap();
    // Check for specific error code if needed

    // Test 2: Cannot withdraw from funding pool with paused config
    let destination = Keypair::new();
    airdrop_lamports(&mut rpc, &destination.pubkey(), 1_000_000)
        .await
        .unwrap();

    // First fund the pool so we have something to withdraw
    let rent_sponsor = rpc.test_accounts.funding_pool_config.rent_sponsor_pda;
    airdrop_lamports(&mut rpc, &rent_sponsor, 1_000_000_000)
        .await
        .unwrap();

    let withdraw_result = withdraw_funding_pool_via_registry(
        &mut rpc,
        &payer, // withdrawal_authority
        destination.pubkey(),
        100_000_000,
        &payer,
    )
    .await;

    assert!(
        withdraw_result.is_err(),
        "Should fail to withdraw with paused config"
    );

    // Test 3: Cannot claim rent with paused config
    let forester_keypair = rpc.test_accounts.protocol.forester.insecure_clone();
    // Try to claim (even though there might not be accounts to claim from, it should fail due to paused state)
    let result = claim_forester(
        &mut rpc,
        &[], // Empty array since we can't create accounts with paused config
        &forester_keypair,
        &payer,
    )
    .await;
    // Note: claim might succeed with empty array, so this check might need adjustment
    // The real check would be when there are actual accounts to claim from
    assert_rpc_error(result, 0, CompressibleError::InvalidState(1).into()).unwrap();

    Ok(())
}

#[tokio::test]
async fn test_pause_compressible_config_with_invalid_authority() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create a wrong authority keypair
    let wrong_authority = Keypair::new();
    airdrop_lamports(&mut rpc, &wrong_authority.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // Try to pause with invalid authority
    let result = pause_compressible_config(&mut rpc, &wrong_authority, &payer).await;

    assert!(
        result.is_err(),
        "Should fail when pausing with invalid authority"
    );

    // Verify the config state is still active (state = 1)
    let compressible_config_pda = CompressibleConfig::ctoken_v1_config_pda();
    let account_data = rpc
        .get_account(compressible_config_pda)
        .await?
        .expect("CompressibleConfig account should exist");

    let config = CompressibleConfig::try_from_slice(&account_data.data[8..])
        .expect("Failed to deserialize CompressibleConfig");

    assert_eq!(config.state, 1, "Config state should still be active (1)");

    Ok(())
}

#[tokio::test]
async fn test_unpause_compressible_config_with_valid_authority() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // First pause the config
    pause_compressible_config(&mut rpc, &payer, &payer)
        .await
        .unwrap();

    // Verify it's paused
    let compressible_config_pda = CompressibleConfig::ctoken_v1_config_pda();
    let account_data = rpc
        .get_account(compressible_config_pda)
        .await?
        .expect("CompressibleConfig account should exist");

    let config = CompressibleConfig::try_from_slice(&account_data.data[8..])
        .expect("Failed to deserialize CompressibleConfig");
    assert_eq!(config.state, 0, "Config should be paused before unpausing");

    // Verify cannot create account while paused
    let compressible_instruction = light_compressed_token_sdk::instructions::create_compressible_associated_token_account(
        light_compressed_token_sdk::instructions::CreateCompressibleAssociatedTokenAccountInputs {
            payer: payer.pubkey(),
            owner: payer.pubkey(),
            mint: Pubkey::new_unique(),
            compressible_config: rpc.test_accounts.funding_pool_config.compressible_config_pda,
            rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
            pre_pay_num_epochs: 2,
            lamports_per_write: None,
            token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
        }
    ).map_err(|e| RpcError::AssertRpcError(format!("Failed to create compressible ATA instruction: {}", e))).unwrap();

    let result = rpc
        .create_and_send_transaction(&[compressible_instruction], &payer.pubkey(), &[&payer])
        .await;
    assert_rpc_error(result, 0, CompressibleError::InvalidState(1).into()).unwrap();

    // Unpause the config with valid authority
    unpause_compressible_config(&mut rpc, &payer, &payer)
        .await
        .unwrap();

    // Verify the config state is active (state = 1)
    let account_data = rpc
        .get_account(compressible_config_pda)
        .await?
        .expect("CompressibleConfig account should exist");

    let config = CompressibleConfig::try_from_slice(&account_data.data[8..])
        .expect("Failed to deserialize CompressibleConfig");

    assert_eq!(config.state, 1, "Config state should be active (1)");

    // Test: CAN create new token accounts after unpausing
    let compressible_instruction = light_compressed_token_sdk::instructions::create_compressible_associated_token_account(
        light_compressed_token_sdk::instructions::CreateCompressibleAssociatedTokenAccountInputs {
            payer: payer.pubkey(),
            owner: payer.pubkey(),
            mint: Pubkey::new_unique(),
            compressible_config: rpc.test_accounts.funding_pool_config.compressible_config_pda,
            rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
            pre_pay_num_epochs: 2,
            lamports_per_write: None,
            token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
        }
    ).map_err(|e| RpcError::AssertRpcError(format!("Failed to create compressible ATA instruction: {}", e))).unwrap();

    let result2 = rpc
        .create_and_send_transaction(&[compressible_instruction], &payer.pubkey(), &[&payer])
        .await;
    assert!(
        result2.is_ok(),
        "Should be able to create account after unpausing"
    );

    Ok(())
}

#[tokio::test]
async fn test_unpause_compressible_config_with_invalid_authority() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // First pause the config with valid authority
    pause_compressible_config(&mut rpc, &payer, &payer)
        .await
        .unwrap();

    // Create a wrong authority keypair
    let wrong_authority = Keypair::new();
    airdrop_lamports(&mut rpc, &wrong_authority.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // Try to unpause with invalid authority
    let result = unpause_compressible_config(&mut rpc, &wrong_authority, &payer).await;

    assert_rpc_error(
        result,
        0,
        anchor_lang::prelude::ErrorCode::ConstraintHasOne.into(),
    )
    .unwrap();

    // Verify the config state is still paused (state = 0)
    let compressible_config_pda = CompressibleConfig::ctoken_v1_config_pda();
    let account_data = rpc
        .get_account(compressible_config_pda)
        .await?
        .expect("CompressibleConfig account should exist");

    let config = CompressibleConfig::try_from_slice(&account_data.data[8..])
        .expect("Failed to deserialize CompressibleConfig");

    assert_eq!(config.state, 0, "Config state should still be paused (0)");

    Ok(())
}

#[tokio::test]
async fn test_deprecate_compressible_config_with_valid_authority() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // First create a compressible account while config is active
    let token_account_keypair = Keypair::new();
    let mint = Pubkey::new_unique();

    let compressible_instruction = light_compressed_token_sdk::instructions::create_compressible_associated_token_account(
        light_compressed_token_sdk::instructions::CreateCompressibleAssociatedTokenAccountInputs {
            payer: payer.pubkey(),
            owner: token_account_keypair.pubkey(),
            mint,
            compressible_config: rpc.test_accounts.funding_pool_config.compressible_config_pda,
            rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
            pre_pay_num_epochs: 10,
            lamports_per_write: None,
            token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
        }
    ).map_err(|e| RpcError::AssertRpcError(format!("Failed to create compressible ATA instruction: {}", e))).unwrap();

    rpc.create_and_send_transaction(&[compressible_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Deprecate the config with valid authority
    deprecate_compressible_config(&mut rpc, &payer, &payer)
        .await
        .unwrap();

    // Verify the config state is deprecated (state = 2)
    let compressible_config_pda = CompressibleConfig::ctoken_v1_config_pda();
    let account_data = rpc
        .get_account(compressible_config_pda)
        .await?
        .expect("CompressibleConfig account should exist");

    let config = CompressibleConfig::try_from_slice(&account_data.data[8..])
        .expect("Failed to deserialize CompressibleConfig");

    assert_eq!(config.state, 2, "Config state should be deprecated (2)");

    // Test 1: Cannot create new token accounts with deprecated config
    let token_account_keypair2 = Keypair::new();
    let compressible_instruction = light_compressed_token_sdk::instructions::create_compressible_associated_token_account(
        light_compressed_token_sdk::instructions::CreateCompressibleAssociatedTokenAccountInputs {
            payer: payer.pubkey(),
            owner: token_account_keypair2.pubkey(),
            mint,
            compressible_config: rpc.test_accounts.funding_pool_config.compressible_config_pda,
            rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
            pre_pay_num_epochs: 2,
            lamports_per_write: None,
            token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
        }
    ).map_err(|e| RpcError::AssertRpcError(format!("Failed to create compressible ATA instruction: {}", e))).unwrap();

    let result = rpc
        .create_and_send_transaction(&[compressible_instruction], &payer.pubkey(), &[&payer])
        .await;
    assert_rpc_error(result, 0, CompressibleError::InvalidState(1).into()).unwrap();

    // Test 2: CAN withdraw from funding pool with deprecated config
    let destination = Keypair::new();
    airdrop_lamports(&mut rpc, &destination.pubkey(), 1_000_000)
        .await
        .unwrap();

    // Fund the pool so we have something to withdraw
    let rent_sponsor = rpc.test_accounts.funding_pool_config.rent_sponsor_pda;
    airdrop_lamports(&mut rpc, &rent_sponsor, 1_000_000_000)
        .await
        .unwrap();

    let withdraw_result = withdraw_funding_pool_via_registry(
        &mut rpc,
        &payer, // withdrawal_authority
        destination.pubkey(),
        100_000_000,
        &payer,
    )
    .await;

    assert!(
        withdraw_result.is_ok(),
        "Should be able to withdraw with deprecated config"
    );

    // Test 3: CAN claim rent with deprecated config

    let forester_keypair = rpc.test_accounts.protocol.forester.insecure_clone();
    let (ata_pubkey, _) = derive_ctoken_ata(&token_account_keypair.pubkey(), &mint);

    // Claim from the account we created earlier
    let claim_result = claim_forester(&mut rpc, &[ata_pubkey], &forester_keypair, &payer).await;

    assert!(
        claim_result.is_ok(),
        "Should be able to claim with deprecated config"
    );

    Ok(())
}

#[tokio::test]
async fn test_deprecate_compressible_config_with_invalid_authority() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create a wrong authority keypair
    let wrong_authority = Keypair::new();
    airdrop_lamports(&mut rpc, &wrong_authority.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // Try to deprecate with invalid authority
    let result = deprecate_compressible_config(&mut rpc, &wrong_authority, &payer).await;

    assert!(
        result.is_err(),
        "Should fail when deprecating with invalid authority"
    );

    // Verify the config state is still active (state = 1)
    let compressible_config_pda = CompressibleConfig::ctoken_v1_config_pda();
    let account_data = rpc
        .get_account(compressible_config_pda)
        .await?
        .expect("CompressibleConfig account should exist");

    let config = CompressibleConfig::try_from_slice(&account_data.data[8..])
        .expect("Failed to deserialize CompressibleConfig");

    assert_eq!(config.state, 1, "Config state should still be active (1)");

    Ok(())
}

#[tokio::test]
async fn test_update_compressible_config_update_authority() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create new update authority
    let new_update_authority = Keypair::new();
    airdrop_lamports(&mut rpc, &new_update_authority.pubkey(), 1_000_000)
        .await
        .unwrap();

    // Update the update_authority
    update_compressible_config_authorities(
        &mut rpc,
        &payer, // current update_authority
        Some(&new_update_authority),
        None,
        &payer,
    )
    .await
    .unwrap();

    // Verify the update_authority was updated
    let compressible_config_pda = CompressibleConfig::ctoken_v1_config_pda();
    let account_data = rpc
        .get_account(compressible_config_pda)
        .await?
        .expect("CompressibleConfig account should exist");

    let config = CompressibleConfig::try_from_slice(&account_data.data[8..])
        .expect("Failed to deserialize CompressibleConfig");

    assert_eq!(
        config.update_authority,
        new_update_authority.pubkey(),
        "Update authority should be updated"
    );

    Ok(())
}

#[tokio::test]
async fn test_update_compressible_config_withdrawal_authority() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Store original withdrawal authority
    let compressible_config_pda = CompressibleConfig::ctoken_v1_config_pda();
    let account_data_before = rpc
        .get_account(compressible_config_pda)
        .await?
        .expect("CompressibleConfig account should exist");
    let config_before = CompressibleConfig::try_from_slice(&account_data_before.data[8..])
        .expect("Failed to deserialize CompressibleConfig");
    let original_withdrawal_authority = config_before.withdrawal_authority;

    // Create new withdrawal authority
    let new_withdrawal_authority = Keypair::new();
    airdrop_lamports(&mut rpc, &new_withdrawal_authority.pubkey(), 1_000_000)
        .await
        .unwrap();

    // Update the withdrawal_authority
    update_compressible_config_authorities(
        &mut rpc,
        &payer, // current update_authority
        None,
        Some(&new_withdrawal_authority),
        &payer,
    )
    .await
    .unwrap();

    // Verify the withdrawal_authority was updated
    let account_data = rpc
        .get_account(compressible_config_pda)
        .await?
        .expect("CompressibleConfig account should exist");

    let config = CompressibleConfig::try_from_slice(&account_data.data[8..])
        .expect("Failed to deserialize CompressibleConfig");

    assert_eq!(
        config.withdrawal_authority,
        new_withdrawal_authority.pubkey(),
        "Withdrawal authority should be updated"
    );
    assert_eq!(
        config.update_authority,
        payer.pubkey(),
        "Update authority should remain unchanged"
    );
    assert_ne!(
        config.withdrawal_authority, original_withdrawal_authority,
        "Withdrawal authority should be different from original"
    );

    Ok(())
}

#[tokio::test]
async fn test_update_compressible_config_both_authorities() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create new authorities
    let new_update_authority = Keypair::new();
    let new_withdrawal_authority = Keypair::new();
    airdrop_lamports(&mut rpc, &new_update_authority.pubkey(), 1_000_000)
        .await
        .unwrap();
    airdrop_lamports(&mut rpc, &new_withdrawal_authority.pubkey(), 1_000_000)
        .await
        .unwrap();

    // Update both authorities
    update_compressible_config_authorities(
        &mut rpc,
        &payer, // current update_authority
        Some(&new_update_authority),
        Some(&new_withdrawal_authority),
        &payer,
    )
    .await
    .unwrap();

    // Verify both authorities were updated
    let compressible_config_pda = CompressibleConfig::ctoken_v1_config_pda();
    let account_data = rpc
        .get_account(compressible_config_pda)
        .await?
        .expect("CompressibleConfig account should exist");

    let config = CompressibleConfig::try_from_slice(&account_data.data[8..])
        .expect("Failed to deserialize CompressibleConfig");

    assert_eq!(
        config.update_authority,
        new_update_authority.pubkey(),
        "Update authority should be updated"
    );
    assert_eq!(
        config.withdrawal_authority,
        new_withdrawal_authority.pubkey(),
        "Withdrawal authority should be updated"
    );

    Ok(())
}

#[tokio::test]
async fn test_update_compressible_config_invalid_authority() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create wrong authority keypair
    let wrong_authority = Keypair::new();
    airdrop_lamports(&mut rpc, &wrong_authority.pubkey(), 1_000_000)
        .await
        .unwrap();

    // Create new update authority
    let new_update_authority = Keypair::new();
    airdrop_lamports(&mut rpc, &new_update_authority.pubkey(), 1_000_000)
        .await
        .unwrap();

    // Try to update with wrong authority
    let result = update_compressible_config_authorities(
        &mut rpc,
        &wrong_authority, // wrong update_authority
        Some(&new_update_authority),
        None,
        &payer,
    )
    .await;

    assert_rpc_error(
        result,
        0,
        anchor_lang::prelude::ErrorCode::ConstraintHasOne.into(),
    )
    .unwrap();

    // Verify the update_authority was NOT updated
    let compressible_config_pda = CompressibleConfig::ctoken_v1_config_pda();
    let account_data = rpc
        .get_account(compressible_config_pda)
        .await?
        .expect("CompressibleConfig account should exist");

    let config = CompressibleConfig::try_from_slice(&account_data.data[8..])
        .expect("Failed to deserialize CompressibleConfig");

    assert_eq!(
        config.update_authority,
        payer.pubkey(),
        "Update authority should remain unchanged"
    );

    Ok(())
}

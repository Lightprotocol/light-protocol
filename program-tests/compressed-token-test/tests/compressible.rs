use light_ctoken_types::state::extensions::compressible::SLOTS_PER_EPOCH;
use light_program_test::{program_test::TestRpc, LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    airdrop_lamports, assert_claim::assert_claim, spl::create_mint_helper, Rpc, RpcError,
};
use light_token_client::actions::create_compressible_token_account;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

#[tokio::test]
async fn test_claim_rent_for_completed_epochs() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();
    let mint = Pubkey::new_unique();

    // Create rent authority and compressible owner
    let rent_authority_keypair = Keypair::new();
    let rent_authority_pubkey = rent_authority_keypair.pubkey();
    let compressible_owner_keypair = Keypair::new();
    let compressible_owner_pubkey = compressible_owner_keypair.pubkey();

    // Airdrop to rent authority to cover fees
    airdrop_lamports(&mut rpc, &rent_authority_pubkey, 1_000_000_000).await?;

    // Derive pool PDA to fund it
    let (pool_pda, pool_pda_bump) =
        light_compressed_token_sdk::instructions::derive_pool_pda(&rent_authority_pubkey);

    // Fund pool PDA to receive rent
    rpc.context
        .airdrop(&pool_pda, 1_000_000_000)
        .map_err(|_| RpcError::AssertRpcError("Failed to airdrop to pool PDA".to_string()))?;

    // Create compressible token account with 2 epochs of rent prepaid
    let prepaid_epochs = 2u64;
    let write_top_up_lamports = Some(100);

    // Use the new action to create the compressible token account
    let token_account_pubkey = create_compressible_token_account(
        &mut rpc,
        rent_authority_pubkey,
        compressible_owner_pubkey,
        mint,
        prepaid_epochs,
        &payer,
        None,
        write_top_up_lamports,
    )
    .await?;

    // Warp forward one epoch
    let current_slot = rpc.get_slot().await?;
    let target_slot = current_slot + SLOTS_PER_EPOCH;
    rpc.warp_to_slot(target_slot)?;

    // Build claim instruction using the SDK
    let claim_instruction = light_compressed_token_sdk::instructions::claim(
        pool_pda,
        pool_pda_bump,
        rent_authority_pubkey,
        &[token_account_pubkey],
    );

    // Execute claim transaction
    rpc.create_and_send_transaction(
        &[claim_instruction],
        &payer_pubkey,
        &[&payer, &rent_authority_keypair],
    )
    .await?;

    // Verify the claim using the assert function
    // We warped forward 1 epoch, so we expect to claim 1 epoch of rent
    assert_claim(
        &mut rpc,
        &[token_account_pubkey],
        pool_pda,
        rent_authority_pubkey,
    )
    .await;

    Ok(())
}

#[tokio::test]
async fn test_claim_multiple_accounts_different_epochs() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();
    let mint = create_mint_helper(&mut rpc, &payer).await;

    // Create rent authority
    let rent_authority_keypair = Keypair::new();
    let rent_authority_pubkey = rent_authority_keypair.pubkey();

    // Airdrop to rent authority
    airdrop_lamports(&mut rpc, &rent_authority_pubkey, 10_000_000_000).await?;

    // Derive pool PDA and fund it
    let (pool_pda, pool_pda_bump) =
        light_compressed_token_sdk::instructions::derive_pool_pda(&rent_authority_pubkey);

    rpc.context
        .airdrop(&pool_pda, 10_000_000_000)
        .map_err(|_| RpcError::AssertRpcError("Failed to airdrop to pool PDA".to_string()))?;

    // Create 10 token accounts with varying prepaid epochs (1 to 10)
    let mut token_accounts = Vec::new();
    let mut owners = Vec::new();

    for i in 1..=10 {
        let owner_keypair = Keypair::new();
        let owner_pubkey = owner_keypair.pubkey();
        owners.push(owner_keypair);

        // Create token account with i epochs prepaid
        let token_account_pubkey = create_compressible_token_account(
            &mut rpc,
            rent_authority_pubkey,
            owner_pubkey,
            mint,
            i as u64, // Prepay i epochs
            &payer,
            None,
            Some(100),
        )
        .await?;

        token_accounts.push(token_account_pubkey);

        println!("Created token account {} with {} epochs prepaid", i, i);
    }

    // Store initial lamports for each account
    let mut initial_lamports = Vec::new();
    for account in &token_accounts {
        let account_data = rpc.get_account(*account).await?.unwrap();
        initial_lamports.push(account_data.lamports);
    }

    // Warp forward different amounts for different claim scenarios
    // Account 1: prepaid 1 epoch, warp 0.5 epochs -> can't claim
    // Account 2: prepaid 2 epochs, warp 1 epoch -> can claim 1
    // Account 3: prepaid 3 epochs, warp 1.5 epochs -> can claim 1
    // Account 4: prepaid 4 epochs, warp 2 epochs -> can claim 2
    // Account 5: prepaid 5 epochs, warp 2.5 epochs -> can claim 2
    // Account 6: prepaid 6 epochs, warp 3 epochs -> can claim 3
    // Account 7: prepaid 7 epochs, warp 3.5 epochs -> can claim 3
    // Account 8: prepaid 8 epochs, warp 4 epochs -> can claim 4
    // Account 9: prepaid 9 epochs, warp 4.5 epochs -> can claim 4
    // Account 10: prepaid 10 epochs, warp 5 epochs -> can claim 5

    // Warp forward 2 epochs to allow some accounts to claim
    let current_slot = rpc.get_slot().await?;
    let target_slot = current_slot + (SLOTS_PER_EPOCH * 2);
    rpc.warp_to_slot(target_slot)?;

    // Build claim instruction for all 10 accounts
    let claim_instruction = light_compressed_token_sdk::instructions::claim(
        pool_pda,
        pool_pda_bump,
        rent_authority_pubkey,
        &token_accounts,
    );

    // Execute claim transaction
    rpc.create_and_send_transaction(
        &[claim_instruction],
        &payer_pubkey,
        &[&payer, &rent_authority_keypair],
    )
    .await?;
    assert_claim(&mut rpc, &token_accounts, pool_pda, rent_authority_pubkey).await;
    Ok(())
}

#[tokio::test]
async fn test_withdraw_funding_pool() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Create rent authority
    let rent_authority_keypair = Keypair::new();
    let rent_authority_pubkey = rent_authority_keypair.pubkey();

    // Airdrop to rent authority
    airdrop_lamports(&mut rpc, &rent_authority_pubkey, 10_000_000_000).await?;

    // Derive pool PDA and fund it
    let (pool_pda, pool_pda_bump) =
        light_compressed_token_sdk::instructions::derive_pool_pda(&rent_authority_pubkey);

    // Fund pool PDA with 5 SOL
    let initial_pool_balance = 5_000_000_000u64;
    rpc.context
        .airdrop(&pool_pda, initial_pool_balance)
        .map_err(|_| RpcError::AssertRpcError("Failed to airdrop to pool PDA".to_string()))?;

    // Create a destination account for withdrawal
    let destination_keypair = Keypair::new();
    let destination_pubkey = destination_keypair.pubkey();

    // Fund destination with minimum rent exemption
    airdrop_lamports(&mut rpc, &destination_pubkey, 1_000_000).await?;

    // Get initial balances
    let initial_destination_balance = rpc.get_account(destination_pubkey).await?.unwrap().lamports;
    let pool_balance_before = rpc.get_account(pool_pda).await?.unwrap().lamports;

    // Withdraw 1 SOL from pool to destination
    let withdraw_amount = 1_000_000_000u64;
    let withdraw_instruction = light_compressed_token_sdk::instructions::withdraw_funding_pool(
        pool_pda,
        pool_pda_bump,
        rent_authority_pubkey,
        destination_pubkey,
        withdraw_amount,
    );

    // Execute withdrawal
    rpc.create_and_send_transaction(
        &[withdraw_instruction],
        &payer_pubkey,
        &[&payer, &rent_authority_keypair],
    )
    .await?;

    // Verify balances after withdrawal
    let pool_balance_after = rpc.get_account(pool_pda).await?.unwrap().lamports;
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
    airdrop_lamports(&mut rpc, &wrong_authority.pubkey(), 1_000_000).await?;

    let wrong_withdraw_instruction =
        light_compressed_token_sdk::instructions::withdraw_funding_pool(
            pool_pda,
            pool_pda_bump,
            wrong_authority.pubkey(),
            destination_pubkey,
            100_000_000,
        );

    let result = rpc
        .create_and_send_transaction(
            &[wrong_withdraw_instruction],
            &payer_pubkey,
            &[&payer, &wrong_authority],
        )
        .await;

    assert!(
        result.is_err(),
        "Should fail when withdrawing with wrong authority"
    );

    // Test: Try to withdraw more than available (should fail)
    let remaining_balance = rpc.get_account(pool_pda).await?.unwrap().lamports;
    let excessive_amount = remaining_balance + 1_000_000;

    let excessive_withdraw_instruction =
        light_compressed_token_sdk::instructions::withdraw_funding_pool(
            pool_pda,
            pool_pda_bump,
            rent_authority_pubkey,
            destination_pubkey,
            excessive_amount,
        );

    let result = rpc
        .create_and_send_transaction(
            &[excessive_withdraw_instruction],
            &payer_pubkey,
            &[&payer, &rent_authority_keypair],
        )
        .await;

    assert!(
        result.is_err(),
        "Should fail when withdrawing more than available balance"
    );

    Ok(())
}

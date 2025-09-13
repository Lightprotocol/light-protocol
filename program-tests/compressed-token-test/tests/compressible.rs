use light_compressible::rent::SLOTS_PER_EPOCH;
use light_program_test::{
    forester::claim_forester, program_test::TestRpc, LightProgramTest, ProgramTestConfig,
};
use light_test_utils::{
    airdrop_lamports, assert_claim::assert_claim, spl::create_mint_helper, Rpc, RpcError,
};
use light_token_client::actions::{
    create_compressible_token_account, CreateCompressibleTokenAccountInputs,
};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

#[tokio::test]
async fn test_claim_rent_for_completed_epochs() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();
    let mint = Pubkey::new_unique();

    let compressible_owner_keypair = Keypair::new();
    let compressible_owner_pubkey = compressible_owner_keypair.pubkey();

    // Create compressible token account with 2 epochs of rent prepaid
    let prepaid_epochs = 2u64;
    let write_top_up_lamports = Some(100);

    // Use the new action to create the compressible token account
    let token_account_pubkey = create_compressible_token_account(
        &mut rpc,
        CreateCompressibleTokenAccountInputs {
            owner: compressible_owner_pubkey,
            mint,
            num_prepaid_epochs: prepaid_epochs,
            payer: &payer,
            token_account_keypair: None,
            write_top_up_lamports,
        },
    )
    .await?;

    // Warp forward one epoch
    let current_slot = rpc.get_slot().await?;
    let target_slot = current_slot + SLOTS_PER_EPOCH;
    rpc.warp_to_slot(target_slot)?;

    // Get the forester keypair from test accounts
    let forester_keypair = rpc.test_accounts.protocol.forester.insecure_clone();

    // Use the claim_forester function to claim via registry program
    claim_forester(&mut rpc, &[token_account_pubkey], &forester_keypair, &payer).await?;

    // Verify the claim using the assert function
    // We warped forward 1 epoch, so we expect to claim 1 epoch of rent
    let config = rpc.test_accounts.funding_pool_config;

    assert_claim(
        &mut rpc,
        &[token_account_pubkey],
        config.rent_recipient_pda,
        config.rent_authority_pda,
    )
    .await;

    Ok(())
}

#[tokio::test]
async fn test_claim_multiple_accounts_different_epochs() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();
    let mint = create_mint_helper(&mut rpc, &payer).await;

    // Create 10 token accounts with varying prepaid epochs (1 to 10)
    let mut token_accounts = Vec::new();
    let mut owners = Vec::new();

    for i in 1..=10 {
        let owner_keypair = Keypair::new();
        let owner_pubkey = owner_keypair.pubkey();
        owners.push(owner_keypair);
        let token_account_pubkey = create_compressible_token_account(
            &mut rpc,
            CreateCompressibleTokenAccountInputs {
                owner: owner_pubkey,
                mint,
                num_prepaid_epochs: i as u64,
                payer: &payer,
                token_account_keypair: None,
                write_top_up_lamports: Some(100),
            },
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
    // Warp forward 10 epochs using the new wrapper method
    rpc.warp_epoch_forward(10).await.unwrap();

    // assert all token accounts are closed
    for token_account in token_accounts.iter() {
        let account = rpc.get_account(*token_account).await.unwrap();
        if let Some(account) = account {
            assert_eq!(account.lamports, 0);
        }
    }
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

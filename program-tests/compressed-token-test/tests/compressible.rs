use std::str::FromStr;

use anchor_lang::{InstructionData, ToAccountMetas};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_program_test::{
    forester::claim_forester, program_test::TestRpc, LightProgramTest, ProgramTestConfig,
};
use light_registry::accounts::WithdrawFundingPool as WithdrawFundingPoolAccounts;
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
/// which then CPIs to the compressed token program with the rent_authority PDA as signer.
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

    // Derive CompressibleConfig PDA (version 1)
    let version: u64 = 1;
    let (compressible_config, _) = Pubkey::find_program_address(
        &[b"compressible_config", &version.to_le_bytes()],
        &registry_program_id,
    );

    // Derive rent_authority PDA (uses u16 version)
    let (rent_authority, _) = Pubkey::find_program_address(
        &[
            b"rent_authority".as_slice(),
            (version as u16).to_le_bytes().as_slice(),
            &[0],
        ],
        &registry_program_id,
    );

    // Derive rent_recipient PDA from the compressed token program
    let (rent_recipient, _) = Pubkey::find_program_address(
        &[
            b"rent_recipient".as_slice(),
            (version as u16).to_le_bytes().as_slice(),
            &[0],
        ],
        &compressed_token_program_id,
    );

    // Build accounts using Anchor's account abstraction
    let withdraw_accounts = WithdrawFundingPoolAccounts {
        fee_payer: payer.pubkey(),
        withdrawal_authority: withdrawal_authority.pubkey(),
        compressible_config,
        rent_recipient,
        rent_authority,
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
    let (blockhash, _) = rpc.get_latest_blockhash().await?;
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
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();
    let _payer_pubkey = payer.pubkey();
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

    // The withdrawal authority is the payer (as configured in the CompressibleConfig)
    let withdrawal_authority = payer.insecure_clone();

    // Get the rent_recipient PDA from funding pool config
    let rent_recipient = rpc.test_accounts.funding_pool_config.rent_recipient_pda;

    // Fund the pool with 5 SOL
    let initial_pool_balance = 5_000_000_000u64;
    airdrop_lamports(&mut rpc, &rent_recipient, initial_pool_balance).await?;

    // Create a destination account for withdrawal
    let destination_keypair = Keypair::new();
    let destination_pubkey = destination_keypair.pubkey();

    // Fund destination with minimum rent exemption
    airdrop_lamports(&mut rpc, &destination_pubkey, 1_000_000).await?;

    // Get initial balances
    let initial_destination_balance = rpc.get_account(destination_pubkey).await?.unwrap().lamports;
    let pool_balance_before = rpc.get_account(rent_recipient).await?.unwrap().lamports;

    // Withdraw 1 SOL from pool to destination using registry program
    let withdraw_amount = 1_000_000_000u64;
    withdraw_funding_pool_via_registry(
        &mut rpc,
        &withdrawal_authority,
        destination_pubkey,
        withdraw_amount,
        &payer,
    )
    .await?;

    // Verify balances after withdrawal
    let pool_balance_after = rpc.get_account(rent_recipient).await?.unwrap().lamports;
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
    let remaining_balance = rpc.get_account(rent_recipient).await?.unwrap().lamports;
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
    .await?;
    let pool_balance_after = rpc.get_account(rent_recipient).await?.unwrap().lamports;
    assert_eq!(pool_balance_after, 0, "Pool balance should be 0");

    Ok(())
}

use std::collections::HashMap;

use anchor_lang::AnchorDeserialize;
use light_compressed_token_sdk::{
    account2::CTokenAccount2,
    instructions::transfer2::{
        account_metas::Transfer2AccountsMetaConfig, create_transfer2_instruction, Transfer2Config,
        Transfer2Inputs,
    },
};
use light_ctoken_types::state::{
    extensions::compressible::SLOTS_PER_EPOCH, CompressedToken, ExtensionStruct,
};
use light_program_test::{program_test::TestRpc, LightProgramTest, ProgramTestConfig};
use light_sdk::instruction::PackedAccounts;
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

    let mut stored_compressible_accounts = HashMap::<Pubkey, StoredCompressibleAccount>::new();

    claim_and_compress(
        &mut rpc,
        &mut stored_compressible_accounts,
        pool_pda,
        pool_pda_bump,
        rent_authority_pubkey,
        &rent_authority_keypair,
        &payer,
    )
    .await?;
    let target_slot = current_slot + (SLOTS_PER_EPOCH * 2);
    rpc.warp_to_slot(target_slot)?;
    claim_and_compress(
        &mut rpc,
        &mut stored_compressible_accounts,
        pool_pda,
        pool_pda_bump,
        rent_authority_pubkey,
        &rent_authority_keypair,
        &payer,
    )
    .await?;
    Ok(())
}

#[derive(Eq, Hash, PartialEq)]
pub struct StoredCompressibleAccount {
    pub pubkey: Pubkey,
    pub last_paid_slot: u64,
    pub account: CompressedToken,
}

/// Manual implementation of compress_and_close for a slice of token accounts
/// Uses the already deserialized account data from stored_compressible_accounts
async fn compress_and_close_batch(
    rpc: &mut LightProgramTest,
    compressible_token_accounts: &[&StoredCompressibleAccount],
    rent_authority_keypair: &Keypair,
    payer: &Keypair,
) -> Result<(), RpcError> {
    if compressible_token_accounts.is_empty() {
        return Ok(());
    }

    // Get output queue for compression
    let output_queue = rpc.get_random_state_tree_info()?.get_output_pubkey()?;

    // Build packed accounts and token accounts for all accounts to compress
    let mut packed_accounts = PackedAccounts::default();
    let mut token_accounts = Vec::with_capacity(compressible_token_accounts.len());

    // Add output queue first
    let output_queue_index = packed_accounts.insert_or_get(output_queue);

    for StoredCompressibleAccount {
        pubkey,
        last_paid_slot: _,
        account,
    } in compressible_token_accounts
    {
        let compressed_token = account;

        // Extract necessary data (convert from light_compressed_account::Pubkey to solana_sdk::Pubkey)
        let mint = Pubkey::from(compressed_token.mint.to_bytes());
        let owner = Pubkey::from(compressed_token.owner.to_bytes());
        let amount = compressed_token.amount;

        // Get rent authority and recipient from compressible extension
        let (rent_authority, rent_recipient) =
            if let Some(extensions) = &compressed_token.extensions {
                let mut authority = owner;
                let mut recipient = owner;

                for extension in extensions {
                    if let ExtensionStruct::Compressible(ext) = extension {
                        if let Some(auth) = ext.rent_authority {
                            authority = Pubkey::from(auth);
                        }
                        if let Some(recip) = ext.rent_recipient {
                            recipient = Pubkey::from(recip);
                        }
                        break;
                    }
                }
                (authority, recipient)
            } else {
                (owner, owner)
            };

        // Verify the rent authority matches the expected signer
        if rent_authority != rent_authority_keypair.pubkey() {
            return Err(RpcError::CustomError(format!(
                "Rent authority mismatch for account {}: expected {}, got {}",
                pubkey,
                rent_authority_keypair.pubkey(),
                rent_authority
            )));
        }

        // Add accounts to packed_accounts
        let source_index = packed_accounts.insert_or_get(*pubkey);
        let mint_index = packed_accounts.insert_or_get(mint);
        let owner_index = packed_accounts.insert_or_get(owner);
        let authority_index = packed_accounts.insert_or_get_config(rent_authority, true, false);
        let rent_recipient_index = packed_accounts.insert_or_get(rent_recipient);

        // Create CTokenAccount2 for CompressAndClose operation
        let mut token_account =
            CTokenAccount2::new_empty(owner_index, mint_index, output_queue_index);

        // Set up compress_and_close
        token_account
            .compress_and_close(
                amount,
                source_index,
                authority_index,
                rent_recipient_index,
                token_accounts.len() as u8, // Index in the output array
            )
            .map_err(|e| {
                RpcError::CustomError(format!("Failed to setup compress_and_close: {:?}", e))
            })?;

        token_accounts.push(token_account);
    }

    // Create the transfer2 instruction
    let meta_config =
        Transfer2AccountsMetaConfig::new(payer.pubkey(), packed_accounts.to_account_metas().0);

    let transfer_config = Transfer2Config::default();

    let inputs = Transfer2Inputs {
        meta_config,
        token_accounts,
        transfer_config,
        ..Default::default()
    };

    let instruction = create_transfer2_instruction(inputs)
        .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {:?}", e)))?;

    // Prepare signers
    let mut signers = vec![payer];
    if rent_authority_keypair.pubkey() != payer.pubkey() {
        signers.push(rent_authority_keypair);
    }

    // Send transaction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &signers)
        .await?;

    Ok(())
}

async fn claim_and_compress(
    rpc: &mut LightProgramTest,
    stored_compressible_accounts: &mut HashMap<Pubkey, StoredCompressibleAccount>,
    pool_pda: Pubkey,
    pool_pda_bump: u8,
    rent_authority_pubkey: Pubkey,
    rent_authority_keypair: &Keypair,
    payer: &Keypair,
) -> Result<(), RpcError> {
    let payer_pubkey = payer.pubkey();
    let compressible_ctoken_accounts = rpc
        .context
        .get_program_accounts(&light_compressed_token::ID);

    for account in compressible_ctoken_accounts
        .iter()
        .filter(|e| e.1.data.len() > 200 && e.1.lamports > 0)
    {
        let des_account = CompressedToken::deserialize(&mut account.1.data.as_slice()).unwrap();
        if let Some(extensions) = des_account.extensions.as_ref() {
            for extension in extensions.iter() {
                if let ExtensionStruct::Compressible(e) = extension {
                    let last_paid_epoch =
                        e.get_last_paid_epoch(account.1.data.len() as u64, account.1.lamports);
                    let last_paid_slot = last_paid_epoch * SLOTS_PER_EPOCH;
                    stored_compressible_accounts.insert(
                        account.0,
                        StoredCompressibleAccount {
                            pubkey: account.0,
                            last_paid_slot,
                            account: des_account.clone(),
                        },
                    );
                }
            }
        }
    }

    let current_slot = rpc.get_slot().await.unwrap();
    let compressible_accounts = {
        stored_compressible_accounts
            .iter()
            .filter(|a| a.1.last_paid_slot < current_slot)
            .map(|e| e.1)
            .collect::<Vec<_>>()
    };

    let claim_able_accounts = stored_compressible_accounts
        .iter()
        .filter(|a| a.1.last_paid_slot >= current_slot)
        .map(|e| *e.0)
        .collect::<Vec<_>>();

    for token_accounts in claim_able_accounts.as_slice().chunks(20) {
        // Build claim instruction for all 10 accounts
        let claim_instruction = light_compressed_token_sdk::instructions::claim(
            pool_pda,
            pool_pda_bump,
            rent_authority_pubkey,
            token_accounts,
        );

        // Execute claim transaction
        rpc.create_and_send_transaction(
            &[claim_instruction],
            &payer_pubkey,
            &[payer, rent_authority_keypair],
        )
        .await?;
        assert_claim(rpc, token_accounts, pool_pda, rent_authority_pubkey).await;
    }

    // Process compressible accounts in batches
    const BATCH_SIZE: usize = 10; // Process up to 10 accounts at a time
    let mut pubkeys = Vec::new();
    for chunk in compressible_accounts.chunks(BATCH_SIZE) {
        // Call manual implementation for batch processing
        compress_and_close_batch(rpc, chunk, rent_authority_keypair, payer).await?;

        // Remove processed accounts from the HashMap
        for account_pubkey in chunk {
            pubkeys.push(account_pubkey.pubkey);
        }
    }
    for pubkey in pubkeys {
        stored_compressible_accounts.remove(&pubkey);
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

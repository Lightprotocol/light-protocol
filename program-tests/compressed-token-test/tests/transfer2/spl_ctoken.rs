// Re-export all necessary imports for test modules
pub use anchor_spl::token_2022::spl_token_2022;
pub use light_compressed_token_sdk::instructions::create_associated_token_account::derive_ctoken_ata;
pub use light_program_test::{LightProgramTest, ProgramTestConfig};
pub use light_test_utils::{
    airdrop_lamports,
    spl::{create_mint_helper, create_token_2022_account, mint_spl_tokens},
    Rpc, RpcError,
};
pub use light_token_client::actions::transfer2::{self};
pub use solana_sdk::{signature::Keypair, signer::Signer};
pub use spl_token_2022::pod::PodAccount;

#[tokio::test]
async fn test_spl_to_ctoken_transfer() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(true, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let sender = Keypair::new();
    airdrop_lamports(&mut rpc, &sender.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let mint = create_mint_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    let transfer_amount = 5000u64;

    // Create SPL token account and mint tokens
    let spl_token_account_keypair = Keypair::new();
    create_token_2022_account(&mut rpc, &mint, &spl_token_account_keypair, &sender, false)
        .await
        .unwrap();
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &spl_token_account_keypair.pubkey(),
        &payer.pubkey(),
        &payer,
        amount,
        false,
    )
    .await
    .unwrap();
    println!(
        "spl_token_account_keypair {:?}",
        spl_token_account_keypair.pubkey()
    );
    // Create recipient for compressed tokens
    let recipient = Keypair::new();
    airdrop_lamports(&mut rpc, &recipient.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // Create compressed token ATA for recipient
    let instruction = light_compressed_token_sdk::instructions::create_associated_token_account(
        payer.pubkey(),
        recipient.pubkey(),
        mint,
    )
    .map_err(|e| RpcError::AssertRpcError(format!("Failed to create ATA instruction: {}", e)))
    .unwrap();
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    let associated_token_account = derive_ctoken_ata(&recipient.pubkey(), &mint).0;

    // Get initial SPL token balance
    let spl_account_data = rpc
        .get_account(spl_token_account_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let spl_account = spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to parse SPL token account: {}", e)))
        .unwrap();
    let initial_spl_balance: u64 = spl_account.amount.into();
    assert_eq!(initial_spl_balance, amount);

    // Use the new spl_to_ctoken_transfer action from light-token-client
    transfer2::spl_to_ctoken_transfer(
        &mut rpc,
        spl_token_account_keypair.pubkey(),
        associated_token_account,
        transfer_amount,
        &sender,
        &payer,
    )
    .await
    .unwrap();

    {
        // Verify SPL token balance decreased
        let spl_account_data = rpc
            .get_account(spl_token_account_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();
        let spl_account = spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data)
            .map_err(|e| {
                RpcError::AssertRpcError(format!("Failed to parse SPL token account: {}", e))
            })
            .unwrap();
        let final_spl_balance: u64 = spl_account.amount.into();
        assert_eq!(final_spl_balance, amount - transfer_amount);
    }
    {
        // Verify compressed token balance increased
        let spl_account_data = rpc
            .get_account(associated_token_account)
            .await
            .unwrap()
            .unwrap();
        let spl_account =
            spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data[..165])
                .map_err(|e| {
                    RpcError::AssertRpcError(format!("Failed to parse SPL token account: {}", e))
                })
                .unwrap();
        assert_eq!(
            u64::from(spl_account.amount),
            transfer_amount,
            "Recipient should have {} compressed tokens",
            transfer_amount
        );
    }

    // Now transfer back from compressed token to SPL token account
    println!("Testing reverse transfer: ctoken to SPL");

    // Transfer from recipient's compressed token account back to sender's SPL token account
    transfer2::ctoken_to_spl_transfer(
        &mut rpc,
        associated_token_account,
        spl_token_account_keypair.pubkey(),
        transfer_amount,
        &recipient,
        mint,
        &payer,
    )
    .await
    .unwrap();

    // Verify final balances
    {
        // Verify SPL token balance is restored
        let spl_account_data = rpc
            .get_account(spl_token_account_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();
        let spl_account = spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data)
            .map_err(|e| {
                RpcError::AssertRpcError(format!("Failed to parse SPL token account: {}", e))
            })
            .unwrap();
        let restored_spl_balance: u64 = spl_account.amount.into();
        assert_eq!(
            restored_spl_balance, amount,
            "SPL token balance should be restored to original amount"
        );
    }

    {
        // Verify compressed token balance is now 0
        let ctoken_account_data = rpc
            .get_account(associated_token_account)
            .await
            .unwrap()
            .unwrap();
        let ctoken_account =
            spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&ctoken_account_data.data[..165])
                .map_err(|e| {
                    RpcError::AssertRpcError(format!(
                        "Failed to parse compressed token account: {}",
                        e
                    ))
                })
                .unwrap();
        assert_eq!(
            u64::from(ctoken_account.amount),
            0,
            "Compressed token account should be empty after transfer back"
        );
    }

    println!("Successfully completed round-trip transfer: SPL -> CToken -> SPL");
}

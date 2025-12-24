// Token-2022 with restricted extensions to cToken scenario test
//
// This test demonstrates the complete flow with Token-2022 restricted extensions:
// 1. Create Token-2022 mint with restricted extensions (PermanentDelegate, Pausable, etc.)
// 2. Create token pool (SPL interface PDA) using SDK instruction
// 3. Create Token-2022 token account
// 4. Mint Token-2022 tokens
// 5. Create cToken ATA with compression_only: true (required for restricted extensions)
// 6. Transfer Token-2022 tokens to cToken account
// 7. Advance epochs to trigger compression
// 8. Verify cToken account is compressed and closed (with TLV data)
// 9. Recreate cToken ATA with compression_only: true
// 10. Decompress compressed tokens back to cToken account
// 11. Verify cToken account has tokens again

use light_client::{indexer::Indexer, rpc::Rpc};
use light_ctoken_sdk::{
    ctoken::{
        derive_ctoken_ata, CompressibleParams, CreateAssociatedCTokenAccount, DecompressToCtoken,
        TransferSplToCtoken,
    },
    spl_interface::find_spl_interface_pda_with_index,
};
use light_program_test::{program_test::TestRpc, LightProgramTest, ProgramTestConfig};
use light_test_utils::mint_2022::{
    create_mint_22_with_extensions, create_token_22_account, mint_spl_tokens_22,
};
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_token_2022::pod::PodAccount;

/// Test the complete Token-2022 (restricted extensions) to cToken flow
#[tokio::test]
async fn test_t22_restricted_to_ctoken_scenario() {
    // 1. Setup test environment
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // Create a token owner
    let token_owner = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &token_owner.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // 2. Create Token-2022 mint with restricted extensions
    let decimals = 2u8;
    let (mint_keypair, _extension_config) =
        create_mint_22_with_extensions(&mut rpc, &payer, decimals).await;
    let mint = mint_keypair.pubkey();

    // Note: create_mint_22_with_extensions already creates the token pool

    let mint_amount = 10_000u64;
    let transfer_amount = 5_000u64;

    // 4. Create Token-2022 token account
    let t22_token_account =
        create_token_22_account(&mut rpc, &payer, &mint, &token_owner.pubkey()).await;

    // 5. Mint Token-2022 tokens to the account
    mint_spl_tokens_22(&mut rpc, &payer, &mint, &t22_token_account, mint_amount).await;

    // Verify T22 account has tokens
    let t22_account_data = rpc.get_account(t22_token_account).await.unwrap().unwrap();
    let t22_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&t22_account_data.data[..165]).unwrap();
    let initial_t22_balance: u64 = t22_account.amount.into();
    assert_eq!(initial_t22_balance, mint_amount);

    // 6. Create cToken ATA for the recipient with compression_only: true (required for restricted extensions)
    let ctoken_recipient = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &ctoken_recipient.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    let (ctoken_ata, _bump) = derive_ctoken_ata(&ctoken_recipient.pubkey(), &mint);
    let compressible_params = CompressibleParams {
        compression_only: true,
        ..Default::default()
    };
    let create_ata_instruction =
        CreateAssociatedCTokenAccount::new(payer.pubkey(), ctoken_recipient.pubkey(), mint)
            .with_compressible(compressible_params)
            .instruction()
            .unwrap();

    rpc.create_and_send_transaction(&[create_ata_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify cToken ATA was created
    let ctoken_account_data = rpc.get_account(ctoken_ata).await.unwrap().unwrap();
    assert!(
        !ctoken_account_data.data.is_empty(),
        "cToken ATA should exist"
    );

    // 7. Transfer Token-2022 tokens to cToken account (use restricted=true for mints with restricted extensions)
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint, 0, true);

    let transfer_instruction = TransferSplToCtoken {
        amount: transfer_amount,
        spl_interface_pda_bump,
        decimals,
        source_spl_token_account: t22_token_account,
        destination_ctoken_account: ctoken_ata,
        authority: token_owner.pubkey(),
        mint,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: spl_token_2022::ID,
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(
        &[transfer_instruction],
        &payer.pubkey(),
        &[&payer, &token_owner],
    )
    .await
    .unwrap();

    // 7. Verify results
    // Check T22 account balance decreased
    let t22_account_data = rpc.get_account(t22_token_account).await.unwrap().unwrap();
    let t22_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&t22_account_data.data[..165]).unwrap();
    let final_t22_balance: u64 = t22_account.amount.into();
    assert_eq!(
        final_t22_balance,
        mint_amount - transfer_amount,
        "T22 account balance should have decreased by transfer amount"
    );

    // Check cToken account balance increased
    let ctoken_account_data = rpc.get_account(ctoken_ata).await.unwrap().unwrap();
    let ctoken_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&ctoken_account_data.data[..165]).unwrap();
    let ctoken_balance: u64 = ctoken_account.amount.into();
    assert_eq!(
        ctoken_balance, transfer_amount,
        "cToken account should have received the transferred tokens"
    );

    println!("Token-2022 to cToken transfer completed!");
    println!("  - Created T22 mint with restricted extensions: {}", mint);
    println!("  - Created T22 token account: {}", t22_token_account);
    println!("  - Minted {} tokens to T22 account", mint_amount);
    println!(
        "  - Created cToken ATA (compression_only: true): {}",
        ctoken_ata
    );
    println!(
        "  - Transferred {} tokens from T22 to cToken",
        transfer_amount
    );
    println!(
        "  - Final T22 balance: {}, cToken balance: {}",
        final_t22_balance, ctoken_balance
    );

    // 8. Advance 25 epochs to trigger compression (default prepaid is 16 epochs)
    println!("\nAdvancing 25 epochs to trigger compression...");
    rpc.warp_epoch_forward(25).await.unwrap();

    // 9. Verify cToken account is compressed and closed
    let closed_account = rpc.get_account(ctoken_ata).await.unwrap();
    match closed_account {
        Some(account) => {
            assert_eq!(
                account.lamports, 0,
                "cToken account should be closed (0 lamports)"
            );
        }
        None => {
            println!("  - cToken account no longer exists (closed)");
        }
    }

    // Verify compressed token account exists
    let compressed_accounts = rpc
        .get_compressed_token_accounts_by_owner(&ctoken_recipient.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert!(
        !compressed_accounts.is_empty(),
        "Compressed token account should exist after compression"
    );

    let compressed_account = &compressed_accounts[0];
    assert_eq!(
        compressed_account.token.owner,
        ctoken_recipient.pubkey(),
        "Compressed account owner should match"
    );
    assert_eq!(
        compressed_account.token.amount, transfer_amount,
        "Compressed account should have the transferred tokens"
    );

    println!("  - cToken account compressed and closed");
    println!(
        "  - Compressed token account owner: {}",
        compressed_account.token.owner
    );
    println!(
        "  - Compressed token account amount: {}",
        compressed_account.token.amount
    );

    // 10. Recreate cToken ATA for decompression with compression_only: true
    println!("\nRecreating cToken ATA for decompression...");
    let compressible_params = CompressibleParams {
        compression_only: true,
        ..Default::default()
    };
    let create_ata_instruction =
        CreateAssociatedCTokenAccount::new(payer.pubkey(), ctoken_recipient.pubkey(), mint)
            .with_compressible(compressible_params)
            .idempotent()
            .instruction()
            .unwrap();

    rpc.create_and_send_transaction(&[create_ata_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify cToken ATA was recreated
    let ctoken_account_data = rpc.get_account(ctoken_ata).await.unwrap().unwrap();
    assert!(
        !ctoken_account_data.data.is_empty(),
        "cToken ATA should exist after recreation"
    );
    println!("  - cToken ATA recreated: {}", ctoken_ata);

    // 11. Get validity proof for the compressed account
    let compressed_hashes: Vec<_> = compressed_accounts
        .iter()
        .map(|acc| acc.account.hash)
        .collect();

    let rpc_result = rpc
        .get_validity_proof(compressed_hashes, vec![], None)
        .await
        .unwrap()
        .value;

    // Get token data and discriminator from compressed account
    let token_data = compressed_accounts[0].token.clone();
    let discriminator = compressed_accounts[0]
        .account
        .data
        .as_ref()
        .unwrap()
        .discriminator;

    // Get tree info from validity proof result
    let account_proof = &rpc_result.accounts[0];

    // 12. Decompress compressed tokens to cToken account
    println!("Decompressing tokens to cToken account...");
    let decompress_instruction = DecompressToCtoken {
        token_data,
        discriminator,
        merkle_tree: account_proof.tree_info.tree,
        queue: account_proof.tree_info.queue,
        leaf_index: account_proof.leaf_index as u32,
        root_index: account_proof.root_index.root_index().unwrap_or(0),
        destination_ctoken_account: ctoken_ata,
        payer: payer.pubkey(),
        validity_proof: rpc_result.proof,
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(
        &[decompress_instruction],
        &payer.pubkey(),
        &[&payer, &ctoken_recipient],
    )
    .await
    .unwrap();

    // 13. Verify compressed accounts are consumed
    let remaining_compressed = rpc
        .get_compressed_token_accounts_by_owner(&ctoken_recipient.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(
        remaining_compressed.len(),
        0,
        "All compressed accounts should be consumed after decompression"
    );
    println!("  - Compressed accounts consumed");

    // 14. Verify cToken account has tokens again
    let ctoken_account_data = rpc.get_account(ctoken_ata).await.unwrap().unwrap();
    let ctoken_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&ctoken_account_data.data[..165]).unwrap();
    let decompressed_balance: u64 = ctoken_account.amount.into();
    assert_eq!(
        decompressed_balance, transfer_amount,
        "cToken account should have the decompressed tokens"
    );
    println!(
        "  - cToken account balance after decompression: {}",
        decompressed_balance
    );

    println!("\nToken-2022 (restricted extensions) to cToken scenario test passed!");
}

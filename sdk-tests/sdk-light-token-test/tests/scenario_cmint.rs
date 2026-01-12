// cMint to cToken scenario test - Direct SDK calls without wrapper program
//
// This test demonstrates the complete flow:
// 1. Create cMint (compressed mint)
// 2. Create 2 cToken ATAs for different owners
// 3. Mint cTokens to both accounts
// 4. Transfer cTokens from account 1 to account 2
// 5. Advance epochs to trigger compression
// 6. Verify cToken account is compressed and closed
// 7. Recreate cToken ATA
// 8. Decompress compressed tokens back to cToken account
// 9. Verify cToken account has tokens again

mod shared;

use borsh::BorshDeserialize;
use light_client::{indexer::Indexer, rpc::Rpc};
use light_program_test::{program_test::TestRpc, LightProgramTest, ProgramTestConfig};
use light_token_sdk::token::{
    CreateAssociatedCTokenAccount, DecompressToCtoken, Token, TransferCToken,
};
use solana_sdk::{signature::Keypair, signer::Signer};

/// Test the complete cMint to cToken flow using direct SDK calls
#[tokio::test]
async fn test_cmint_to_ctoken_scenario() {
    // 1. Setup test environment
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // 2. Create two token owners
    let owner1 = Keypair::new();
    let owner2 = Keypair::new();

    // Airdrop lamports to owners (needed for signing transactions)
    light_test_utils::airdrop_lamports(&mut rpc, &owner1.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    light_test_utils::airdrop_lamports(&mut rpc, &owner2.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // 3. Create cMint and cToken ATAs with initial balances
    let mint_amount1 = 10_000u64;
    let mint_amount2 = 5_000u64;
    let transfer_amount = 3_000u64;

    let (mint, _compression_address, ata_pubkeys, _mint_seed) =
        shared::setup_create_compressed_mint(
            &mut rpc,
            &payer,
            payer.pubkey(), // mint_authority
            9,              // decimals
            vec![
                (mint_amount1, owner1.pubkey()),
                (mint_amount2, owner2.pubkey()),
            ],
        )
        .await;

    let ctoken_ata1 = ata_pubkeys[0];
    let ctoken_ata2 = ata_pubkeys[1];

    // 4. Verify initial balances
    let ctoken_account_data = rpc.get_account(ctoken_ata1).await.unwrap().unwrap();
    let ctoken_account = Token::deserialize(&mut &ctoken_account_data.data[..]).unwrap();
    let balance1 = ctoken_account.amount;
    assert_eq!(balance1, mint_amount1, "cToken account 1 initial balance");

    let ctoken_account_data = rpc.get_account(ctoken_ata2).await.unwrap().unwrap();
    let ctoken_account = Token::deserialize(&mut &ctoken_account_data.data[..]).unwrap();
    let balance2 = ctoken_account.amount;
    assert_eq!(balance2, mint_amount2, "cToken account 2 initial balance");

    println!("cMint scenario test setup complete!");
    println!("  - Created cMint: {}", mint);
    println!(
        "  - cToken account 1: {} (balance: {})",
        ctoken_ata1, balance1
    );
    println!(
        "  - cToken account 2: {} (balance: {})",
        ctoken_ata2, balance2
    );

    // 5. Transfer cTokens from account 1 to account 2
    let transfer_instruction = TransferCToken {
        source: ctoken_ata1,
        destination: ctoken_ata2,
        amount: transfer_amount,
        authority: owner1.pubkey(),
        max_top_up: None,
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[transfer_instruction], &payer.pubkey(), &[&payer, &owner1])
        .await
        .unwrap();

    // 6. Verify balances after transfer
    let ctoken_account_data = rpc.get_account(ctoken_ata1).await.unwrap().unwrap();
    let ctoken_account = Token::deserialize(&mut &ctoken_account_data.data[..]).unwrap();
    let balance1_after = ctoken_account.amount;
    assert_eq!(
        balance1_after,
        mint_amount1 - transfer_amount,
        "cToken account 1 balance after transfer"
    );

    let ctoken_account_data = rpc.get_account(ctoken_ata2).await.unwrap().unwrap();
    let ctoken_account = Token::deserialize(&mut &ctoken_account_data.data[..]).unwrap();
    let balance2_after = ctoken_account.amount;
    assert_eq!(
        balance2_after,
        mint_amount2 + transfer_amount,
        "cToken account 2 balance after transfer"
    );

    println!("\nTransfer completed!");
    println!(
        "  - Transferred {} from account 1 to account 2",
        transfer_amount
    );
    println!(
        "  - cToken account 1 balance: {} -> {}",
        balance1, balance1_after
    );
    println!(
        "  - cToken account 2 balance: {} -> {}",
        balance2, balance2_after
    );

    // 7. Advance 25 epochs to trigger compression (default prepaid is 16 epochs)
    println!("\nAdvancing 25 epochs to trigger compression...");
    rpc.warp_epoch_forward(25).await.unwrap();

    // 8. Verify cToken account 2 is compressed and closed
    let closed_account = rpc.get_account(ctoken_ata2).await.unwrap();
    match closed_account {
        Some(account) => {
            assert_eq!(
                account.lamports, 0,
                "cToken account 2 should be closed (0 lamports)"
            );
        }
        None => {
            println!("  - cToken account 2 no longer exists (closed)");
        }
    }

    // Verify compressed token account exists for the ATA
    // For ATAs, the compressed account owner is the ATA pubkey (not wallet owner)
    let compressed_accounts = rpc
        .get_compressed_token_accounts_by_owner(&ctoken_ata2, None, None)
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
        compressed_account.token.owner, ctoken_ata2,
        "Compressed account owner should be the ATA pubkey"
    );
    assert_eq!(
        compressed_account.token.amount,
        mint_amount2 + transfer_amount,
        "Compressed account should have the expected tokens"
    );

    println!("  - cToken account 2 compressed and closed");
    println!(
        "  - Compressed token account owner: {}",
        compressed_account.token.owner
    );
    println!(
        "  - Compressed token account amount: {}",
        compressed_account.token.amount
    );

    // 9. Recreate cToken ATA for decompression (idempotent)
    println!("\nRecreating cToken ATA for decompression...");
    let create_ata_instruction =
        CreateAssociatedCTokenAccount::new(payer.pubkey(), owner2.pubkey(), mint)
            .idempotent()
            .instruction()
            .unwrap();

    rpc.create_and_send_transaction(&[create_ata_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify cToken ATA was recreated
    let ctoken_account_data = rpc.get_account(ctoken_ata2).await.unwrap().unwrap();
    assert!(
        !ctoken_account_data.data.is_empty(),
        "cToken ATA should exist after recreation"
    );
    println!("  - cToken ATA recreated: {}", ctoken_ata2);
    let deserialized_ata = Token::try_from_slice(ctoken_account_data.data.as_slice()).unwrap();
    println!("deserialized ata {:?}", deserialized_ata);

    // 10. Get validity proof for the compressed account
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

    // 11. Decompress compressed tokens to cToken account
    // For ATA decompress, the wallet owner (owner2) must sign
    println!("Decompressing tokens to cToken account...");
    println!("discriminator {:?}", discriminator);
    println!("token_data {:?}", token_data);
    let decompress_instruction = DecompressToCtoken {
        token_data,
        discriminator,
        merkle_tree: account_proof.tree_info.tree,
        queue: account_proof.tree_info.queue,
        leaf_index: account_proof.leaf_index as u32,
        root_index: account_proof.root_index.root_index().unwrap_or(0),
        destination_ctoken_account: ctoken_ata2,
        payer: payer.pubkey(),
        signer: owner2.pubkey(), // Wallet owner is the signer for ATA decompress
        validity_proof: rpc_result.proof,
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(
        &[decompress_instruction],
        &payer.pubkey(),
        &[&payer, &owner2],
    )
    .await
    .unwrap();

    // 12. Verify compressed accounts are consumed
    let remaining_compressed = rpc
        .get_compressed_token_accounts_by_owner(&ctoken_ata2, None, None)
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

    // 13. Verify cToken account has tokens again
    let ctoken_account_data = rpc.get_account(ctoken_ata2).await.unwrap().unwrap();
    let ctoken_account = Token::deserialize(&mut &ctoken_account_data.data[..]).unwrap();
    let decompressed_balance = ctoken_account.amount;
    assert_eq!(
        decompressed_balance,
        mint_amount2 + transfer_amount,
        "cToken account should have the decompressed tokens"
    );
    println!(
        "  - cToken account balance after decompression: {}",
        decompressed_balance
    );

    println!("\ncMint to cToken scenario test with compression and decompression passed!");
}

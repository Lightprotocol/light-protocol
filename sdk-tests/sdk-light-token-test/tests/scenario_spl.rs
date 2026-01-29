// SPL to cToken scenario test - Direct SDK calls without wrapper program
//
// This test demonstrates the complete flow:
// 1. Create SPL mint manually (with freeze authority)
// 2. Create token pool (SPL interface PDA) using SDK instruction
// 3. Create SPL token account
// 4. Mint SPL tokens
// 5. Create cToken ATA (compressible)
// 6. Transfer SPL tokens to cToken account
// 7. Verify transfer results
// 8. Freeze cToken account
// 9. Thaw cToken account
// 10. Advance epochs to trigger compression
// 11. Verify cToken account is compressed and closed
// 12. Recreate cToken ATA
// 13. Decompress compressed tokens back to cToken account
// 14. Verify cToken account has tokens again

use anchor_spl::token::{spl_token, Mint};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_program_test::{program_test::TestRpc, LightProgramTest, ProgramTestConfig};
use light_test_utils::spl::{create_token_account, mint_spl_tokens};
use light_token::{
    instruction::{
        derive_token_ata, CreateAssociatedTokenAccount, Decompress, Freeze, Thaw, TransferFromSpl,
    },
    spl_interface::{find_spl_interface_pda_with_index, CreateSplInterfacePda},
};
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_token_2022::pod::PodAccount;

/// Test the complete SPL to cToken flow using direct SDK calls
#[tokio::test]
async fn test_spl_to_ctoken_scenario() {
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

    // 2. Create SPL mint manually
    let mint_keypair = Keypair::new();
    let mint = mint_keypair.pubkey();
    let decimals = 2u8;

    // Get rent for mint account
    let mint_rent = rpc
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .await
        .unwrap();

    // Create mint account instruction
    let create_mint_account_ix = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &mint,
        mint_rent,
        Mint::LEN as u64,
        &spl_token::ID,
    );

    // Initialize mint instruction
    let initialize_mint_ix = spl_token::instruction::initialize_mint(
        &spl_token::ID,
        &mint,
        &payer.pubkey(),       // mint authority
        Some(&payer.pubkey()), // freeze authority
        decimals,
    )
    .unwrap();

    rpc.create_and_send_transaction(
        &[create_mint_account_ix, initialize_mint_ix],
        &payer.pubkey(),
        &[&payer, &mint_keypair],
    )
    .await
    .unwrap();

    // 3. Create token pool (SPL interface PDA) using SDK instruction
    let create_pool_ix =
        CreateSplInterfacePda::new(payer.pubkey(), mint, anchor_spl::token::ID, false)
            .instruction();

    rpc.create_and_send_transaction(&[create_pool_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    let mint_amount = 10_000u64;
    let transfer_amount = 5_000u64;

    // 4. Create SPL token account
    let spl_token_account_keypair = Keypair::new();
    create_token_account(&mut rpc, &mint, &spl_token_account_keypair, &token_owner)
        .await
        .unwrap();

    // 5. Mint SPL tokens to the SPL account
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &spl_token_account_keypair.pubkey(),
        &payer.pubkey(),
        &payer,
        mint_amount,
        false,
    )
    .await
    .unwrap();

    // Verify SPL account has tokens
    let spl_account_data = rpc
        .get_account(spl_token_account_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let spl_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data).unwrap();
    let initial_spl_balance: u64 = spl_account.amount.into();
    assert_eq!(initial_spl_balance, mint_amount);

    // 6. Create cToken ATA for the recipient (compressible with default 16 prepaid epochs)
    let ctoken_recipient = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &ctoken_recipient.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    let (ctoken_ata, _bump) = derive_token_ata(&ctoken_recipient.pubkey(), &mint);
    let create_ata_instruction =
        CreateAssociatedTokenAccount::new(payer.pubkey(), ctoken_recipient.pubkey(), mint)
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

    // 7. Transfer SPL tokens to cToken account
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint, 0, false);

    let transfer_instruction = TransferFromSpl {
        amount: transfer_amount,
        spl_interface_pda_bump,
        decimals,
        source_spl_token_account: spl_token_account_keypair.pubkey(),
        destination: ctoken_ata,
        authority: token_owner.pubkey(),
        mint,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: anchor_spl::token::ID,
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
    // Check SPL account balance decreased
    let spl_account_data = rpc
        .get_account(spl_token_account_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let spl_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data).unwrap();
    let final_spl_balance: u64 = spl_account.amount.into();
    assert_eq!(
        final_spl_balance,
        mint_amount - transfer_amount,
        "SPL account balance should have decreased by transfer amount"
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

    println!("SPL to cToken transfer completed!");
    println!("  - Created SPL mint: {}", mint);
    println!(
        "  - Created SPL token account: {}",
        spl_token_account_keypair.pubkey()
    );
    println!("  - Minted {} tokens to SPL account", mint_amount);
    println!("  - Created cToken ATA: {}", ctoken_ata);
    println!(
        "  - Transferred {} tokens from SPL to cToken",
        transfer_amount
    );
    println!(
        "  - Final SPL balance: {}, cToken balance: {}",
        final_spl_balance, ctoken_balance
    );

    // 8. Freeze the cToken account
    println!("\nFreezing cToken account...");
    let freeze_instruction = Freeze {
        token_account: ctoken_ata,
        mint,
        freeze_authority: payer.pubkey(),
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[freeze_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify account is frozen (state == 2)
    let ctoken_account_data = rpc.get_account(ctoken_ata).await.unwrap().unwrap();
    let ctoken_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&ctoken_account_data.data[..165]).unwrap();
    assert_eq!(
        ctoken_account.state,
        spl_token_2022::state::AccountState::Frozen as u8,
        "cToken account should be frozen"
    );
    println!("  - cToken account frozen");

    // 9. Thaw the cToken account
    println!("Thawing cToken account...");
    let thaw_instruction = Thaw {
        token_account: ctoken_ata,
        mint,
        freeze_authority: payer.pubkey(),
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[thaw_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify account is thawed (state == 1)
    let ctoken_account_data = rpc.get_account(ctoken_ata).await.unwrap().unwrap();
    let ctoken_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&ctoken_account_data.data[..165]).unwrap();
    assert_eq!(
        ctoken_account.state,
        spl_token_2022::state::AccountState::Initialized as u8,
        "cToken account should be thawed (initialized)"
    );
    println!("  - cToken account thawed");

    // 10. Advance 25 epochs to trigger compression (default prepaid is 16 epochs)
    println!("\nAdvancing 25 epochs to trigger compression...");
    rpc.warp_epoch_forward(25).await.unwrap();

    // 11. Verify cToken account is compressed and closed
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

    // Verify compressed token account exists (owner is ATA pubkey for is_ata accounts)
    let compressed_accounts = rpc
        .get_compressed_token_accounts_by_owner(&ctoken_ata, None, None)
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
        compressed_account.token.owner, ctoken_ata,
        "Compressed account owner should be ATA pubkey"
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

    // 12. Recreate cToken ATA for decompression (idempotent)
    println!("\nRecreating cToken ATA for decompression...");
    let create_ata_instruction =
        CreateAssociatedTokenAccount::new(payer.pubkey(), ctoken_recipient.pubkey(), mint)
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

    // Get validity proof for the compressed account
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
    let token_data = compressed_accounts[0].token.clone().into();
    let discriminator = compressed_accounts[0]
        .account
        .data
        .as_ref()
        .unwrap()
        .discriminator;

    // Get tree info from validity proof result
    let account_proof = &rpc_result.accounts[0];

    // 13. Decompress compressed tokens to cToken account
    println!("Decompressing tokens to cToken account...");
    let decompress_instruction = Decompress {
        token_data,
        discriminator,
        merkle_tree: account_proof.tree_info.tree,
        queue: account_proof.tree_info.queue,
        leaf_index: account_proof.leaf_index as u32,
        root_index: account_proof.root_index.root_index().unwrap_or(0),
        destination: ctoken_ata,
        payer: payer.pubkey(),
        signer: ctoken_recipient.pubkey(),
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

    // Verify compressed accounts are consumed
    let remaining_compressed = rpc
        .get_compressed_token_accounts_by_owner(&ctoken_ata, None, None)
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

    println!("\nSPL to cToken scenario test with compression and decompression passed!");
}

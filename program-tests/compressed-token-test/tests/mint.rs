// #![cfg(feature = "test-sbf")]

use std::assert_eq;

use anchor_lang::{prelude::borsh::BorshDeserialize, solana_program::program_pack::Pack};
use anchor_spl::token_2022::spl_token_2022;
use light_client::indexer::Indexer;
use light_compressed_token_sdk::instructions::{
    create_associated_token_account, derive_compressed_mint_address, derive_ctoken_ata,
    find_spl_mint_address,
};
use light_ctoken_types::{
    instructions::{
        extensions::token_metadata::TokenMetadataInstructionData, mint_to_compressed::Recipient,
    },
    state::{
        extensions::{AdditionalMetadata, ExtensionStruct, Metadata},
        CompressedMint,
    },
    COMPRESSED_MINT_SEED,
};
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    assert_mint_to_compressed::assert_mint_to_compressed_one, assert_spl_mint::assert_spl_mint,
    mint_assert::assert_compressed_mint_account, Rpc,
};
use light_token_client::{
    actions::{create_mint, create_spl_mint, mint_to_compressed, transfer2},
    instructions::transfer2::{
        create_decompress_instruction, create_generic_transfer2_instruction, CompressInput,
        DecompressInput, Transfer2InstructionType, TransferInput,
    },
};
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

#[tokio::test]
#[serial]
async fn test_create_compressed_mint() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Get necessary values for the rest of the test
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Test parameters
    let decimals = 6u8;
    let mint_authority_keypair = Keypair::new(); // Create keypair so we can sign
    let mint_authority = mint_authority_keypair.pubkey();
    let freeze_authority = Pubkey::new_unique();
    let mint_seed = Keypair::new();
    // Derive compressed mint address for verification
    let compressed_mint_address =
        derive_compressed_mint_address(&mint_seed.pubkey(), &address_tree_pubkey);

    // Find mint PDA for the rest of the test
    let (spl_mint_pda, _) = find_spl_mint_address(&mint_seed.pubkey());

    // Create compressed mint using the action
    create_mint(
        &mut rpc,
        &mint_seed,
        decimals,
        mint_authority,
        Some(freeze_authority),
        None, // No metadata
        &payer,
    )
    .await
    .unwrap();

    // Verify the compressed mint was created
    let compressed_mint_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value;

    assert_compressed_mint_account(
        &compressed_mint_account,
        compressed_mint_address,
        spl_mint_pda,
        decimals,
        mint_authority,
        freeze_authority,
        None, // No metadata
    );

    // Test mint_to_compressed functionality
    let recipient_keypair = Keypair::new();
    let recipient = recipient_keypair.pubkey();
    let mint_amount = 1000u64;
    let expected_supply = mint_amount; // After minting tokens, SPL mint should have this supply
    let lamports = Some(10000u64);

    // Use our mint_to_compressed action helper
    mint_to_compressed(
        &mut rpc,
        spl_mint_pda,
        vec![Recipient {
            recipient: recipient.into(),
            amount: mint_amount,
        }],
        &mint_authority_keypair,
        &payer,
        lamports,
    )
    .await
    .unwrap();

    // Verify minted tokens using our assertion helper
    let _token_account = assert_mint_to_compressed_one(
        &mut rpc,
        spl_mint_pda,
        recipient,
        mint_amount,
        expected_supply,
    )
    .await;

    // Test create_spl_mint functionality using our helper

    // Use our create_spl_mint action helper (automatically handles proofs, PDAs, and transaction)
    create_spl_mint(
        &mut rpc,
        compressed_mint_address,
        &mint_seed,
        &mint_authority_keypair,
        &payer,
    )
    .await
    .unwrap();

    // Verify SPL mint was created using our assertion helper
    assert_spl_mint(&mut rpc, mint_seed.pubkey()).await;

    // Test decompression functionality
    println!("Testing token decompression...");

    // Create SPL token account for the recipient
    let recipient_token_keypair = Keypair::new(); // Create keypair for token account
    light_test_utils::spl::create_token_2022_account(
        &mut rpc,
        &spl_mint_pda,
        &recipient_token_keypair,
        &payer,
        true, // token_22
    )
    .await
    .unwrap();

    // Get the compressed token account for decompression
    let compressed_token_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&recipient, None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(
        compressed_token_accounts.len(),
        1,
        "Should have one compressed token account"
    );

    let new_recipient_keypair = Keypair::new();
    let new_recipient = new_recipient_keypair.pubkey();
    let transfer_amount = mint_amount; // Transfer all tokens (1000)
    transfer2::transfer(
        &mut rpc,
        &compressed_token_accounts,
        new_recipient,
        transfer_amount,
        &recipient_keypair,
        &payer,
    )
    .await
    .unwrap();

    // Verify the transfer was successful
    let new_token_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&new_recipient, None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(
        new_token_accounts.len(),
        1,
        "New recipient should have exactly one token account"
    );
    assert_eq!(
        new_token_accounts[0].token.amount, transfer_amount,
        "New recipient should have the transferred amount"
    );
    assert_eq!(
        new_token_accounts[0].token.mint, spl_mint_pda,
        "New recipient token should have correct mint"
    );

    println!("✅ Multi-transfer executed successfully!");
    println!(
        "   - Transferred {} tokens from {} to {}",
        transfer_amount, recipient, new_recipient
    );

    // Get fresh compressed token accounts after the multi-transfer
    let fresh_token_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&new_recipient, None, None)
        .await
        .unwrap()
        .value
        .items;

    assert!(
        !fresh_token_accounts.is_empty(),
        "Recipient should have compressed tokens after transfer"
    );
    let compressed_token_account = &fresh_token_accounts[0];

    // Debug: Print the compressed token account details
    println!("🔍 Debug compressed token account:");
    println!("   - Amount: {}", compressed_token_account.token.amount);
    println!("   - Owner: {}", compressed_token_account.token.owner);
    println!("   - Mint: {}", compressed_token_account.token.mint);

    let decompress_amount = 300u64;
    let remaining_amount = transfer_amount - decompress_amount;

    // Create compressed token associated token account for decompression
    let (ctoken_ata_pubkey, _bump) = derive_ctoken_ata(&new_recipient, &spl_mint_pda);
    let create_ata_instruction =
        create_associated_token_account(payer.pubkey(), new_recipient, spl_mint_pda).unwrap();
    rpc.create_and_send_transaction(&[create_ata_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Create decompression instruction using the wrapper
    let decompress_instruction = create_decompress_instruction(
        &mut rpc,
        std::slice::from_ref(compressed_token_account),
        decompress_amount,
        ctoken_ata_pubkey,
        payer.pubkey(),
    )
    .await
    .unwrap();

    println!("🔓 Sending decompression transaction...");
    println!("   - Decompress amount: {}", decompress_amount);
    println!("   - Remaining amount: {}", remaining_amount);
    println!("   - SPL token account: {}", ctoken_ata_pubkey);
    println!(" metas {:?}", decompress_instruction.accounts);
    // Send the decompression transaction
    let tx_result = rpc
        .create_and_send_transaction(
            &[decompress_instruction],
            &payer.pubkey(),
            &[&payer, &new_recipient_keypair],
        )
        .await;

    match tx_result {
        Ok(_) => {
            println!("✅ Decompression transaction sent successfully!");

            // Verify the decompression worked
            let ctoken_account = rpc.get_account(ctoken_ata_pubkey).await.unwrap().unwrap();

            let token_account =
                spl_token_2022::state::Account::unpack(&ctoken_account.data).unwrap();
            println!("   - CToken ATA balance: {}", token_account.amount);

            // Assert that the token account contains the expected decompressed amount
            assert_eq!(
                token_account.amount, decompress_amount,
                "Token account should contain exactly the decompressed amount"
            );

            // Check remaining compressed tokens
            let remaining_compressed = rpc
                .indexer()
                .unwrap()
                .get_compressed_token_accounts_by_owner(&new_recipient, None, None)
                .await
                .unwrap()
                .value
                .items;

            if !remaining_compressed.is_empty() {
                println!(
                    "   - Remaining compressed tokens: {}",
                    remaining_compressed[0].token.amount
                );
            }
        }
        Err(e) => {
            println!("❌ Decompression transaction failed: {:?}", e);
            panic!("Decompression transaction failed");
        }
    }

    // Test compressing tokens to a new account
    println!("Testing compression of SPL tokens to compressed tokens...");

    let compress_recipient = Keypair::new();
    let compress_amount = 100u64; // Compress 100 tokens

    // Create compress instruction using the multi-transfer functionality
    let compress_instruction = create_generic_transfer2_instruction(
        &mut rpc,
        vec![Transfer2InstructionType::Compress(CompressInput {
            compressed_token_account: None, // No existing compressed tokens
            solana_token_account: ctoken_ata_pubkey, // Source SPL token account
            to: compress_recipient.pubkey(), // New recipient for compressed tokens
            mint: spl_mint_pda,
            amount: compress_amount,
            authority: new_recipient_keypair.pubkey(), // Authority for compression
            output_queue,
        })],
        payer.pubkey(),
    )
    .await
    .unwrap();
    println!("compress_instruction {:?}", compress_instruction);
    // Execute compression
    rpc.create_and_send_transaction(
        &[compress_instruction],
        &payer.pubkey(),
        &[&payer, &new_recipient_keypair],
    )
    .await
    .unwrap();

    // Verify compressed tokens were created for the new recipient
    let compressed_tokens = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&compress_recipient.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(
        compressed_tokens.len(),
        1,
        "Should have exactly one compressed token account"
    );

    let compressed_token = &compressed_tokens[0].token;
    assert_eq!(
        compressed_token.amount, compress_amount,
        "Compressed token should have correct amount"
    );
    assert_eq!(
        compressed_token.owner,
        compress_recipient.pubkey(),
        "Compressed token should have correct owner"
    );
    assert_eq!(
        compressed_token.mint, spl_mint_pda,
        "Compressed token should have correct mint"
    );

    // Verify SPL token account balance was reduced
    let updated_ctoken_account = rpc.get_account(ctoken_ata_pubkey).await.unwrap().unwrap();
    let updated_token_account =
        spl_token_2022::state::Account::unpack(&updated_ctoken_account.data).unwrap();

    assert_eq!(
        updated_token_account.amount,
        decompress_amount - compress_amount,
        "SPL token account balance should be reduced by compressed amount"
    );

    println!("✅ Compression test completed successfully!");
    println!(
        "   - Compressed {} tokens to new recipient",
        compress_amount
    );
    println!(
        "   - New compressed token owner: {}",
        compress_recipient.pubkey()
    );
    println!(
        "   - Remaining SPL balance: {}",
        updated_token_account.amount
    );

    // Test combining compress, decompress, and transfer in a single instruction
    println!("Testing combined compress + decompress + transfer in single instruction...");

    // Create completely fresh compressed tokens for the transfer operation to avoid double spending
    let transfer_source_recipient = Keypair::new();
    let transfer_compress_amount = 100u64;
    let transfer_compress_instruction = create_generic_transfer2_instruction(
        &mut rpc,
        vec![Transfer2InstructionType::Compress(CompressInput {
            compressed_token_account: None,
            solana_token_account: ctoken_ata_pubkey,
            to: transfer_source_recipient.pubkey(),
            mint: spl_mint_pda,
            amount: transfer_compress_amount,
            authority: new_recipient_keypair.pubkey(), // Authority for compression
            output_queue,
        })],
        payer.pubkey(),
    )
    .await
    .unwrap();

    rpc.create_and_send_transaction(
        &[transfer_compress_instruction],
        &payer.pubkey(),
        &[&payer, &new_recipient_keypair],
    )
    .await
    .unwrap();

    let remaining_compressed_tokens = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&transfer_source_recipient.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;
    println!(
        "Remaining compressed tokens: {:?}",
        remaining_compressed_tokens
    );
    // Create new compressed tokens specifically for the multi-operation test to avoid double spending
    let multi_test_recipient = Keypair::new();
    let multi_compress_amount = 50u64;
    let compress_for_multi_instruction = create_generic_transfer2_instruction(
        &mut rpc,
        vec![Transfer2InstructionType::Compress(CompressInput {
            compressed_token_account: None,
            solana_token_account: ctoken_ata_pubkey,
            to: multi_test_recipient.pubkey(),
            mint: spl_mint_pda,
            amount: multi_compress_amount,
            authority: new_recipient_keypair.pubkey(), // Authority for compression
            output_queue,
        })],
        payer.pubkey(),
    )
    .await
    .unwrap();

    rpc.create_and_send_transaction(
        &[compress_for_multi_instruction],
        &payer.pubkey(),
        &[&payer, &new_recipient_keypair],
    )
    .await
    .unwrap();

    let compressed_tokens_for_compress = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&multi_test_recipient.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;
    println!(
        "compressed_tokens_for_compress: {:?}",
        compressed_tokens_for_compress
    );
    // Create recipients for our multi-operation
    let transfer_recipient = Keypair::new();
    let decompress_recipient = Keypair::new();
    let compress_from_spl_recipient = Keypair::new();

    // Create SPL token account for compression source
    let (compress_source_ata, _) = derive_ctoken_ata(&new_recipient, &spl_mint_pda);
    // This already exists from our previous test

    // Create SPL token account for decompression destination
    let (decompress_dest_ata, _) = derive_ctoken_ata(&decompress_recipient.pubkey(), &spl_mint_pda);
    let create_decompress_ata_instruction = create_associated_token_account(
        payer.pubkey(),
        decompress_recipient.pubkey(),
        spl_mint_pda,
    )
    .unwrap();
    rpc.create_and_send_transaction(
        &[create_decompress_ata_instruction],
        &payer.pubkey(),
        &[&payer],
    )
    .await
    .unwrap();

    // Define amounts for each operation (ensure they don't exceed available balances)
    let transfer_amount = 50u64; // From 700 compressed tokens - safe
    let decompress_amount = 30u64; // From 100 compressed tokens - safe
    let compress_amount_multi = 20u64; // From 200 SPL tokens - very conservative to avoid conflicts

    // Get output queues for the operations
    let multi_output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Create the combined multi-transfer instruction
    let transfer2_instruction = create_generic_transfer2_instruction(
        &mut rpc,
        vec![
            // 1. Transfer compressed tokens to a new recipient
            Transfer2InstructionType::Transfer(TransferInput {
                compressed_token_account: &remaining_compressed_tokens,
                to: transfer_recipient.pubkey(),
                amount: transfer_amount,
            }),
            // 2. Decompress some compressed tokens to SPL tokens
            Transfer2InstructionType::Decompress(DecompressInput {
                compressed_token_account: &compressed_tokens_for_compress,
                decompress_amount,
                solana_token_account: decompress_dest_ata,
                amount: decompress_amount,
            }),
            // 3. Compress SPL tokens to compressed tokens
            Transfer2InstructionType::Compress(CompressInput {
                compressed_token_account: None,
                solana_token_account: compress_source_ata, // Use remaining SPL tokens
                to: compress_from_spl_recipient.pubkey(),
                mint: spl_mint_pda,
                amount: compress_amount_multi,
                authority: new_recipient_keypair.pubkey(), // Authority for compression
                output_queue: multi_output_queue,
            }),
        ],
        payer.pubkey(),
    )
    .await
    .unwrap();

    // Execute the combined instruction with multiple signers
    let tx_result = rpc
        .create_and_send_transaction(
            &[transfer2_instruction],
            &payer.pubkey(),
            &[
                &payer,
                &transfer_source_recipient,
                &multi_test_recipient,
                &new_recipient_keypair,
            ], // Both token owners need to sign
        )
        .await;

    match tx_result {
        Ok(_) => println!("✅ Combined multi-operation transaction succeeded!"),
        Err(e) => {
            println!("❌ Combined multi-operation transaction failed: {:?}", e);

            // Let's check the current state to debug
            println!("Debug info:");
            println!(
                "remaining_compressed_tokens: {:?}",
                remaining_compressed_tokens.len()
            );
            if !remaining_compressed_tokens.is_empty() {
                println!(
                    "  - Amount: {}",
                    remaining_compressed_tokens[0].token.amount
                );
                println!("  - Owner: {}", remaining_compressed_tokens[0].token.owner);
            }
            println!(
                "compressed_tokens_for_compress: {:?}",
                compressed_tokens_for_compress.len()
            );
            if !compressed_tokens_for_compress.is_empty() {
                println!(
                    "  - Amount: {}",
                    compressed_tokens_for_compress[0].token.amount
                );
                println!(
                    "  - Owner: {}",
                    compressed_tokens_for_compress[0].token.owner
                );
            }

            // Check SPL token account balance
            let spl_balance = rpc.get_account(compress_source_ata).await.unwrap().unwrap();
            let spl_token_account =
                spl_token_2022::state::Account::unpack(&spl_balance.data).unwrap();
            println!("SPL token balance: {}", spl_token_account.amount);

            panic!("Combined multi-operation transaction failed");
        }
    }

    // Verify all operations worked correctly

    // 1. Verify transfer: new recipient should have the transferred tokens
    let transfer_result = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&transfer_recipient.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(
        transfer_result.len(),
        1,
        "Transfer recipient should have one token account"
    );
    assert_eq!(
        transfer_result[0].token.amount, transfer_amount,
        "Transfer amount should be correct"
    );

    // 2. Verify decompression: SPL token account should have the decompressed tokens
    let decompress_spl_account = rpc.get_account(decompress_dest_ata).await.unwrap().unwrap();
    let decompress_token_account =
        spl_token_2022::state::Account::unpack(&decompress_spl_account.data).unwrap();
    assert_eq!(
        decompress_token_account.amount, decompress_amount,
        "Decompressed amount should be correct"
    );

    // 3. Verify compression: new recipient should have compressed tokens from SPL
    let compression_result = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&compress_from_spl_recipient.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(
        compression_result.len(),
        1,
        "Compression recipient should have one token account"
    );
    assert_eq!(
        compression_result[0].token.amount, compress_amount_multi,
        "Compression amount should be correct"
    );

    // 4. Verify SPL token account was reduced by compression amount
    let final_spl_account = rpc.get_account(compress_source_ata).await.unwrap().unwrap();
    let final_token_account =
        spl_token_2022::state::Account::unpack(&final_spl_account.data).unwrap();

    // Get the initial balance that compress_source_ata had before the multi-operation
    // compress_source_ata (same as ctoken_ata_pubkey) started with 200 tokens from earlier tests
    // But during the multi-operation test setup, it was used for two more compressions:
    // - transfer_compress_amount = 100 (line 924)
    // - multi_compress_amount = 50 (line 958)
    // - compress_amount_multi = 20 (line 1042 - this operation)
    let initial_balance_from_earlier_tests = 300u64 - 100u64; // 200 tokens
    let balance_after_setup_compressions =
        initial_balance_from_earlier_tests - transfer_compress_amount - multi_compress_amount;
    let expected_final_balance = balance_after_setup_compressions - compress_amount_multi;

    println!(
        "Initial balance from earlier tests: {}",
        initial_balance_from_earlier_tests
    );
    println!(
        "Balance after setup compressions: {}",
        balance_after_setup_compressions
    );
    println!("compress_amount_multi: {}", compress_amount_multi);
    println!("Expected final balance: {}", expected_final_balance);
    println!("Actual final balance: {}", final_token_account.amount);

    assert_eq!(
        final_token_account.amount, expected_final_balance,
        "SPL balance should be reduced by compression amount"
    );
}

#[tokio::test]
#[serial]
async fn test_create_compressed_mint_with_token_metadata() {
    use light_compressed_account::Pubkey as LightPubkey;

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Test parameters
    let decimals = 6u8;
    let mint_authority_keypair = Keypair::new();
    let mint_authority = mint_authority_keypair.pubkey();
    let freeze_authority = Pubkey::new_unique();
    let mint_seed = Keypair::new();

    // Get address tree for creating compressed mint address
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    // Create token metadata extension with additional metadata
    let additional_metadata = vec![
        AdditionalMetadata {
            key: b"website".to_vec(),
            value: b"https://mytoken.com".to_vec(),
        },
        AdditionalMetadata {
            key: b"category".to_vec(),
            value: b"DeFi".to_vec(),
        },
        AdditionalMetadata {
            key: b"creator".to_vec(),
            value: b"TokenMaker Inc.".to_vec(),
        },
    ];

    let token_metadata = TokenMetadataInstructionData {
        update_authority: Some(LightPubkey::from(mint_authority.to_bytes())),
        metadata: Metadata {
            name: b"Test Token".to_vec(),
            symbol: b"TEST".to_vec(),
            uri: b"https://example.com/token.json".to_vec(),
        },
        additional_metadata: Some(additional_metadata.clone()),
        version: 0, // Poseidon hash version
    };
    light_token_client::actions::create_mint(
        &mut rpc,
        &mint_seed,
        decimals,
        mint_authority,
        Some(freeze_authority),
        Some(token_metadata.clone()),
        &payer,
    )
    .await
    .unwrap();
    let (spl_mint_pda, _) = Pubkey::find_program_address(
        &[COMPRESSED_MINT_SEED, mint_seed.pubkey().as_ref()],
        &light_compressed_token::ID,
    );
    let compressed_mint_address = light_compressed_token_sdk::instructions::create_compressed_mint::derive_compressed_mint_address(&mint_seed.pubkey(), &address_tree_pubkey);

    // Verify the compressed mint was created
    let compressed_mint_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value;

    assert_compressed_mint_account(
        &compressed_mint_account,
        compressed_mint_address,
        spl_mint_pda,
        decimals,
        mint_authority,
        freeze_authority,
        Some(token_metadata.clone()),
    );

    // Use our create_spl_mint action helper (automatically handles proofs, PDAs, and transaction)
    create_spl_mint(
        &mut rpc,
        compressed_mint_address,
        &mint_seed,
        &mint_authority_keypair,
        &payer,
    )
    .await
    .unwrap();

    // Verify SPL mint was created using our assertion helper
    assert_spl_mint(&mut rpc, mint_seed.pubkey()).await;

    // Additional verification: Check that extensions are preserved
    let final_compressed_mint_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value;

    let final_compressed_mint: CompressedMint = BorshDeserialize::deserialize(
        &mut final_compressed_mint_account.data.unwrap().data.as_slice(),
    )
    .unwrap();

    // Verify extensions are preserved
    assert!(final_compressed_mint.extensions.is_some());
    let final_extensions = final_compressed_mint.extensions.as_ref().unwrap();
    assert_eq!(final_extensions.len(), 1);
    match &final_extensions[0] {
        ExtensionStruct::TokenMetadata(metadata) => {
            assert_eq!(metadata.mint.to_bytes(), spl_mint_pda.to_bytes());
            assert_eq!(metadata.update_authority, Some(mint_authority.into()));
            assert_eq!(metadata.metadata.name, b"Test Token".to_vec());
            assert_eq!(metadata.metadata.symbol, b"TEST".to_vec());
            assert_eq!(
                metadata.metadata.uri,
                b"https://example.com/token.json".to_vec()
            );
            assert_eq!(metadata.additional_metadata.len(), 3);
            assert_eq!(metadata.version, 0);
        }
        _ => panic!("Expected TokenMetadata extension"),
    }

    // Test mint_to_compressed with the decompressed mint containing metadata extensions

    let mint_amount = 100_000u64; // Mint 100,000 tokens
    let recipient_keypair = Keypair::new();
    let recipient = recipient_keypair.pubkey();

    // Get the updated compressed mint account after decompression (with is_decompressed = true)
    let address_array = final_compressed_mint_account.address.unwrap();
    let updated_compressed_mint_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(address_array, None)
        .await
        .unwrap()
        .value;
    println!(
        "updated_compressed_mint_account {:?}",
        updated_compressed_mint_account
    );
    let updated_compressed_mint: CompressedMint = BorshDeserialize::deserialize(
        &mut updated_compressed_mint_account
            .data
            .as_ref()
            .unwrap()
            .data
            .as_slice(),
    )
    .unwrap();

    // Verify the mint is now marked as decompressed
    assert!(
        updated_compressed_mint.is_decompressed,
        "Compressed mint should be marked as decompressed"
    );

    // Use our mint_to_compressed action helper (automatically handles decompressed mint config)
    mint_to_compressed(
        &mut rpc,
        spl_mint_pda,
        vec![Recipient {
            recipient: recipient.into(),
            amount: mint_amount,
        }],
        &mint_authority_keypair,
        &payer,
        None, // No lamports
    )
    .await
    .unwrap();

    // Verify minted tokens using our assertion helper
    assert_mint_to_compressed_one(
        &mut rpc,
        spl_mint_pda,
        recipient,
        mint_amount,
        mint_amount, // Expected total supply after minting
    )
    .await;
}

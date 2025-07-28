// #![cfg(feature = "test-sbf")]

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
        extensions::{AdditionalMetadata, Metadata},
        CompressedMint,
    },
    COMPRESSED_MINT_SEED,
};
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    assert_mint_to_compressed::assert_mint_to_compressed_one,
    assert_spl_mint::assert_spl_mint,
    assert_transfer2::{
        assert_transfer2, assert_transfer2_compress, assert_transfer2_decompress,
        assert_transfer2_transfer,
    },
    mint_assert::assert_compressed_mint_account,
    Rpc,
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

/// 1. Create compressed mint (no metadata)
/// 2. Mint tokens with compressed mint
/// 3. Create SPL mint from compressed mint
/// 4. Transfer compressed tokens to new recipient
/// 5. Decompress compressed tokens to SPL tokens
/// 6. Compress SPL tokens to compressed tokens
/// 7. Multi-operation transaction (transfer + decompress + compress)
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

    // 1. Create compressed mint (no metadata)
    {
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
    }
    // 2. Mint tokens with compressed mint
    // Test mint_to_compressed functionality
    let recipient_keypair = Keypair::new();
    let recipient = recipient_keypair.pubkey();
    let mint_amount = 1000u64;
    let expected_supply = mint_amount; // After minting tokens, SPL mint should have this supply
    let lamports = Some(10000u64);

    // Use our mint_to_compressed action helper
    {
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

        // Get pre-compressed mint for assertion
        let pre_compressed_mint_account = rpc
            .indexer()
            .unwrap()
            .get_compressed_account(compressed_mint_address, None)
            .await
            .unwrap()
            .value;
        let pre_compressed_mint: CompressedMint = BorshDeserialize::deserialize(
            &mut pre_compressed_mint_account.data.unwrap().data.as_slice(),
        )
        .unwrap();

        // Verify minted tokens using our assertion helper
        assert_mint_to_compressed_one(
            &mut rpc,
            spl_mint_pda,
            recipient,
            mint_amount,
            expected_supply,
            None, // No pre-token pool account for compressed mint
            pre_compressed_mint,
            None, // No pre-spl mint for compressed mint
        )
        .await;
    }
    // 3. Create SPL mint from compressed mint
    // Get compressed mint data before creating SPL mint
    {
        let pre_compressed_mint_account = rpc
            .indexer()
            .unwrap()
            .get_compressed_account(compressed_mint_address, None)
            .await
            .unwrap()
            .value;
        let pre_compressed_mint: CompressedMint = BorshDeserialize::deserialize(
            &mut pre_compressed_mint_account.data.unwrap().data.as_slice(),
        )
        .unwrap();

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
        assert_spl_mint(&mut rpc, mint_seed.pubkey(), &pre_compressed_mint).await;
    }

    // 4. Transfer compressed tokens to new recipient
    // Get the compressed token account for decompression
    let compressed_token_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&recipient, None, None)
        .await
        .unwrap()
        .value
        .items;

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

    // Verify the transfer was successful using new transfer wrapper
    assert_transfer2_transfer(
        &mut rpc,
        light_token_client::instructions::transfer2::TransferInput {
            compressed_token_account: &compressed_token_accounts,
            to: new_recipient,
            amount: transfer_amount,
        },
    )
    .await;

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

    let decompress_amount = 300u64;

    // 5. Decompress compressed tokens to SPL tokens
    // Create compressed token associated token account for decompression
    let (ctoken_ata_pubkey, _bump) = derive_ctoken_ata(&new_recipient, &spl_mint_pda);
    let create_ata_instruction =
        create_associated_token_account(payer.pubkey(), new_recipient, spl_mint_pda).unwrap();
    rpc.create_and_send_transaction(&[create_ata_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Get pre-decompress SPL token account state
    let pre_decompress_account_data = rpc.get_account(ctoken_ata_pubkey).await.unwrap().unwrap();
    let pre_decompress_spl_account =
        spl_token_2022::state::Account::unpack(&pre_decompress_account_data.data).unwrap();

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

            // Use comprehensive decompress assertion
            assert_transfer2_decompress(
                &mut rpc,
                light_token_client::instructions::transfer2::DecompressInput {
                    compressed_token_account: std::slice::from_ref(compressed_token_account),
                    decompress_amount,
                    solana_token_account: ctoken_ata_pubkey,
                    amount: decompress_amount,
                },
                pre_decompress_spl_account,
            )
            .await;

            println!("   - Decompression assertion completed successfully");
        }
        Err(e) => {
            println!("❌ Decompression transaction failed: {:?}", e);
            panic!("Decompression transaction failed");
        }
    }

    // 6. Compress SPL tokens to compressed tokens
    // Test compressing tokens to a new account

    let compress_recipient = Keypair::new();
    let compress_amount = 100u64; // Compress 100 tokens

    // Get pre-compress SPL token account state
    let pre_compress_account_data = rpc.get_account(ctoken_ata_pubkey).await.unwrap().unwrap();
    let pre_compress_spl_account =
        spl_token_2022::state::Account::unpack(&pre_compress_account_data.data).unwrap();

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
    println!("Compress 0 in 1 out");
    // Execute compression
    rpc.create_and_send_transaction(
        &[compress_instruction],
        &payer.pubkey(),
        &[&payer, &new_recipient_keypair],
    )
    .await
    .unwrap();

    // Use comprehensive compress assertion
    assert_transfer2_compress(
        &mut rpc,
        light_token_client::instructions::transfer2::CompressInput {
            compressed_token_account: None,
            solana_token_account: ctoken_ata_pubkey,
            to: compress_recipient.pubkey(),
            mint: spl_mint_pda,
            amount: compress_amount,
            authority: new_recipient_keypair.pubkey(),
            output_queue,
        },
        pre_compress_spl_account,
    )
    .await;

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
    println!("Compress 0 in 1 out");
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
    println!("Compress 0 in 1 out");
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
    // 7. Multi-operation transaction (transfer + decompress + compress)
    // Test transfer + compress + decompress
    {
        // Define amounts for each operation (ensure they don't exceed available balances)
        let transfer_amount = 50u64; // From 700 compressed tokens - safe
        let decompress_amount = 30u64; // From 100 compressed tokens - safe
        let compress_amount_multi = 20u64; // From 200 SPL tokens - very conservative to avoid conflicts

        // Get output queues for the operations
        let multi_output_queue = rpc.get_random_state_tree_info().unwrap().queue;

        // Get pre-account states for SPL token accounts
        let pre_compress_source_data = rpc.get_account(compress_source_ata).await.unwrap().unwrap();
        let pre_compress_source_account =
            spl_token_2022::state::Account::unpack(&pre_compress_source_data.data).unwrap();

        let pre_decompress_dest_data = rpc.get_account(decompress_dest_ata).await.unwrap().unwrap();
        let pre_decompress_dest_account =
            spl_token_2022::state::Account::unpack(&pre_decompress_dest_data.data).unwrap();
        let instruction_actions = vec![
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
        ];
        // Create the combined multi-transfer instruction
        let transfer2_instruction = create_generic_transfer2_instruction(
            &mut rpc,
            instruction_actions.clone(),
            payer.pubkey(),
        )
        .await
        .unwrap();

        // Execute the combined instruction with multiple signers
        println!(
            "Transfer {} in 2 out, compress 0 in 1 out, decompress {} in 1 out",
            remaining_compressed_tokens.len(),
            compressed_tokens_for_compress.len()
        );
        rpc.create_and_send_transaction(
            &[transfer2_instruction],
            &payer.pubkey(),
            &[
                &payer,
                &transfer_source_recipient,
                &multi_test_recipient,
                &new_recipient_keypair,
            ], // Both token owners need to sign
        )
        .await
        .unwrap();

        let pre_token_accounts = vec![
            None,                              // Transfer operation - no pre-account needed
            Some(pre_decompress_dest_account), // Decompress operation - needs pre-account
            Some(pre_compress_source_account), // Compress operation - needs pre-account
        ];

        assert_transfer2(&mut rpc, instruction_actions, pre_token_accounts).await;
    }
}

/// 1. Create compressed mint with metadata
/// 2. Create spl mint
/// 3. mint tokens with compressed mint
#[tokio::test]
#[serial]
async fn test_create_compressed_mint_with_token_metadata_poseidon() {
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
    // 1. Create compressed mint with metadata

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
        update_authority: None,
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

    // 2. Create SPL mint
    {
        // Get compressed mint data before creating SPL mint
        let pre_compressed_mint: CompressedMint = BorshDeserialize::deserialize(
            &mut compressed_mint_account.data.unwrap().data.as_slice(),
        )
        .unwrap();

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
        assert_spl_mint(&mut rpc, mint_seed.pubkey(), &pre_compressed_mint).await;
    }
    // 3. Mint to compressed
    {
        // Get pre-token pool account state for decompressed mint
        let (token_pool_pda, _) =
            light_compressed_token::instructions::create_token_pool::find_token_pool_pda_with_index(
                &spl_mint_pda,
                0,
            );
        let pre_pool_data = rpc.get_account(token_pool_pda).await.unwrap().unwrap();
        let pre_token_pool_account =
            spl_token_2022::state::Account::unpack(&pre_pool_data.data).unwrap();

        let mint_amount = 100_000u64; // Mint 100,000 tokens
        let recipient_keypair = Keypair::new();
        let recipient = recipient_keypair.pubkey();

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

        // Get pre-compressed mint and pre-spl mint for assertion
        let pre_compressed_mint_account = rpc
            .indexer()
            .unwrap()
            .get_compressed_account(compressed_mint_address, None)
            .await
            .unwrap()
            .value;
        let pre_compressed_mint: CompressedMint = BorshDeserialize::deserialize(
            &mut pre_compressed_mint_account.data.unwrap().data.as_slice(),
        )
        .unwrap();

        let pre_spl_mint_data = rpc.get_account(spl_mint_pda).await.unwrap().unwrap();
        let pre_spl_mint = spl_token_2022::state::Mint::unpack(&pre_spl_mint_data.data).unwrap();

        // Verify minted tokens using our assertion helper
        assert_mint_to_compressed_one(
            &mut rpc,
            spl_mint_pda,
            recipient,
            mint_amount,
            mint_amount,                  // Expected total supply after minting
            Some(pre_token_pool_account), // Pass pre-token pool account for decompressed mint validation
            pre_compressed_mint,
            Some(pre_spl_mint),
        )
        .await;
    }
}

#[tokio::test]
#[serial]
async fn test_create_compressed_mint_with_token_metadata_sha() {
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
    // 1. Create compressed mint with metadata

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
        update_authority: None,
        metadata: Metadata {
            name: b"Test Token".to_vec(),
            symbol: b"TEST".to_vec(),
            uri: b"https://example.com/token.json".to_vec(),
        },
        additional_metadata: Some(additional_metadata.clone()),
        version: 1, // Sha hash version
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

    // 2. Create SPL mint
    {
        // Get compressed mint data before creating SPL mint
        let pre_compressed_mint: CompressedMint = BorshDeserialize::deserialize(
            &mut compressed_mint_account.data.unwrap().data.as_slice(),
        )
        .unwrap();

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
        assert_spl_mint(&mut rpc, mint_seed.pubkey(), &pre_compressed_mint).await;
    }
    // 3. Mint to compressed
    {
        // Get pre-token pool account state for decompressed mint
        let (token_pool_pda, _) =
            light_compressed_token::instructions::create_token_pool::find_token_pool_pda_with_index(
                &spl_mint_pda,
                0,
            );
        let pre_pool_data = rpc.get_account(token_pool_pda).await.unwrap().unwrap();
        let pre_token_pool_account =
            spl_token_2022::state::Account::unpack(&pre_pool_data.data).unwrap();

        let mint_amount = 100_000u64; // Mint 100,000 tokens
        let recipient_keypair = Keypair::new();
        let recipient = recipient_keypair.pubkey();

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

        // Get pre-compressed mint and pre-spl mint for assertion
        let pre_compressed_mint_account = rpc
            .indexer()
            .unwrap()
            .get_compressed_account(compressed_mint_address, None)
            .await
            .unwrap()
            .value;
        let pre_compressed_mint: CompressedMint = BorshDeserialize::deserialize(
            &mut pre_compressed_mint_account.data.unwrap().data.as_slice(),
        )
        .unwrap();

        let pre_spl_mint_data = rpc.get_account(spl_mint_pda).await.unwrap().unwrap();
        let pre_spl_mint = spl_token_2022::state::Mint::unpack(&pre_spl_mint_data.data).unwrap();

        // Verify minted tokens using our assertion helper
        assert_mint_to_compressed_one(
            &mut rpc,
            spl_mint_pda,
            recipient,
            mint_amount,
            mint_amount,                  // Expected total supply after minting
            Some(pre_token_pool_account), // Pass pre-token pool account for decompressed mint validation
            pre_compressed_mint,
            Some(pre_spl_mint),
        )
        .await;
    }
}

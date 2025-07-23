// #![cfg(feature = "test-sbf")]

use std::assert_eq;

use anchor_lang::{prelude::borsh::BorshDeserialize, solana_program::program_pack::Pack};
use anchor_spl::token_2022::spl_token_2022;
use light_client::indexer::Indexer;
use light_compressed_token::LIGHT_CPI_SIGNER;
use light_compressed_token_sdk::instructions::{
    close::close_account, create_associated_token_account, create_compressed_mint,
    create_mint_to_compressed_instruction, create_spl_mint_instruction, create_token_account,
    derive_ctoken_ata, CreateCompressedMintInputs, CreateSplMintInputs, DecompressedMintConfig,
    MintToCompressedInputs,
};
use light_ctoken_types::state::solana_ctoken::CompressedToken;
use light_ctoken_types::state::CompressibleExtension;
use light_ctoken_types::COMPRESSED_MINT_SEED;
use light_ctoken_types::{
    instructions::{
        extensions::{token_metadata::TokenMetadataInstructionData, ExtensionInstructionData},
        mint_to_compressed::{CompressedMintInputs, Recipient},
    },
    state::{
        extensions::{AdditionalMetadata, ExtensionStruct, Metadata},
        CompressedMint,
    },
    BASIC_TOKEN_ACCOUNT_SIZE, COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
};
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::Rpc;
use light_token_client::instructions::multi_transfer::{
    create_decompress_instruction, create_generic_multi_transfer_instruction, CompressInput,
    DecompressInput, MultiTransferInstructionType, TransferInput,
};
use light_zero_copy::borsh::Deserialize;
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

#[tokio::test]
#[serial]
async fn test_create_compressed_mint() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Test parameters
    let decimals = 6u8;
    let mint_authority_keypair = Keypair::new(); // Create keypair so we can sign
    let mint_authority = mint_authority_keypair.pubkey();
    let freeze_authority = Pubkey::new_unique();
    let mint_signer = Keypair::new();

    // Get address tree for creating compressed mint address
    let address_tree_pubkey = rpc.get_address_merkle_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;
    let state_merkle_tree = rpc.get_random_state_tree_info().unwrap().tree;

    // Find mint PDA and bump
    let (mint_pda, mint_bump) = Pubkey::find_program_address(
        &[COMPRESSED_MINT_SEED, mint_signer.pubkey().as_ref()],
        &light_compressed_token::ID,
    );

    // Use the mint PDA as the seed for the compressed account address
    let address_seed = mint_pda.to_bytes();

    let compressed_mint_address = light_compressed_account::address::derive_address(
        &address_seed,
        &address_tree_pubkey.to_bytes(),
        &light_compressed_token::ID.to_bytes(),
    );

    // Get validity proof for address creation
    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![light_program_test::AddressWithTree {
                address: compressed_mint_address,
                tree: address_tree_pubkey,
            }],
            None,
        )
        .await
        .unwrap()
        .value;

    let address_merkle_tree_root_index = rpc_result.addresses[0].root_index;

    // Create instruction
    let instruction = create_compressed_mint(CreateCompressedMintInputs {
        decimals,
        mint_authority,
        freeze_authority: Some(freeze_authority),
        proof: rpc_result.proof.0.unwrap(),
        mint_bump,
        address_merkle_tree_root_index,
        mint_signer: mint_signer.pubkey(),
        payer: payer.pubkey(),
        address_tree_pubkey,
        output_queue,
        extensions: None,
    });

    // Send transaction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &mint_signer])
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

    // Create expected compressed mint for comparison
    let expected_compressed_mint = CompressedMint {
        spl_mint: mint_pda.into(),
        supply: 0,
        decimals,
        is_decompressed: false,
        mint_authority: Some(mint_authority.into()),
        freeze_authority: Some(freeze_authority.into()),
        version: 0,
        extensions: None,
    };

    // Verify the account exists and has correct properties
    assert_eq!(
        compressed_mint_account.address.unwrap(),
        compressed_mint_address
    );
    assert_eq!(compressed_mint_account.owner, light_compressed_token::ID);
    assert_eq!(compressed_mint_account.lamports, 0);

    // Verify the compressed mint data
    let compressed_account_data = compressed_mint_account.data.unwrap();
    assert_eq!(
        compressed_account_data.discriminator,
        light_compressed_token::constants::COMPRESSED_MINT_DISCRIMINATOR
    );

    // Deserialize and verify the CompressedMint struct matches expected
    let actual_compressed_mint: CompressedMint =
        BorshDeserialize::deserialize(&mut compressed_account_data.data.as_slice()).unwrap();

    assert_eq!(actual_compressed_mint, expected_compressed_mint);

    // Test mint_to_compressed functionality
    let recipient_keypair = Keypair::new();
    let recipient = recipient_keypair.pubkey();
    let mint_amount = 1000u64;
    let expected_supply = mint_amount; // After minting tokens, SPL mint should have this supply
    let lamports = Some(10000u64);

    // Get state tree for output token accounts
    let state_tree_info = rpc.get_random_state_tree_info().unwrap();
    let state_tree_pubkey = state_tree_info.tree;
    let state_output_queue = state_tree_info.queue;
    println!("state_tree_pubkey {:?}", state_tree_pubkey);
    println!("state_output_queue {:?}", state_output_queue);

    // Prepare compressed mint inputs for minting
    let compressed_mint_inputs = CompressedMintInputs {
        merkle_context: light_compressed_account::compressed_account::PackedMerkleContext {
            merkle_tree_pubkey_index: 0, // Will be set in remaining accounts
            queue_pubkey_index: 1,
            leaf_index: compressed_mint_account.leaf_index,
            prove_by_index: true,
        },
        root_index: 0,
        address: compressed_mint_address,
        compressed_mint_input: expected_compressed_mint,
        output_merkle_tree_index: 3,
    };

    // Create mint_to_compressed instruction using SDK function
    let mint_instruction = create_mint_to_compressed_instruction(MintToCompressedInputs {
        compressed_mint_inputs,
        lamports,
        recipients: vec![Recipient {
            recipient: recipient.into(),
            amount: mint_amount,
        }],
        mint_authority,
        payer: payer.pubkey(),
        state_merkle_tree,
        output_queue,
        state_tree_pubkey,
        decompressed_mint_config: None, // Not a decompressed mint
    })
    .unwrap();

    // Execute mint_to_compressed
    // Note: We need the mint authority to sign since it's the authority for minting
    rpc.create_and_send_transaction(
        &[mint_instruction],
        &payer.pubkey(),
        &[&payer, &mint_authority_keypair],
    )
    .await
    .unwrap();

    // Verify minted token account
    let token_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&recipient, None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(
        token_accounts.len(),
        1,
        "Should have exactly one token account"
    );
    let token_account = &token_accounts[0].token;
    assert_eq!(
        token_account.mint, mint_pda,
        "Token account should have correct mint"
    );
    assert_eq!(
        token_account.amount, mint_amount,
        "Token account should have correct amount"
    );
    assert_eq!(
        token_account.owner, recipient,
        "Token account should have correct owner"
    );

    // Verify updated compressed mint supply
    let updated_compressed_mint_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value;

    let updated_compressed_mint: CompressedMint = BorshDeserialize::deserialize(
        &mut updated_compressed_mint_account
            .data
            .unwrap()
            .data
            .as_slice(),
    )
    .unwrap();

    assert_eq!(
        updated_compressed_mint.supply, mint_amount,
        "Compressed mint supply should be updated to match minted amount"
    );

    // Test create_spl_mint functionality
    println!("Creating SPL mint for the compressed mint...");

    // Find token pool PDA and bump
    let (token_pool_pda, _token_pool_bump) =
        light_compressed_token::instructions::create_token_pool::find_token_pool_pda_with_index(
            &mint_pda, 0,
        );

    // Get validity proof for compressed mint input
    let proof_result = rpc
        .get_validity_proof(vec![updated_compressed_mint_account.hash], vec![], None)
        .await
        .unwrap()
        .value;

    // Prepare compressed mint inputs for create_spl_mint
    let compressed_mint_inputs_for_spl = CompressedMintInputs {
        merkle_context: light_compressed_account::compressed_account::PackedMerkleContext {
            merkle_tree_pubkey_index: 0, // Will be set in remaining accounts
            queue_pubkey_index: 1,
            leaf_index: updated_compressed_mint_account.leaf_index,
            prove_by_index: true,
        },
        root_index: proof_result.accounts[0]
            .root_index
            .root_index()
            .unwrap_or_default(),
        address: compressed_mint_address,
        compressed_mint_input: CompressedMint {
            version: 0,
            spl_mint: mint_pda.into(),
            supply: mint_amount,
            decimals,
            is_decompressed: false,
            mint_authority: Some(mint_authority.into()),
            freeze_authority: Some(freeze_authority.into()),
            extensions: None,
        },
        output_merkle_tree_index: 2,
    };

    // Create create_spl_mint instruction using SDK function
    let create_spl_mint_instruction = create_spl_mint_instruction(CreateSplMintInputs {
        mint_signer: mint_signer.pubkey(),
        mint_bump,
        compressed_mint_inputs: compressed_mint_inputs_for_spl,
        proof: proof_result.proof,
        payer: payer.pubkey(),
        input_merkle_tree: state_merkle_tree,
        input_output_queue: output_queue,
        output_queue,
        mint_authority,
    })
    .unwrap();

    // Execute create_spl_mint
    rpc.create_and_send_transaction(
        &[create_spl_mint_instruction],
        &payer.pubkey(),
        &[&payer, &mint_authority_keypair],
    )
    .await
    .unwrap();

    // Verify SPL mint was created
    let mint_account_data = rpc.get_account(mint_pda).await.unwrap().unwrap();
    let spl_mint = spl_token_2022::state::Mint::unpack(&mint_account_data.data).unwrap();
    assert_eq!(
        spl_mint.decimals, decimals,
        "SPL mint should have correct decimals"
    );
    assert_eq!(
        spl_mint.supply, expected_supply,
        "SPL mint should have minted supply"
    );
    assert_eq!(
        spl_mint.mint_authority.unwrap(),
        LIGHT_CPI_SIGNER.cpi_signer.into(),
        "SPL mint should have correct authority"
    );

    // Verify token pool was created and has the supply
    let token_pool_account_data = rpc.get_account(token_pool_pda).await.unwrap().unwrap();
    let token_pool = spl_token_2022::state::Account::unpack(&token_pool_account_data.data).unwrap();
    assert_eq!(
        token_pool.mint, mint_pda,
        "Token pool should have correct mint"
    );
    assert_eq!(
        token_pool.amount, expected_supply,
        "Token pool should have the minted supply"
    );

    // Verify compressed mint is now marked as decompressed
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

    assert!(
        final_compressed_mint.is_decompressed,
        "Compressed mint should now be marked as decompressed"
    );

    // Test decompression functionality
    println!("Testing token decompression...");

    // Create SPL token account for the recipient
    let recipient_token_keypair = Keypair::new(); // Create keypair for token account
    light_test_utils::spl::create_token_2022_account(
        &mut rpc,
        &mint_pda,
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

    let multi_transfer_instruction = create_generic_multi_transfer_instruction(
        &mut rpc,
        vec![MultiTransferInstructionType::Transfer(TransferInput {
            compressed_token_account: &token_accounts,
            to: new_recipient,
            amount: transfer_amount,
        })],
        payer.pubkey(),
    )
    .await
    .unwrap();
    println!(
        "Multi-transfer instruction: {:?}",
        multi_transfer_instruction.accounts
    );
    // Execute the multi-transfer instruction
    rpc.create_and_send_transaction(
        &[multi_transfer_instruction],
        &payer.pubkey(),
        &[&payer, &recipient_keypair], // Both payer and recipient need to sign
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
        new_token_accounts[0].token.mint, mint_pda,
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
    let (ctoken_ata_pubkey, _bump) = derive_ctoken_ata(&new_recipient, &mint_pda);
    let create_ata_instruction =
        create_associated_token_account(payer.pubkey(), new_recipient, mint_pda).unwrap();
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
    let compress_instruction = create_generic_multi_transfer_instruction(
        &mut rpc,
        vec![MultiTransferInstructionType::Compress(CompressInput {
            compressed_token_account: None, // No existing compressed tokens
            solana_token_account: ctoken_ata_pubkey, // Source SPL token account
            to: compress_recipient.pubkey(), // New recipient for compressed tokens
            mint: mint_pda,
            amount: compress_amount,
            output_queue,
        })],
        payer.pubkey(),
    )
    .await
    .unwrap();
    println!("compress_instruction {:?}", compress_instruction);
    // Execute compression
    rpc.create_and_send_transaction(&[compress_instruction], &payer.pubkey(), &[&payer])
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
        compressed_token.mint, mint_pda,
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
    let transfer_compress_instruction = create_generic_multi_transfer_instruction(
        &mut rpc,
        vec![MultiTransferInstructionType::Compress(CompressInput {
            compressed_token_account: None,
            solana_token_account: ctoken_ata_pubkey,
            to: transfer_source_recipient.pubkey(),
            mint: mint_pda,
            amount: transfer_compress_amount,
            output_queue,
        })],
        payer.pubkey(),
    )
    .await
    .unwrap();

    rpc.create_and_send_transaction(&[transfer_compress_instruction], &payer.pubkey(), &[&payer])
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
    let compress_for_multi_instruction = create_generic_multi_transfer_instruction(
        &mut rpc,
        vec![MultiTransferInstructionType::Compress(CompressInput {
            compressed_token_account: None,
            solana_token_account: ctoken_ata_pubkey,
            to: multi_test_recipient.pubkey(),
            mint: mint_pda,
            amount: multi_compress_amount,
            output_queue,
        })],
        payer.pubkey(),
    )
    .await
    .unwrap();

    rpc.create_and_send_transaction(
        &[compress_for_multi_instruction],
        &payer.pubkey(),
        &[&payer],
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
    let (compress_source_ata, _) = derive_ctoken_ata(&new_recipient, &mint_pda);
    // This already exists from our previous test

    // Create SPL token account for decompression destination
    let (decompress_dest_ata, _) = derive_ctoken_ata(&decompress_recipient.pubkey(), &mint_pda);
    let create_decompress_ata_instruction =
        create_associated_token_account(payer.pubkey(), decompress_recipient.pubkey(), mint_pda)
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
    let multi_transfer_instruction = create_generic_multi_transfer_instruction(
        &mut rpc,
        vec![
            // 1. Transfer compressed tokens to a new recipient
            MultiTransferInstructionType::Transfer(TransferInput {
                compressed_token_account: &remaining_compressed_tokens,
                to: transfer_recipient.pubkey(),
                amount: transfer_amount,
            }),
            // 2. Decompress some compressed tokens to SPL tokens
            MultiTransferInstructionType::Decompress(DecompressInput {
                compressed_token_account: &compressed_tokens_for_compress,
                decompress_amount,
                solana_token_account: decompress_dest_ata,
                amount: decompress_amount,
            }),
            // 3. Compress SPL tokens to compressed tokens
            MultiTransferInstructionType::Compress(CompressInput {
                compressed_token_account: None,
                solana_token_account: compress_source_ata, // Use remaining SPL tokens
                to: compress_from_spl_recipient.pubkey(),
                mint: mint_pda,
                amount: compress_amount_multi,
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
            &[multi_transfer_instruction],
            &payer.pubkey(),
            &[&payer, &transfer_source_recipient, &multi_test_recipient], // Both token owners need to sign
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
async fn test_create_and_close_token_account() {
    use spl_pod::bytemuck::pod_from_bytes;
    use spl_token_2022::{pod::PodAccount, state::AccountState};

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Create a mock mint pubkey (we don't need actual mint for this test)
    let mint_pubkey = Pubkey::new_unique();

    // Create owner for the token account
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();

    // Create a new keypair for the token account
    let token_account_keypair = Keypair::new();
    let token_account_pubkey = token_account_keypair.pubkey();

    // First create the account using system program
    let create_account_system_ix = solana_sdk::system_instruction::create_account(
        &payer_pubkey,
        &token_account_pubkey,
        rpc.get_minimum_balance_for_rent_exemption(165)
            .await
            .unwrap(), // SPL token account size
        165,
        &light_compressed_token::ID, // Our program owns the account
    );

    // Then use SPL token SDK format but with our compressed token program ID
    // This tests that our create_token_account instruction is compatible with SPL SDKs
    let mut initialize_account_ix =
        create_token_account(token_account_pubkey, mint_pubkey, owner_pubkey).unwrap();
    initialize_account_ix.data.push(0);
    // Execute both instructions in one transaction
    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[create_account_system_ix, initialize_account_ix],
        Some(&payer_pubkey),
        &[&payer, &token_account_keypair],
        blockhash,
    );

    rpc.process_transaction(transaction.clone())
        .await
        .expect("Failed to create token account using SPL SDK");

    // Verify the token account was created correctly
    let account_info = rpc
        .get_account(token_account_pubkey)
        .await
        .unwrap()
        .unwrap();

    // Verify account exists and has correct owner
    assert_eq!(account_info.owner, light_compressed_token::ID);
    assert_eq!(account_info.data.len(), 165); // SPL token account size

    let pod_account = pod_from_bytes::<PodAccount>(&account_info.data)
        .expect("Failed to parse token account data");

    // Verify the token account fields
    assert_eq!(Pubkey::from(pod_account.mint), mint_pubkey);
    assert_eq!(Pubkey::from(pod_account.owner), owner_pubkey);
    assert_eq!(u64::from(pod_account.amount), 0); // Should start with zero balance
    assert_eq!(pod_account.state, AccountState::Initialized as u8);

    // Now test closing the account using SPL SDK format
    let destination_keypair = Keypair::new();
    let destination_pubkey = destination_keypair.pubkey();

    // Airdrop some lamports to destination account so it exists
    rpc.context.airdrop(&destination_pubkey, 1_000_000).unwrap();

    // Get initial lamports before closing
    let initial_token_account_lamports = rpc
        .get_account(token_account_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let initial_destination_lamports = rpc
        .get_account(destination_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Create close account instruction using SPL SDK format
    let close_account_ix = close_account(
        &light_compressed_token::ID,
        &token_account_pubkey,
        &destination_pubkey,
        &owner_pubkey,
    );

    // Execute the close instruction
    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let close_transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[close_account_ix],
        Some(&payer_pubkey),
        &[&payer, &owner_keypair], // Need owner to sign
        blockhash,
    );

    rpc.process_transaction(close_transaction)
        .await
        .expect("Failed to close token account using SPL SDK");

    // Verify the account was closed (data should be cleared, lamports should be 0)
    let closed_account = rpc.get_account(token_account_pubkey).await.unwrap();
    if let Some(account) = closed_account {
        // Account still exists, but should have 0 lamports and cleared data
        assert_eq!(account.lamports, 0, "Closed account should have 0 lamports");
        assert!(
            account.data.iter().all(|&b| b == 0),
            "Closed account data should be cleared"
        );
    }

    // Verify lamports were transferred to destination
    let final_destination_lamports = rpc
        .get_account(destination_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    assert_eq!(
        final_destination_lamports,
        initial_destination_lamports + initial_token_account_lamports,
        "Destination should receive all lamports from closed account"
    );
}

#[tokio::test]
async fn test_create_and_close_account_with_rent_authority() {
    use solana_sdk::signature::Signer;
    use solana_sdk::system_instruction;

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Create mint
    let mint_pubkey = Pubkey::new_unique();

    // Create account owner
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();

    // Create rent authority
    let rent_authority_keypair = Keypair::new();
    let rent_authority_pubkey = rent_authority_keypair.pubkey();

    // Create rent recipient
    let rent_recipient_keypair = Keypair::new();
    let rent_recipient_pubkey = rent_recipient_keypair.pubkey();

    // Airdrop lamports to rent recipient so it exists
    rpc.context
        .airdrop(&rent_recipient_pubkey, 1_000_000)
        .unwrap();

    // Create token account keypair
    let token_account_keypair = Keypair::new();
    let token_account_pubkey = token_account_keypair.pubkey();

    // Create system account for token account with space for compressible extension
    let rent_exempt_lamports = rpc
        .get_minimum_balance_for_rent_exemption(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize)
        .await
        .unwrap();

    let create_account_ix = system_instruction::create_account(
        &payer_pubkey,
        &token_account_pubkey,
        rent_exempt_lamports,
        COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
        &light_compressed_token::ID,
    );

    // Create token account using SDK function with compressible extension
    let create_token_account_ix =
        light_compressed_token_sdk::instructions::create_compressible_token_account(
            light_compressed_token_sdk::instructions::CreateCompressibleTokenAccount {
                account_pubkey: token_account_pubkey,
                mint_pubkey,
                owner_pubkey,
                rent_authority: rent_authority_pubkey,
                rent_recipient: rent_recipient_pubkey,
                slots_until_compression: 0, // Allow immediate compression
            },
        )
        .unwrap();

    // Execute account creation
    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let create_transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[create_account_ix, create_token_account_ix],
        Some(&payer_pubkey),
        &[&payer, &token_account_keypair],
        blockhash,
    );

    rpc.process_transaction(create_transaction)
        .await
        .expect("Failed to create token account");

    // Verify the account was created correctly
    let token_account_info = rpc
        .get_account(token_account_pubkey)
        .await
        .unwrap()
        .unwrap();

    // Assert complete token account values
    assert_eq!(token_account_info.owner, light_compressed_token::ID);
    assert_eq!(
        token_account_info.data.len(),
        COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize
    );
    assert!(token_account_info.executable == false);
    assert!(token_account_info.lamports > 0); // Should be rent-exempt

    let expected_token_account = CompressedToken {
        mint: mint_pubkey.into(),
        owner: owner_pubkey.into(),
        amount: 0,
        delegate: None,
        state: 1, // Initialized
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        extensions: Some(vec![
            light_ctoken_types::state::extensions::ExtensionStruct::Compressible(
                CompressibleExtension {
                    last_written_slot: 2, // Program sets this to current slot (2 in test environment)
                    slots_until_compression: 0,
                    rent_authority: rent_authority_pubkey.into(),
                    rent_recipient: rent_recipient_pubkey.into(),
                },
            ),
        ]),
    };

    let (actual_token_account, _) = CompressedToken::zero_copy_at(&token_account_info.data)
        .expect("Failed to deserialize token account with zero-copy");

    assert_eq!(actual_token_account, expected_token_account);

    // Get initial lamports before closing
    let initial_token_account_lamports = rpc
        .get_account(token_account_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let initial_recipient_lamports = rpc
        .get_account(rent_recipient_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // First, try to close with rent authority (should fail for basic token account)
    let close_account_ix = close_account(
        &light_compressed_token::ID,
        &token_account_pubkey,
        &rent_recipient_pubkey, // Use rent recipient as destination
        &rent_authority_pubkey, // Use rent authority as authority
    );

    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let close_transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[close_account_ix],
        Some(&payer_pubkey),
        &[&payer, &rent_authority_keypair], // Sign with rent authority, not owner
        blockhash,
    );

    rpc.process_transaction(close_transaction).await.unwrap();

    // Verify the account was closed (should have 0 lamports and cleared data)
    let closed_account = rpc.get_account(token_account_pubkey).await.unwrap();
    if let Some(account) = closed_account {
        assert_eq!(account.lamports, 0, "Closed account should have 0 lamports");
        assert!(
            account.data.iter().all(|&b| b == 0),
            "Closed account data should be cleared"
        );
    }

    // Verify lamports were transferred to rent recipient
    let final_recipient_lamports = rpc
        .get_account(rent_recipient_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    assert_eq!(
        final_recipient_lamports,
        initial_recipient_lamports + initial_token_account_lamports,
        "Rent recipient should receive all lamports from closed account"
    );
}

#[tokio::test]
async fn test_create_compressible_account_insufficient_size() {
    use light_test_utils::spl::create_mint_helper;
    use solana_sdk::signature::Signer;
    use solana_sdk::system_instruction;

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Create mint
    let mint_pubkey = create_mint_helper(&mut rpc, &payer).await;

    // Create owner and rent authority keypairs
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let rent_authority_keypair = Keypair::new();
    let rent_authority_pubkey = rent_authority_keypair.pubkey();
    let rent_recipient_keypair = Keypair::new();
    let rent_recipient_pubkey = rent_recipient_keypair.pubkey();

    // Create token account keypair
    let token_account_keypair = Keypair::new();
    let token_account_pubkey = token_account_keypair.pubkey();

    // Create system account with INSUFFICIENT size - too small for compressible extension
    let rent_exempt_lamports = rpc
        .get_minimum_balance_for_rent_exemption(BASIC_TOKEN_ACCOUNT_SIZE as usize)
        .await
        .unwrap();

    let create_account_ix = system_instruction::create_account(
        &payer_pubkey,
        &token_account_pubkey,
        rent_exempt_lamports,
        light_ctoken_types::BASIC_TOKEN_ACCOUNT_SIZE, // Intentionally too small for compressible extension
        &light_compressed_token::ID,
    );

    // Create token account using SDK function with compressible extension
    let create_token_account_ix =
        light_compressed_token_sdk::instructions::create_compressible_token_account(
            light_compressed_token_sdk::instructions::CreateCompressibleTokenAccount {
                account_pubkey: token_account_pubkey,
                mint_pubkey,
                owner_pubkey,
                rent_authority: rent_authority_pubkey,
                rent_recipient: rent_recipient_pubkey,
                slots_until_compression: 0,
            },
        )
        .unwrap();

    // Execute account creation - this should fail with account size error
    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let create_transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[create_account_ix, create_token_account_ix],
        Some(&payer_pubkey),
        &[&payer, &token_account_keypair],
        blockhash,
    );

    let result = rpc.process_transaction(create_transaction).await;
    assert!(
        result.is_err(),
        "Expected account creation to fail due to insufficient account size"
    );

    println!("✅ Correctly failed to create compressible token account with insufficient size");
}

#[tokio::test]
async fn test_create_associated_token_account() {
    use spl_pod::bytemuck::pod_from_bytes;
    use spl_token_2022::{pod::PodAccount, state::AccountState};

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Create a mock mint pubkey
    let mint_pubkey = Pubkey::new_unique();

    // Create owner for the associated token account
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();

    // Calculate the expected associated token account address
    let (expected_ata_pubkey, bump) = Pubkey::find_program_address(
        &[
            owner_pubkey.as_ref(),
            light_compressed_token::ID.as_ref(),
            mint_pubkey.as_ref(),
        ],
        &light_compressed_token::ID,
    );

    // Create basic ATA instruction using SDK function
    let instruction = light_compressed_token_sdk::instructions::create_associated_token_account(
        payer_pubkey,
        owner_pubkey,
        mint_pubkey,
    )
    .unwrap();

    // Execute the instruction
    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        blockhash,
    );

    rpc.process_transaction(transaction.clone())
        .await
        .expect("Failed to create associated token account");

    // Verify the associated token account was created correctly
    let token_account_info = rpc.get_account(expected_ata_pubkey).await.unwrap().unwrap();
    {
        // Verify account exists and has correct owner
        assert_eq!(token_account_info.owner, light_compressed_token::ID);
        assert_eq!(token_account_info.data.len(), 165); // SPL token account size

        let pod_account = pod_from_bytes::<PodAccount>(&token_account_info.data)
            .expect("Failed to parse token account data");

        // Verify the token account fields
        assert_eq!(Pubkey::from(pod_account.mint), mint_pubkey);
        assert_eq!(Pubkey::from(pod_account.owner), owner_pubkey);
        assert_eq!(u64::from(pod_account.amount), 0); // Should start with zero balance
        assert_eq!(pod_account.state, AccountState::Initialized as u8);

        // Verify the PDA derivation is correct
        let (derived_ata_pubkey, derived_bump) = Pubkey::find_program_address(
            &[
                owner_pubkey.as_ref(),
                light_compressed_token::ID.as_ref(),
                mint_pubkey.as_ref(),
            ],
            &light_compressed_token::ID,
        );
        assert_eq!(expected_ata_pubkey, derived_ata_pubkey);
        assert_eq!(bump, derived_bump);
    }
    {
        let expected_token_account = CompressedToken {
            mint: mint_pubkey.into(),
            owner: owner_pubkey.into(),
            amount: 0,
            delegate: None,
            state: 1, // Initialized
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            extensions: None,
        };

        let (actual_token_account, _) = CompressedToken::zero_copy_at(&token_account_info.data)
            .expect("Failed to deserialize token account with zero-copy");

        assert_eq!(actual_token_account, expected_token_account);
    }

    // Test compressible associated token account creation
    println!("🧪 Testing compressible associated token account creation...");

    // Create rent authority and recipient for compressible account
    let rent_authority_keypair = Keypair::new();
    let rent_authority_pubkey = rent_authority_keypair.pubkey();
    let rent_recipient_keypair = Keypair::new();
    let rent_recipient_pubkey = rent_recipient_keypair.pubkey();

    // Airdrop lamports to rent recipient so it exists
    rpc.context
        .airdrop(&rent_recipient_pubkey, 1_000_000)
        .unwrap();

    // Create a different owner for the compressible account
    let compressible_owner_keypair = Keypair::new();
    let compressible_owner_pubkey = compressible_owner_keypair.pubkey();

    // Calculate the expected compressible associated token account address
    let (expected_compressible_ata_pubkey, _) = Pubkey::find_program_address(
        &[
            compressible_owner_pubkey.as_ref(),
            light_compressed_token::ID.as_ref(),
            mint_pubkey.as_ref(),
        ],
        &light_compressed_token::ID,
    );

    // Create compressible ATA instruction using SDK function
    let compressible_instruction = light_compressed_token_sdk::instructions::create_compressible_associated_token_account(
        light_compressed_token_sdk::instructions::CreateCompressibleAssociatedTokenAccountInputs {
            payer: payer_pubkey,
            owner: compressible_owner_pubkey,
            mint: mint_pubkey,
            rent_authority: rent_authority_pubkey,
            rent_recipient: rent_recipient_pubkey,
            slots_until_compression: 0,
        }
    ).unwrap();

    // Execute the compressible instruction
    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let compressible_transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[compressible_instruction],
        Some(&payer_pubkey),
        &[&payer],
        blockhash,
    );

    rpc.process_transaction(compressible_transaction)
        .await
        .expect("Failed to create compressible associated token account");

    // Verify the compressible associated token account was created correctly
    let compressible_account_info = rpc
        .get_account(expected_compressible_ata_pubkey)
        .await
        .unwrap()
        .unwrap();

    // Verify account exists and has correct owner and size for compressible account
    assert_eq!(compressible_account_info.owner, light_compressed_token::ID);
    assert_eq!(
        compressible_account_info.data.len(),
        COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize
    ); // Should be compressible size, not basic size

    // Use zero-copy deserialization to verify the compressible account structure
    let (actual_compressible_token_account, _) =
        CompressedToken::zero_copy_at(&compressible_account_info.data)
            .expect("Failed to deserialize compressible token account with zero-copy");

    // Create expected compressible token account with compressible extension

    let expected_compressible_token_account = CompressedToken {
        mint: mint_pubkey.into(),
        owner: compressible_owner_pubkey.into(),
        amount: 0,
        delegate: None,
        state: 1, // Initialized
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        extensions: Some(vec![
            light_ctoken_types::state::extensions::ExtensionStruct::Compressible(
                CompressibleExtension {
                    last_written_slot: 2, // Program sets this to current slot
                    slots_until_compression: 0,
                    rent_authority: rent_authority_pubkey.into(),
                    rent_recipient: rent_recipient_pubkey.into(),
                },
            ),
        ]),
    };

    assert_eq!(
        actual_compressible_token_account,
        expected_compressible_token_account
    );

    // Test that we can close the compressible account using rent authority
    let initial_compressible_lamports = rpc
        .get_account(expected_compressible_ata_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let initial_recipient_lamports = rpc
        .get_account(rent_recipient_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Close account with rent authority
    let close_account_ix = close_account(
        &light_compressed_token::ID,
        &expected_compressible_ata_pubkey,
        &rent_recipient_pubkey,
        &rent_authority_pubkey,
    );

    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let close_transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[close_account_ix],
        Some(&payer_pubkey),
        &[&payer, &rent_authority_keypair],
        blockhash,
    );

    rpc.process_transaction(close_transaction).await.unwrap();

    // Verify the compressible account was closed and lamports transferred
    let closed_compressible_account = rpc
        .get_account(expected_compressible_ata_pubkey)
        .await
        .unwrap();
    if let Some(account) = closed_compressible_account {
        assert_eq!(account.lamports, 0, "Closed account should have 0 lamports");
    }

    let final_recipient_lamports = rpc
        .get_account(rent_recipient_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    assert_eq!(
        final_recipient_lamports,
        initial_recipient_lamports + initial_compressible_lamports,
        "Rent recipient should receive all lamports from closed compressible account"
    );

    println!("✅ Both basic and compressible associated token accounts work correctly!");
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
    let mint_signer = Keypair::new();

    // Get address tree for creating compressed mint address
    let address_tree_pubkey = rpc.get_address_merkle_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Find mint PDA and bump
    let (mint_pda, mint_bump) = Pubkey::find_program_address(
        &[COMPRESSED_MINT_SEED, mint_signer.pubkey().as_ref()],
        &light_compressed_token::ID,
    );

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

    let extensions = vec![ExtensionInstructionData::TokenMetadata(token_metadata)];

    // Use the mint PDA as the seed for the compressed account address
    let address_seed = mint_pda.to_bytes();

    let compressed_mint_address = light_compressed_account::address::derive_address(
        &address_seed,
        &address_tree_pubkey.to_bytes(),
        &light_compressed_token::ID.to_bytes(),
    );

    // Get validity proof for address creation
    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![light_program_test::AddressWithTree {
                address: compressed_mint_address,
                tree: address_tree_pubkey,
            }],
            None,
        )
        .await
        .unwrap()
        .value;

    let address_merkle_tree_root_index = rpc_result.addresses[0].root_index;

    // Create instruction using the helper function
    let instruction = create_compressed_mint(CreateCompressedMintInputs {
        decimals,
        mint_authority,
        freeze_authority: Some(freeze_authority),
        proof: rpc_result.proof.0.unwrap(),
        mint_bump,
        address_merkle_tree_root_index,
        mint_signer: mint_signer.pubkey(),
        payer: payer.pubkey(),
        address_tree_pubkey,
        output_queue,
        extensions: Some(extensions),
    });

    // Send transaction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &mint_signer])
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

    // Verify the account exists and has correct properties
    assert_eq!(
        compressed_mint_account.address.unwrap(),
        compressed_mint_address
    );
    assert_eq!(compressed_mint_account.owner, light_compressed_token::ID);
    assert_eq!(compressed_mint_account.lamports, 0);

    // Verify the compressed mint data
    let compressed_account_data = compressed_mint_account.data.unwrap();
    assert_eq!(
        compressed_account_data.discriminator,
        light_compressed_token::constants::COMPRESSED_MINT_DISCRIMINATOR
    );

    // Deserialize and verify the CompressedMint struct
    let actual_compressed_mint: CompressedMint =
        BorshDeserialize::deserialize(&mut compressed_account_data.data.as_slice()).unwrap();
    // asserts
    {
        // Verify basic mint fields
        assert_eq!(actual_compressed_mint.spl_mint, mint_pda);
        assert_eq!(actual_compressed_mint.supply, 0);
        assert_eq!(actual_compressed_mint.decimals, decimals);
        assert_eq!(actual_compressed_mint.is_decompressed, false);
        assert_eq!(
            actual_compressed_mint.mint_authority,
            Some(mint_authority.into())
        );
        assert_eq!(
            actual_compressed_mint.freeze_authority,
            Some(freeze_authority.into())
        );
        assert_eq!(actual_compressed_mint.version, 0);

        // Verify extensions
        assert!(actual_compressed_mint.extensions.is_some());
        let extensions = actual_compressed_mint.extensions.as_ref().unwrap();
        assert_eq!(extensions.len(), 1);

        match &extensions[0] {
            ExtensionStruct::TokenMetadata(metadata) => {
                assert_eq!(metadata.mint.to_bytes(), mint_pda.to_bytes());
                assert_eq!(metadata.update_authority, Some(mint_authority.into()));
                assert_eq!(metadata.metadata.name, b"Test Token".to_vec());
                assert_eq!(metadata.metadata.symbol, b"TEST".to_vec());
                assert_eq!(
                    metadata.metadata.uri,
                    b"https://example.com/token.json".to_vec()
                );
                // Verify additional metadata
                assert_eq!(metadata.additional_metadata.len(), 3);

                // Sort both expected and actual for comparison
                let mut expected_additional = additional_metadata.clone();
                expected_additional.sort_by(|a, b| a.key.cmp(&b.key));

                let mut actual_additional = metadata.additional_metadata.clone();
                actual_additional.sort_by(|a, b| a.key.cmp(&b.key));

                for (expected, actual) in expected_additional.iter().zip(actual_additional.iter()) {
                    assert_eq!(actual.key, expected.key);
                    assert_eq!(actual.value, expected.value);
                }
                assert_eq!(metadata.version, 0);
            }
            _ => panic!("Expected TokenMetadata extension"),
        }
    }
    // Note: We're creating SPL mint from a compressed mint with 0 supply
    let expected_supply = 0u64; // Should be 0 since compressed mint has no tokens minted

    // Find token pool PDA
    let (token_pool_pda, _token_pool_bump) = Pubkey::find_program_address(
        &[
            light_compressed_token::constants::POOL_SEED,
            &mint_pda.to_bytes(),
        ],
        &light_compressed_token::ID,
    );

    // Get the tree and queue info from the compressed mint account
    let input_tree = compressed_mint_account.tree_info.tree;
    let input_queue = compressed_mint_account.tree_info.queue;

    // Get a separate output queue for the new compressed mint state
    let output_tree_info = rpc.get_random_state_tree_info().unwrap();
    let output_queue = output_tree_info.queue;

    // Get validity proof for compressed mint input - pass the hash
    let proof_result = rpc
        .get_validity_proof(vec![compressed_mint_account.hash], vec![], None)
        .await
        .unwrap()
        .value;

    // Prepare compressed mint inputs
    let compressed_mint_inputs = CompressedMintInputs {
        merkle_context: light_compressed_account::compressed_account::PackedMerkleContext {
            merkle_tree_pubkey_index: 0, // Index 0 in tree_accounts: in_merkle_tree
            queue_pubkey_index: 1,       // Index 1 in tree_accounts: in_output_queue
            leaf_index: compressed_mint_account.leaf_index,
            prove_by_index: true,
        },
        root_index: proof_result.accounts[0]
            .root_index
            .root_index()
            .unwrap_or_default(),
        address: compressed_mint_address,
        compressed_mint_input: actual_compressed_mint.clone(),
        output_merkle_tree_index: 2, // Index 2 in tree_accounts: out_output_queue
    };

    // Create the create_spl_mint instruction using the helper function
    let create_spl_mint_instruction = create_spl_mint_instruction(CreateSplMintInputs {
        mint_signer: mint_signer.pubkey(),
        mint_bump,
        compressed_mint_inputs,
        proof: proof_result.proof,
        payer: payer.pubkey(),
        input_merkle_tree: input_tree,
        input_output_queue: input_queue,
        output_queue,
        mint_authority: mint_authority_keypair.pubkey(),
    })
    .unwrap();

    // Execute create_spl_mint
    rpc.create_and_send_transaction(
        &[create_spl_mint_instruction],
        &payer.pubkey(),
        &[&payer, &mint_authority_keypair],
    )
    .await
    .unwrap();

    // Verify SPL mint was created
    let mint_account_data = rpc.get_account(mint_pda).await.unwrap().unwrap();
    let spl_mint = spl_token_2022::state::Mint::unpack(&mint_account_data.data).unwrap();
    assert_eq!(
        spl_mint.decimals, decimals,
        "SPL mint should have correct decimals"
    );
    assert_eq!(
        spl_mint.supply, expected_supply,
        "SPL mint should have expected supply"
    );
    assert_eq!(
        spl_mint.mint_authority.unwrap(),
        light_compressed_token::LIGHT_CPI_SIGNER.cpi_signer.into(),
        "SPL mint should have correct authority"
    );

    // Verify token pool was created and has the supply
    let token_pool_account_data = rpc.get_account(token_pool_pda).await.unwrap().unwrap();
    let token_pool = spl_token_2022::state::Account::unpack(&token_pool_account_data.data).unwrap();
    assert_eq!(
        token_pool.mint, mint_pda,
        "Token pool should have correct mint"
    );
    assert_eq!(
        token_pool.amount, expected_supply,
        "Token pool should have the expected supply"
    );

    // Verify compressed mint is now marked as decompressed but retains extensions
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

    assert!(
        final_compressed_mint.is_decompressed,
        "Compressed mint should now be marked as decompressed"
    );

    // Verify extensions are preserved
    assert!(final_compressed_mint.extensions.is_some());
    let final_extensions = final_compressed_mint.extensions.as_ref().unwrap();
    assert_eq!(final_extensions.len(), 1);
    match &final_extensions[0] {
        ExtensionStruct::TokenMetadata(metadata) => {
            assert_eq!(metadata.mint.to_bytes(), mint_pda.to_bytes());
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
    println!(
        "🧪 Testing mint_to_compressed with decompressed mint containing metadata extensions..."
    );

    let mint_amount = 100_000u64; // Mint 100,000 tokens
    let recipient_keypair = Keypair::new();
    let recipient = recipient_keypair.pubkey();

    // Get tree info for the mint_to_compressed operation
    let mint_tree_info = rpc.get_random_state_tree_info().unwrap();
    let mint_output_queue = mint_tree_info.queue;

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

    // Create CompressedMintInputs from the updated compressed mint
    let compressed_mint_inputs = CompressedMintInputs {
        merkle_context: light_compressed_account::compressed_account::PackedMerkleContext {
            merkle_tree_pubkey_index: 0, // Index for input tree in tree accounts array
            queue_pubkey_index: 1,       // Index for input queue in tree accounts array
            leaf_index: final_compressed_mint_account.leaf_index,
            prove_by_index: true,
        },
        root_index: 0, // Use default root index for this test
        address: updated_compressed_mint_account.address.unwrap(),
        compressed_mint_input: updated_compressed_mint.clone(),
        output_merkle_tree_index: 0,
    };

    // Create decompressed mint config since this is a decompressed mint
    let decompressed_mint_config = DecompressedMintConfig {
        mint_pda,
        token_pool_pda,
        token_program: spl_token_2022::ID,
    };

    // Create mint_to_compressed instruction using SDK
    let mint_to_instruction = create_mint_to_compressed_instruction(MintToCompressedInputs {
        compressed_mint_inputs,
        lamports: None,
        recipients: vec![Recipient {
            recipient: recipient.into(),
            amount: mint_amount,
        }],
        mint_authority,
        payer: payer.pubkey(),
        state_merkle_tree: updated_compressed_mint_account.tree_info.tree,
        output_queue: mint_output_queue,
        state_tree_pubkey: mint_tree_info.tree,
        decompressed_mint_config: Some(decompressed_mint_config),
    })
    .unwrap();

    // Execute mint_to_compressed
    rpc.create_and_send_transaction(
        &[mint_to_instruction],
        &payer.pubkey(),
        &[&payer, &mint_authority_keypair],
    )
    .await
    .unwrap();
    // TODO: add assert
}

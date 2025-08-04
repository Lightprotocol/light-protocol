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
    assert_mint_to_compressed::{assert_mint_to_compressed, assert_mint_to_compressed_one},
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
            Some(pre_token_pool_account), // Pass pre-token pool account for decompressed mint validation
            pre_compressed_mint,
            Some(pre_spl_mint),
        )
        .await;
    }
}

/// Test updating compressed mint authorities
#[tokio::test]
#[serial]
async fn test_update_compressed_mint_authority() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();

    let payer = Keypair::new();
    rpc.airdrop_lamports(&payer.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let mint_seed = Keypair::new();
    let initial_mint_authority = Keypair::new();
    let initial_freeze_authority = Keypair::new();
    let new_mint_authority = Keypair::new();
    let new_freeze_authority = Keypair::new();

    // 1. Create compressed mint with both authorities
    let _signature = create_mint(
        &mut rpc,
        &mint_seed,
        8, // decimals
        initial_mint_authority.pubkey(),
        Some(initial_freeze_authority.pubkey()),
        None, // no metadata
        &payer,
    )
    .await
    .unwrap();

    // Get the compressed mint address and info
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_compressed_mint_address(&mint_seed.pubkey(), &address_tree_pubkey);

    // Get compressed mint account from indexer
    let compressed_mint_account = rpc
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value;

    // 2. Update mint authority
    let _signature = light_token_client::actions::update_mint_authority(
        &mut rpc,
        &initial_mint_authority,
        Some(new_mint_authority.pubkey()),
        compressed_mint_account.hash,
        compressed_mint_account.leaf_index,
        compressed_mint_account.tree_info.tree,
        &payer,
    )
    .await
    .unwrap();

    println!("Updated mint authority successfully");
    let compressed_mint_account = rpc
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value;
    let compressed_mint =
        CompressedMint::deserialize(&mut &compressed_mint_account.data.as_ref().unwrap().data[..])
            .unwrap();
    println!("compressed_mint {:?}", compressed_mint);
    assert_eq!(
        compressed_mint.mint_authority.unwrap(),
        new_mint_authority.pubkey()
    );
    // 3. Update freeze authority (need to preserve mint authority)
    let _signature = light_token_client::actions::update_freeze_authority(
        &mut rpc,
        &initial_freeze_authority,
        Some(new_freeze_authority.pubkey()),
        new_mint_authority.pubkey(), // Pass the updated mint authority
        compressed_mint_account.hash,
        compressed_mint_account.leaf_index,
        compressed_mint_account.tree_info.tree,
        &payer,
    )
    .await
    .unwrap();
    let compressed_mint_account = rpc
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value;
    let compressed_mint =
        CompressedMint::deserialize(&mut &compressed_mint_account.data.as_ref().unwrap().data[..])
            .unwrap();
    println!("compressed_mint {:?}", compressed_mint);
    assert_eq!(
        compressed_mint.freeze_authority.unwrap(),
        new_freeze_authority.pubkey()
    );
    println!("Updated freeze authority successfully");

    // 4. Test revoking mint authority (setting to None)
    // Note: We need to get fresh account info after the updates
    let updated_compressed_accounts = rpc
        .get_compressed_accounts_by_owner(
            &Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
            None,
            None,
        )
        .await
        .unwrap();

    let updated_compressed_mint_account = updated_compressed_accounts
        .value
        .items
        .iter()
        .find(|account| account.address == Some(compressed_mint_address))
        .expect("Updated compressed mint account not found");

    let _signature = light_token_client::actions::update_mint_authority(
        &mut rpc,
        &new_mint_authority,
        None, // Revoke authority
        updated_compressed_mint_account.hash,
        updated_compressed_mint_account.leaf_index,
        updated_compressed_mint_account.tree_info.tree,
        &payer,
    )
    .await
    .unwrap();

    println!("Revoked mint authority successfully");

    // The test passes if all operations complete without errors
    // In a real scenario, you would verify the compressed mint state
    // but for now we're testing that the instruction can be created and executed
}

/// Test comprehensive mint actions in a single instruction
#[tokio::test]
#[serial]
async fn test_mint_actions_comprehensive() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Test parameters
    let decimals = 8u8;
    let mint_seed = Keypair::new();
    let mint_authority = Keypair::new();
    let freeze_authority = Keypair::new();
    let new_mint_authority = Keypair::new();

    // Recipients for minting
    let recipients = vec![
        light_ctoken_types::instructions::mint_to_compressed::Recipient {
            recipient: Keypair::new().pubkey().to_bytes().into(),
            amount: 1000u64,
        },
        light_ctoken_types::instructions::mint_to_compressed::Recipient {
            recipient: Keypair::new().pubkey().to_bytes().into(),
            amount: 2000u64,
        },
        light_ctoken_types::instructions::mint_to_compressed::Recipient {
            recipient: Keypair::new().pubkey().to_bytes().into(),
            amount: 3000u64,
        },
    ];
    let total_mint_amount = 6000u64;

    // Fund authority accounts
    rpc.airdrop_lamports(&mint_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    rpc.airdrop_lamports(&freeze_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    rpc.airdrop_lamports(&new_mint_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // Derive addresses
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_compressed_mint_address(&mint_seed.pubkey(), &address_tree_pubkey);
    let (spl_mint_pda, _) = find_spl_mint_address(&mint_seed.pubkey());

    // === SINGLE MINT ACTION INSTRUCTION ===
    // Execute ONE instruction with ALL actions
    let signature = light_token_client::actions::mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &mint_authority,
        &payer,
        true,                                // create_spl_mint
        recipients.clone(),                  // mint_to_recipients
        Some(new_mint_authority.pubkey()),   // update_mint_authority
       None,// Some(new_freeze_authority.pubkey()), // update_freeze_authority
        None,                                // no lamports
        Some(light_token_client::instructions::mint_action::NewMint {
            decimals,
            supply:0,
            mint_authority: mint_authority.pubkey(),
            freeze_authority: Some(freeze_authority.pubkey()),
            metadata: Some(light_ctoken_types::instructions::extensions::token_metadata::TokenMetadataInstructionData {
                update_authority: Some(mint_authority.pubkey().into()),
                metadata: light_ctoken_types::state::Metadata {
                    name: "Test Token".as_bytes().to_vec(),
                    symbol: "TEST".as_bytes().to_vec(),
                    uri: "https://example.com/token.json".as_bytes().to_vec(),
                },
                additional_metadata: None,
                version: 1,
            }),
            version: 1,
        }),
    )
    .await
    .unwrap();

    println!("Mint action transaction signature: {}", signature);

    // === VERIFY RESULTS USING EXISTING ASSERTION HELPERS ===

    // Recipients are already in the correct format for assertions
    let expected_recipients: Vec<Recipient> = recipients.clone();

    // Create empty pre-states since everything was created from scratch
    let empty_pre_compressed_mint = CompressedMint {
        spl_mint: spl_mint_pda.into(),
        supply: 0,
        decimals,
        mint_authority: Some(new_mint_authority.pubkey().into()),
        freeze_authority: Some(freeze_authority.pubkey().into()), // We didn't update freeze authority
        is_decompressed: true, // Should be true after CreateSplMint action
        version: 1,            // With metadata
        extensions: Some(vec![
            light_ctoken_types::state::extensions::ExtensionStruct::TokenMetadata(
                light_ctoken_types::state::extensions::TokenMetadata {
                    update_authority: Some(mint_authority.pubkey().into()), // Original authority in metadata
                    mint: spl_mint_pda.into(),
                    metadata: light_ctoken_types::state::Metadata {
                        name: "Test Token".as_bytes().to_vec(),
                        symbol: "TEST".as_bytes().to_vec(),
                        uri: "https://example.com/token.json".as_bytes().to_vec(),
                    },
                    additional_metadata: vec![], // No additional metadata in our test
                    version: 1,
                },
            ),
        ]), // Match the metadata we're creating
    };

    // Use empty token pool account (before creation)
    let empty_token_pool = spl_token_2022::state::Account {
        mint: spl_mint_pda,
        owner: Pubkey::find_program_address(
            &[light_sdk::constants::CPI_AUTHORITY_PDA_SEED],
            &light_compressed_token::ID,
        )
        .0,
        amount: 0, // Started with 0
        delegate: None.into(),
        state: spl_token_2022::state::AccountState::Initialized,
        is_native: None.into(),
        delegated_amount: 0,
        close_authority: None.into(),
    };

    // Use empty SPL mint (before creation)
    let empty_spl_mint = spl_token_2022::state::Mint {
        mint_authority: Some(
            Pubkey::find_program_address(
                &[light_sdk::constants::CPI_AUTHORITY_PDA_SEED],
                &light_compressed_token::ID,
            )
            .0,
        )
        .into(), // SPL mint always has CPI authority as mint authority
        supply: 0, // Started with 0
        decimals,
        is_initialized: true, // Is initialized after creation
        freeze_authority: Some(freeze_authority.pubkey().into()).into(),
    };

    assert_mint_to_compressed(
        &mut rpc,
        spl_mint_pda,
        &expected_recipients,
        Some(empty_token_pool),
        empty_pre_compressed_mint,
        Some(empty_spl_mint),
    )
    .await;

    // 3. Verify authority updates
    let updated_compressed_mint_account = rpc
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

    // Authority update assertions
    assert_eq!(
        updated_compressed_mint.mint_authority.unwrap(),
        new_mint_authority.pubkey(),
        "Mint authority should be updated"
    );
    assert_eq!(
        updated_compressed_mint.supply, total_mint_amount,
        "Supply should match minted amount"
    );
    assert!(
        updated_compressed_mint.is_decompressed,
        "Mint should be decompressed after CreateSplMint"
    );

    println!("✅ Comprehensive mint action test passed!");

    // === TEST 2: MINT_ACTION ON EXISTING MINT ===
    // Now test mint_action on the existing mint (no creation, just minting and authority updates)

    println!("\n=== Testing mint_action on existing mint ===");

    // Get current mint state for input
    let current_compressed_mint_account = rpc
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value;
    let current_compressed_mint: CompressedMint = BorshDeserialize::deserialize(
        &mut current_compressed_mint_account
            .data
            .unwrap()
            .data
            .as_slice(),
    )
    .unwrap();

    // Create another new authority to test second update
    let newer_mint_authority = Keypair::new();

    // Fund both the current authority (new_mint_authority) and newer authority
    rpc.airdrop_lamports(&new_mint_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    rpc.airdrop_lamports(&newer_mint_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // Additional recipients for second minting
    let additional_recipients = vec![
        light_ctoken_types::instructions::mint_to_compressed::Recipient {
            recipient: Keypair::new().pubkey().to_bytes().into(),
            amount: 5000u64,
        },
        light_ctoken_types::instructions::mint_to_compressed::Recipient {
            recipient: Keypair::new().pubkey().to_bytes().into(),
            amount: 2500u64,
        },
    ];
    let additional_mint_amount = 7500u64;
    // Token pool should have previous amount
    let (token_pool_pda, _) =
        light_compressed_token::instructions::create_token_pool::find_token_pool_pda_with_index(
            &spl_mint_pda,
            0,
        );
    let pre_pool_data = rpc.get_account(token_pool_pda).await.unwrap().unwrap();
    let pre_token_pool_for_second =
        spl_token_2022::state::Account::unpack(&pre_pool_data.data).unwrap();

    let pre_spl_mint_data = rpc.get_account(spl_mint_pda).await.unwrap().unwrap();
    let pre_spl_mint_for_second =
        spl_token_2022::state::Mint::unpack(&pre_spl_mint_data.data).unwrap();
    // Execute mint_action on existing mint (no creation)
    let signature2 = light_token_client::actions::mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &new_mint_authority, // Current authority from first test (now the authority for this mint)
        &payer,
        false,                               // create_spl_mint = false (already exists)
        additional_recipients.clone(),       // mint_to_recipients
        Some(newer_mint_authority.pubkey()), // update_mint_authority to newer authority
        None,                                // update_freeze_authority (no change)
        None,                                // no lamports
        None,                                // no new mint data (already exists)
    )
    .await
    .unwrap();

    println!("Second mint action transaction signature: {}", signature2);

    // Verify results of second mint action
    let expected_additional_recipients: Vec<Recipient> = additional_recipients.clone();

    // Create pre-states for the second action (current state after first action)
    let mut pre_compressed_mint_for_second = current_compressed_mint.clone();
    pre_compressed_mint_for_second.mint_authority = Some(newer_mint_authority.pubkey().into());

    // Verify second minting using assertion helper
    assert_mint_to_compressed(
        &mut rpc,
        spl_mint_pda,
        &expected_additional_recipients,
        Some(pre_token_pool_for_second),
        pre_compressed_mint_for_second,
        Some(pre_spl_mint_for_second),
    )
    .await;

    // Verify final authority update
    let final_compressed_mint_account = rpc
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value;
    let final_compressed_mint: CompressedMint = BorshDeserialize::deserialize(
        &mut final_compressed_mint_account.data.unwrap().data.as_slice(),
    )
    .unwrap();

    // Final assertions
    assert_eq!(
        final_compressed_mint.mint_authority.unwrap(),
        newer_mint_authority.pubkey(),
        "Mint authority should be updated to newer authority"
    );
    assert_eq!(
        final_compressed_mint.supply,
        total_mint_amount + additional_mint_amount,
        "Supply should include both mintings"
    );
    assert!(
        final_compressed_mint.is_decompressed,
        "Mint should remain decompressed"
    );

    println!("✅ Existing mint test passed!");
    println!("✅ All comprehensive mint action tests passed!");
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
            Some(pre_token_pool_account), // Pass pre-token pool account for decompressed mint validation
            pre_compressed_mint,
            Some(pre_spl_mint),
        )
        .await;
    }
}

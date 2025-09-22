// #![cfg(feature = "test-sbf")]

use anchor_lang::{prelude::borsh::BorshDeserialize, solana_program::program_pack::Pack};
use anchor_spl::token_2022::spl_token_2022;
use light_batched_merkle_tree::initialize_state_tree::InitStateTreeAccountsInstructionData;
use light_client::indexer::Indexer;
use light_compressed_token_sdk::instructions::{
    create_associated_token_account::{
        create_associated_token_account, create_compressible_associated_token_account,
        CreateCompressibleAssociatedTokenAccountInputs,
    },
    derive_compressed_mint_address, derive_ctoken_ata, find_spl_mint_address,
};
use light_ctoken_types::{
    instructions::{
        extensions::token_metadata::TokenMetadataInstructionData, mint_action::Recipient,
    },
    state::{
        extensions::AdditionalMetadata, BaseMint, CompressedMint, CompressedMintMetadata,
        TokenDataVersion,
    },
    COMPRESSED_MINT_SEED,
};
use light_program_test::{utils::assert::assert_rpc_error, LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    assert_ctoken_transfer::assert_ctoken_transfer,
    assert_mint_action::assert_mint_action,
    assert_mint_to_compressed::{assert_mint_to_compressed, assert_mint_to_compressed_one},
    assert_transfer2::{
        assert_transfer2, assert_transfer2_compress, assert_transfer2_decompress,
        assert_transfer2_transfer,
    },
    mint_assert::assert_compressed_mint_account,
    Rpc,
};
use light_token_client::{
    actions::{create_mint, ctoken_transfer, mint_to_compressed, transfer2},
    instructions::transfer2::{
        create_decompress_instruction, create_generic_transfer2_instruction, CompressInput,
        DecompressInput, Transfer2InstructionType, TransferInput,
    },
};
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

/// 1. Create compressed mint (no metadata)
/// 2. Mint tokens with compressed mint
/// 3. Transfer compressed tokens to new recipient
/// 4. Decompress compressed tokens to SPL tokens
/// 5. Compress SPL tokens to compressed tokens
/// 6. Multi-operation transaction (transfer + decompress + compress)
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
            &mint_authority_keypair,
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

    // Use our mint_to_compressed action helper
    {
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

        mint_to_compressed(
            &mut rpc,
            spl_mint_pda,
            vec![Recipient {
                recipient: recipient.into(),
                amount: mint_amount,
            }],
            TokenDataVersion::V2,
            &mint_authority_keypair,
            &payer,
        )
        .await
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
    // // 3. Create SPL mint from compressed mint
    // // Get compressed mint data before creating SPL mint
    // {
    //     let pre_compressed_mint_account = rpc
    //         .indexer()
    //         .unwrap()
    //         .get_compressed_account(compressed_mint_address, None)
    //         .await
    //         .unwrap()
    //         .value;
    //     let pre_compressed_mint: CompressedMint = BorshDeserialize::deserialize(
    //         &mut pre_compressed_mint_account.data.unwrap().data.as_slice(),
    //     )
    //     .unwrap();

    //     // Use our create_spl_mint action helper (automatically handles proofs, PDAs, and transaction)
    //     create_spl_mint(
    //         &mut rpc,
    //         compressed_mint_address,
    //         &mint_seed,
    //         &mint_authority_keypair,
    //         &payer,
    //     )
    //     .await
    //     .unwrap();

    //     // Verify SPL mint was created using our assertion helper
    //     assert_spl_mint(&mut rpc, mint_seed.pubkey(), &pre_compressed_mint).await;
    // }

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
            compressed_token_account: compressed_token_accounts,
            to: new_recipient,
            amount: transfer_amount,
            is_delegate_transfer: false,
            mint: None,
            change_amount: None,
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

    // 5. Decompress compressed tokens to ctokens
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
            // Use comprehensive decompress assertion
            assert_transfer2_decompress(
                &mut rpc,
                light_token_client::instructions::transfer2::DecompressInput {
                    compressed_token_account: vec![compressed_token_account.clone()],
                    decompress_amount,
                    solana_token_account: ctoken_ata_pubkey,
                    amount: decompress_amount,
                },
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

        let instruction_actions = vec![
            // 1. Transfer compressed tokens to a new recipient
            Transfer2InstructionType::Transfer(TransferInput {
                compressed_token_account: remaining_compressed_tokens.clone(),
                to: transfer_recipient.pubkey(),
                amount: transfer_amount,
                is_delegate_transfer: false,
                mint: None,
                change_amount: None,
            }),
            // 2. Decompress some compressed tokens to SPL tokens
            Transfer2InstructionType::Decompress(DecompressInput {
                compressed_token_account: compressed_tokens_for_compress.clone(),
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

        assert_transfer2(&mut rpc, instruction_actions).await;
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
    create_mint(
        &mut rpc,
        &mint_seed,
        8, // decimals
        &initial_mint_authority,
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
        compressed_mint.base.mint_authority.unwrap(),
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
        compressed_mint.base.freeze_authority.unwrap(),
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
}

/// Test decompressed token transfer with mint action creating tokens in decompressed account
#[tokio::test]
#[serial]
async fn test_ctoken_transfer() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Test parameters
    let decimals = 8u8;
    let mint_seed = Keypair::new();
    let mint_authority = payer.insecure_clone(); // Use payer as mint authority to avoid KeypairPubkeyMismatch
    let freeze_authority = Keypair::new();

    // Create recipient for decompressed tokens
    let recipient_keypair = Keypair::new();
    let transfer_amount = 500u64;

    // Fund authority accounts
    rpc.airdrop_lamports(&mint_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    rpc.airdrop_lamports(&freeze_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    rpc.airdrop_lamports(&recipient_keypair.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // Derive addresses
    let (spl_mint_pda, _) = find_spl_mint_address(&mint_seed.pubkey());

    // Create compressed token ATA for recipient
    let (recipient_ata, _) = derive_ctoken_ata(&recipient_keypair.pubkey(), &spl_mint_pda);
    let create_ata_instruction = create_compressible_associated_token_account(
        CreateCompressibleAssociatedTokenAccountInputs {
            payer: payer.pubkey(),
            owner: recipient_keypair.pubkey(),
            mint: spl_mint_pda,
            rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
            pre_pay_num_epochs: 1,
            lamports_per_write: Some(1000),
            compressible_config: rpc
                .test_accounts
                .funding_pool_config
                .compressible_config_pda,
            token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
        },
    )
    .unwrap();
    rpc.create_and_send_transaction(&[create_ata_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // === STEP 1: CREATE COMPRESSED MINT AND MINT TO DECOMPRESSED ACCOUNT ===
    let decompressed_recipients = vec![Recipient {
        recipient: recipient_keypair.pubkey().to_bytes().into(),
        amount: 100000000u64,
    }];

    let signature = light_token_client::actions::mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &mint_authority,
        &payer,
        vec![],                  // no compressed recipients
        decompressed_recipients, // mint to decompressed recipients
        None,                    // no mint authority update
        None,                    // no freeze authority update
        Some(light_token_client::instructions::mint_action::NewMint {
            decimals,
            supply: 0,
            mint_authority: mint_authority.pubkey(),
            freeze_authority: Some(freeze_authority.pubkey()),
            metadata: None, // No metadata for simplicity
            version: 3,
        }),
    )
    .await
    .unwrap();

    println!(
        "✅ Mint creation and decompressed minting signature: {}",
        signature
    );

    // Verify the recipient ATA has the tokens (should have been minted by the mint action)
    let recipient_account_data = rpc.get_account(recipient_ata).await.unwrap().unwrap();
    let recipient_account =
        spl_token_2022::state::Account::unpack(&recipient_account_data.data[..165]).unwrap();
    println!("Recipient account balance: {}", recipient_account.amount);
    assert_eq!(
        recipient_account.amount, 100000000u64,
        "Recipient should have 100000000u64 tokens"
    );

    // === CREATE SECOND RECIPIENT FOR TRANSFER TEST ===
    let second_recipient_keypair = Keypair::new();
    let (second_recipient_ata, _) =
        derive_ctoken_ata(&second_recipient_keypair.pubkey(), &spl_mint_pda);

    rpc.airdrop_lamports(&second_recipient_keypair.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let create_second_ata_instruction = create_associated_token_account(
        payer.pubkey(),
        second_recipient_keypair.pubkey(),
        spl_mint_pda,
    )
    .unwrap();
    rpc.create_and_send_transaction(&[create_second_ata_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // === PERFORM DECOMPRESSED TOKEN TRANSFER ===
    // Get account states before transfer
    let sender_account_data = rpc.get_account(recipient_ata).await.unwrap().unwrap();
    let sender_account_before =
        spl_token_2022::state::Account::unpack(&sender_account_data.data[..165]).unwrap();

    let recipient_account_data = rpc
        .get_account(second_recipient_ata)
        .await
        .unwrap()
        .unwrap();
    let recipient_account_before =
        spl_token_2022::state::Account::unpack(&recipient_account_data.data[..165]).unwrap();

    println!(
        "Sender balance before transfer: {}",
        sender_account_before.amount
    );
    println!(
        "Recipient balance before transfer: {}",
        recipient_account_before.amount
    );
    rpc.context.warp_to_slot(2);
    // Execute the decompressed transfer
    let transfer_result = ctoken_transfer(
        &mut rpc,
        recipient_ata,        // Source account (has 1000 tokens)
        second_recipient_ata, // Destination account
        transfer_amount,      // Amount to transfer (500)
        &recipient_keypair,   // Authority/owner
        &payer,               // Transaction payer
    )
    .await;

    match transfer_result {
        Ok(signature) => {
            println!(
                "✅ Decompressed token transfer successful! Signature: {}",
                signature
            );

            // Use comprehensive assertion helper
            assert_ctoken_transfer(
                &mut rpc,
                recipient_ata,
                second_recipient_ata,
                transfer_amount,
            )
            .await;
        }
        Err(e) => {
            panic!("❌ Decompressed token transfer failed: {:?}", e);
        }
    }

    // === COMPRESS TOKENS BACK TO COMPRESSED STATE ===
    println!("🔄 Compressing tokens back to compressed state...");

    // Create a compress recipient
    let compress_recipient = Keypair::new();
    let compress_amount = 200u64; // Compress 200 tokens from second_recipient_ata (which now has 500)

    // Get output queue
    let output_queue = rpc
        .get_random_state_tree_info()
        .unwrap()
        .get_output_pubkey()
        .unwrap();

    // Create compress instruction
    let compress_instruction = create_generic_transfer2_instruction(
        &mut rpc,
        vec![Transfer2InstructionType::Compress(CompressInput {
            compressed_token_account: None, // No existing compressed tokens
            solana_token_account: second_recipient_ata, // Source SPL token account
            to: compress_recipient.pubkey(), // New recipient for compressed tokens
            mint: spl_mint_pda,
            amount: compress_amount,
            authority: second_recipient_keypair.pubkey(), // Authority for compression
            output_queue,
        })],
        payer.pubkey(),
    )
    .await
    .unwrap();

    // Get account state before compression for assertion
    let pre_compress_account_data = rpc
        .get_account(second_recipient_ata)
        .await
        .unwrap()
        .unwrap();
    let pre_compress_spl_account =
        spl_token_2022::state::Account::unpack(&pre_compress_account_data.data).unwrap();
    println!(
        "Account balance before compression: {}",
        pre_compress_spl_account.amount
    );

    // Execute compression
    let compress_signature = rpc
        .create_and_send_transaction(
            &[compress_instruction],
            &payer.pubkey(),
            &[&payer, &second_recipient_keypair],
        )
        .await
        .unwrap();

    println!(
        "✅ Compression successful! Signature: {}",
        compress_signature
    );

    // Use comprehensive compress assertion
    assert_transfer2_compress(
        &mut rpc,
        light_token_client::instructions::transfer2::CompressInput {
            compressed_token_account: None,
            solana_token_account: second_recipient_ata,
            to: compress_recipient.pubkey(),
            mint: spl_mint_pda,
            amount: compress_amount,
            authority: second_recipient_keypair.pubkey(),
            output_queue,
        },
    )
    .await;

    // Verify final balances
    let final_account_data = rpc
        .get_account(second_recipient_ata)
        .await
        .unwrap()
        .unwrap();
    let final_spl_account =
        spl_token_2022::state::Account::unpack(&final_account_data.data).unwrap();
    println!(
        "Final account balance after compression: {}",
        final_spl_account.amount
    );
    assert_eq!(
        final_spl_account.amount, 300,
        "Should have 300 tokens remaining (500 - 200)"
    );

    // Check that compressed tokens were created for the recipient
    let compressed_tokens = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&compress_recipient.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert!(
        !compressed_tokens.is_empty(),
        "Should have compressed tokens"
    );
    let total_compressed = compressed_tokens
        .iter()
        .map(|t| t.token.amount)
        .sum::<u64>();
    assert_eq!(
        total_compressed, compress_amount,
        "Should have {} compressed tokens",
        compress_amount
    );

    println!(
        "✅ Complete decompressed token transfer and compression test completed successfully!"
    );
}

/// Test comprehensive mint actions in a single instruction
#[tokio::test]
#[serial]
async fn test_mint_actions() {
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
        Recipient {
            recipient: Keypair::new().pubkey().to_bytes().into(),
            amount: 1000u64,
        },
        Recipient {
            recipient: Keypair::new().pubkey().to_bytes().into(),
            amount: 2000u64,
        },
        Recipient {
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
    rpc.context.warp_to_slot(1);
    // === SINGLE MINT ACTION INSTRUCTION ===
    // Execute ONE instruction with ALL actions
    let signature = light_token_client::actions::mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &mint_authority,
        &payer,
        recipients.clone(),                  // mint_to_recipients
        vec![],                              // mint_to_decompressed_recipients
        Some(new_mint_authority.pubkey()),   // update_mint_authority
       None,// Some(new_freeze_authority.pubkey()), // update_freeze_authority
        Some(light_token_client::instructions::mint_action::NewMint {
            decimals,
            supply:0,
            mint_authority: mint_authority.pubkey(),
            freeze_authority: Some(freeze_authority.pubkey()),
            metadata: Some(light_ctoken_types::instructions::extensions::token_metadata::TokenMetadataInstructionData {
                update_authority: Some(mint_authority.pubkey().into()),
                name: "Test Token".as_bytes().to_vec(),
                symbol: "TEST".as_bytes().to_vec(),
                uri: "https://example.com/token.json".as_bytes().to_vec(),
                additional_metadata: None,
            }),
            version: 3,
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
        base: BaseMint {
            mint_authority: Some(new_mint_authority.pubkey().into()),
            supply: 0,
            decimals,
            is_initialized: true,
            freeze_authority: Some(freeze_authority.pubkey().into()), // We didn't update freeze authority
        },
        metadata: CompressedMintMetadata {
            version: 3, // With metadata
            spl_mint: spl_mint_pda.into(),
            spl_mint_initialized: false, // Should be true after CreateSplMint action
        },
        extensions: Some(vec![
            light_ctoken_types::state::extensions::ExtensionStruct::TokenMetadata(
                light_ctoken_types::state::extensions::TokenMetadata {
                    update_authority: mint_authority.pubkey().into(), // Original authority in metadata
                    mint: spl_mint_pda.into(),
                    name: "Test Token".as_bytes().to_vec(),
                    symbol: "TEST".as_bytes().to_vec(),
                    uri: "https://example.com/token.json".as_bytes().to_vec(),
                    additional_metadata: vec![], // No additional metadata in our test
                },
            ),
        ]), // Match the metadata we're creating
    };

    assert_mint_to_compressed(
        &mut rpc,
        spl_mint_pda,
        &expected_recipients,
        None,
        empty_pre_compressed_mint,
        None,
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
        updated_compressed_mint.base.mint_authority.unwrap(),
        new_mint_authority.pubkey(),
        "Mint authority should be updated"
    );
    assert_eq!(
        updated_compressed_mint.base.supply, total_mint_amount,
        "Supply should match minted amount"
    );
    assert!(
        !updated_compressed_mint.metadata.spl_mint_initialized,
        "Mint should not be decompressed "
    );

    // === TEST 2: MINT_ACTION ON EXISTING MINT ===
    // Now test mint_action on the existing mint (no creation, just minting and authority updates)

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
        Recipient {
            recipient: Keypair::new().pubkey().to_bytes().into(),
            amount: 5000u64,
        },
        Recipient {
            recipient: Keypair::new().pubkey().to_bytes().into(),
            amount: 2500u64,
        },
    ];
    let additional_mint_amount = 7500u64;
    rpc.context.warp_to_slot(3);
    // Execute mint_action on existing mint (no creation)
    let signature2 = light_token_client::actions::mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &new_mint_authority, // Current authority from first test (now the authority for this mint)
        &payer,
        additional_recipients.clone(),       // mint_to_recipients
        vec![],                              // mint_to_decompressed_recipients
        Some(newer_mint_authority.pubkey()), // update_mint_authority to newer authority
        None,                                // update_freeze_authority (no change)
        None,                                // no new mint data (already exists)
    )
    .await
    .unwrap();

    println!("Second mint action transaction signature: {}", signature2);

    // Verify results of second mint action
    let expected_additional_recipients: Vec<Recipient> = additional_recipients.clone();

    // Create pre-states for the second action (current state after first action)
    let mut pre_compressed_mint_for_second = current_compressed_mint.clone();
    pre_compressed_mint_for_second.base.mint_authority = Some(newer_mint_authority.pubkey().into());

    // Verify second minting using assertion helper
    assert_mint_to_compressed(
        &mut rpc,
        spl_mint_pda,
        &expected_additional_recipients,
        None,
        pre_compressed_mint_for_second,
        None,
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
        final_compressed_mint.base.mint_authority.unwrap(),
        newer_mint_authority.pubkey(),
        "Mint authority should be updated to newer authority"
    );
    assert_eq!(
        final_compressed_mint.base.supply,
        total_mint_amount + additional_mint_amount,
        "Supply should include both mintings"
    );
    assert!(
        !final_compressed_mint.metadata.spl_mint_initialized,
        "Mint should remain compressed"
    );
}

#[tokio::test]
#[serial]
async fn test_create_compressed_mint_with_token_metadata() {
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
        name: b"Test Token".to_vec(),
        symbol: b"TEST".to_vec(),
        uri: b"https://example.com/token.json".to_vec(),
        additional_metadata: Some(additional_metadata.clone()),
    };
    light_token_client::actions::create_mint(
        &mut rpc,
        &mint_seed,
        decimals,
        &mint_authority_keypair,
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

    // 2. Mint to compressed
    {
        let mint_amount = 100_000u64; // Mint 100,000 tokens
        let recipient_keypair = Keypair::new();
        let recipient = recipient_keypair.pubkey();

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

        // Use our mint_to_compressed action helper (automatically handles decompressed mint config)
        mint_to_compressed(
            &mut rpc,
            spl_mint_pda,
            vec![Recipient {
                recipient: recipient.into(),
                amount: mint_amount,
            }],
            TokenDataVersion::ShaFlat,
            &mint_authority_keypair,
            &payer,
        )
        .await
        .unwrap();

        // Verify minted tokens using our assertion helper
        assert_mint_to_compressed_one(
            &mut rpc,
            spl_mint_pda,
            recipient,
            mint_amount,
            None, // Pass pre-token pool account for decompressed mint validation
            pre_compressed_mint,
            None,
        )
        .await;
    }
}

/// Functional and Failing tests:
/// 1. FAIL - MintToCompressed - invalid mint authority
/// 2. SUCCEED - MintToCompressed
/// 3. FAIL - UpdateMintAuthority - invalid mint authority
/// 4. SUCCEED - UpdateMintAuthority
/// 5. FAIL - UpdateFreezeAuthority - invalid freeze authority
/// 6. SUCCEED - UpdateFreezeAuthority
/// 7. FAIL - MintToCToken - invalid mint authority
/// 8. SUCCEED - MintToCToken
/// 9. FAIL - UpdateMetadataField - invalid metadata authority
/// 10. SUCCEED - UpdateMetadataField
/// 11. FAIL - UpdateMetadataAuthority  - invalid metadata authority
/// 12. SUCCEED - UpdateMetadataAuthority
/// 13. FAIL -  RemoveMetadataKey  - invalid metadata authority
/// 14. SUCCEED - RemoveMetadataKey
/// 15. SUCCEED - RemoveMetadataKey - idempotent
#[tokio::test]
#[serial]
async fn functional_and_failing_tests() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();

    let payer = Keypair::new();
    rpc.airdrop_lamports(&payer.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let mint_seed = Keypair::new();
    let mint_authority = Keypair::new();
    let freeze_authority = Keypair::new();
    let metadata_authority = Keypair::new();
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    // Derive compressed mint address for verification
    let compressed_mint_address =
        derive_compressed_mint_address(&mint_seed.pubkey(), &address_tree_pubkey);

    // Find mint PDA for the rest of the test
    let (spl_mint_pda, _) = find_spl_mint_address(&mint_seed.pubkey());
    // 1. Create compressed mint with both authorities
    {
        create_mint(
        &mut rpc,
        &mint_seed,
        8, // decimals
        &mint_authority,
        Some(freeze_authority.pubkey()),
        Some(light_ctoken_types::instructions::extensions::token_metadata::TokenMetadataInstructionData {
            update_authority: Some(metadata_authority.pubkey().into()),
            name: "Test Token".as_bytes().to_vec(),
            symbol: "TEST".as_bytes().to_vec(),
            uri: "https://example.com/token.json".as_bytes().to_vec(),
            additional_metadata: Some(vec![AdditionalMetadata {
                key: vec![1,2,3,4],
                value: vec![2u8;5]
            }]),
        }),
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
        8,
        mint_authority.pubkey(),
        freeze_authority.pubkey(),
        Some(light_ctoken_types::instructions::extensions::token_metadata::TokenMetadataInstructionData {
            update_authority: Some(metadata_authority.pubkey().into()),
            name: "Test Token".as_bytes().to_vec(),
            symbol: "TEST".as_bytes().to_vec(),
            uri: "https://example.com/token.json".as_bytes().to_vec(),
            additional_metadata: Some(vec![AdditionalMetadata {
                key: vec![1,2,3,4],
                value: vec![2u8;5]
            }]),
        }), // No metadata
    );
    }

    // Create invalid authorities for testing
    let invalid_mint_authority = Keypair::new();
    let invalid_freeze_authority = Keypair::new();
    let invalid_metadata_authority = Keypair::new();

    // Create new authorities for updates
    let new_mint_authority = Keypair::new();
    let new_freeze_authority = Keypair::new();
    let new_metadata_authority = Keypair::new();

    // Fund invalid authorities
    rpc.airdrop_lamports(&invalid_mint_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    rpc.airdrop_lamports(&invalid_freeze_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    rpc.airdrop_lamports(&invalid_metadata_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // Fund new authorities
    rpc.airdrop_lamports(&new_mint_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    rpc.airdrop_lamports(&new_freeze_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    rpc.airdrop_lamports(&new_metadata_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // 1. MintToCompressed with invalid mint authority
    {
        let result = light_token_client::actions::mint_to_compressed(
            &mut rpc,
            spl_mint_pda,
            vec![light_ctoken_types::instructions::mint_action::Recipient {
                recipient: Keypair::new().pubkey().to_bytes().into(),
                amount: 1000u64,
            }],
            light_ctoken_types::state::TokenDataVersion::V2,
            &invalid_mint_authority, // Invalid authority
            &payer,
        )
        .await;

        assert_rpc_error(
            result, 0, 18, // light_compressed_token::ErrorCode::InvalidAuthorityMint.into(),
        )
        .unwrap();
    }

    // 2. SUCCEED - MintToCompressed with valid mint authority
    {
        // Get pre-transaction compressed mint state
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

        let recipient = Keypair::new().pubkey().to_bytes().into();
        let result = light_token_client::actions::mint_to_compressed(
            &mut rpc,
            spl_mint_pda,
            vec![light_ctoken_types::instructions::mint_action::Recipient {
                recipient,
                amount: 1000u64,
            }],
            light_ctoken_types::state::TokenDataVersion::V2,
            &mint_authority, // Valid authority
            &payer,
        )
        .await;

        assert!(result.is_ok(), "Should succeed with valid mint authority");

        // Verify using assert_mint_action
        assert_mint_action(
            &mut rpc,
            compressed_mint_address,
            pre_compressed_mint,
            vec![
                light_compressed_token_sdk::instructions::mint_action::MintActionType::MintTo {
                    recipients: vec![
                        light_compressed_token_sdk::instructions::mint_action::MintToRecipient {
                            recipient: recipient.into(),
                            amount: 1000u64,
                        },
                    ],
                    token_account_version: light_ctoken_types::state::TokenDataVersion::V2 as u8,
                },
            ],
        )
        .await;
    }

    // Get compressed mint account for update operations
    let compressed_mint_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value;

    // 2. UpdateMintAuthority with invalid mint authority
    {
        let result = light_token_client::actions::update_mint_authority(
            &mut rpc,
            &invalid_mint_authority, // Invalid authority
            Some(Keypair::new().pubkey()),
            compressed_mint_account.hash,
            compressed_mint_account.leaf_index,
            compressed_mint_account.tree_info.tree,
            &payer,
        )
        .await;

        assert_rpc_error(
            result, 0, 18, // light_compressed_token::ErrorCode::InvalidAuthorityMint.into(),
        )
        .unwrap();
    }

    // 4. SUCCEED - UpdateMintAuthority with valid mint authority
    {
        // Get fresh compressed mint account
        let compressed_mint_account = rpc
            .indexer()
            .unwrap()
            .get_compressed_account(compressed_mint_address, None)
            .await
            .unwrap()
            .value;
        let pre_compressed_mint: CompressedMint = BorshDeserialize::deserialize(
            &mut compressed_mint_account.data.unwrap().data.as_slice(),
        )
        .unwrap();

        let result = light_token_client::actions::update_mint_authority(
            &mut rpc,
            &mint_authority, // Valid current authority
            Some(new_mint_authority.pubkey()),
            compressed_mint_account.hash,
            compressed_mint_account.leaf_index,
            compressed_mint_account.tree_info.tree,
            &payer,
        )
        .await;

        assert!(result.is_ok(), "Should succeed with valid mint authority");

        // Verify using assert_mint_action
        assert_mint_action(
            &mut rpc,
            compressed_mint_address,
            pre_compressed_mint,
            vec![light_compressed_token_sdk::instructions::mint_action::MintActionType::UpdateMintAuthority {
                new_authority: Some(new_mint_authority.pubkey()),
            }],
        )
        .await;
    }

    // 5. UpdateFreezeAuthority with invalid freeze authority
    {
        // Get fresh compressed mint account after mint authority update
        let compressed_mint_account = rpc
            .indexer()
            .unwrap()
            .get_compressed_account(compressed_mint_address, None)
            .await
            .unwrap()
            .value;

        let result = light_token_client::actions::update_freeze_authority(
            &mut rpc,
            &invalid_freeze_authority, // Invalid authority
            Some(Keypair::new().pubkey()),
            new_mint_authority.pubkey(), // Must pass the NEW mint authority after update
            compressed_mint_account.hash,
            compressed_mint_account.leaf_index,
            compressed_mint_account.tree_info.tree,
            &payer,
        )
        .await;

        assert_rpc_error(
            result, 0,
            18, // InvalidAuthorityMint error code (authority validation always returns 18)
        )
        .unwrap();
    }

    // 6. SUCCEED - UpdateFreezeAuthority with valid freeze authority
    {
        // Get fresh compressed mint account
        let compressed_mint_account = rpc
            .indexer()
            .unwrap()
            .get_compressed_account(compressed_mint_address, None)
            .await
            .unwrap()
            .value;
        let pre_compressed_mint: CompressedMint = BorshDeserialize::deserialize(
            &mut compressed_mint_account.data.unwrap().data.as_slice(),
        )
        .unwrap();

        let result = light_token_client::actions::update_freeze_authority(
            &mut rpc,
            &freeze_authority, // Valid current freeze authority
            Some(new_freeze_authority.pubkey()),
            new_mint_authority.pubkey(), // Pass the updated mint authority
            compressed_mint_account.hash,
            compressed_mint_account.leaf_index,
            compressed_mint_account.tree_info.tree,
            &payer,
        )
        .await;

        assert!(result.is_ok(), "Should succeed with valid freeze authority");

        // Verify using assert_mint_action
        assert_mint_action(
            &mut rpc,
            compressed_mint_address,
            pre_compressed_mint,
            vec![light_compressed_token_sdk::instructions::mint_action::MintActionType::UpdateFreezeAuthority {
                new_authority: Some(new_freeze_authority.pubkey()),
            }],
        )
        .await;
    }

    // 7. MintToCToken with invalid mint authority
    {
        // Create a ctoken account first
        let recipient = Keypair::new();

        let create_ata_ix =
            light_compressed_token_sdk::instructions::create_associated_token_account(
                payer.pubkey(),
                recipient.pubkey(),
                spl_mint_pda,
            )
            .unwrap();

        rpc.create_and_send_transaction(&[create_ata_ix], &payer.pubkey(), &[&payer])
            .await
            .unwrap();

        // Try to mint with invalid authority
        let result = light_token_client::actions::mint_action_comprehensive(
            &mut rpc,
            &mint_seed,
            &invalid_mint_authority, // Invalid authority
            &payer,
            vec![], // No compressed recipients
            vec![light_ctoken_types::instructions::mint_action::Recipient {
                recipient: recipient.pubkey().to_bytes().into(),
                amount: 1000u64,
            }], // Mint to decompressed
            None,   // No mint authority update
            None,   // No freeze authority update
            None,   // Not creating new mint
        )
        .await;

        assert_rpc_error(
            result, 0,
            18, //    light_compressed_token::ErrorCode::InvalidAuthorityMint.into(),
        )
        .unwrap();
    }

    // 8. SUCCEED - MintToCToken with valid mint authority
    {
        // Get pre-transaction compressed mint state
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

        // Create a new recipient for successful mint
        let recipient2 = Keypair::new();

        let create_ata_ix2 =
            light_compressed_token_sdk::instructions::create_associated_token_account(
                payer.pubkey(),
                recipient2.pubkey(),
                spl_mint_pda,
            )
            .unwrap();

        rpc.create_and_send_transaction(&[create_ata_ix2], &payer.pubkey(), &[&payer])
            .await
            .unwrap();

        let recipient_ata = light_compressed_token_sdk::instructions::derive_ctoken_ata(
            &recipient2.pubkey(),
            &spl_mint_pda,
        )
        .0;

        // Try to mint with valid NEW authority (since we updated it)
        let result = light_token_client::actions::mint_action_comprehensive(
            &mut rpc,
            &mint_seed,
            &new_mint_authority, // Valid NEW authority after update
            &payer,
            vec![], // No compressed recipients
            vec![light_ctoken_types::instructions::mint_action::Recipient {
                recipient: recipient2.pubkey().to_bytes().into(),
                amount: 2000u64,
            }], // Mint to decompressed
            None,   // No mint authority update
            None,   // No freeze authority update
            None,   // Not creating new mint
        )
        .await;

        assert!(result.is_ok(), "Should succeed with valid mint authority");

        // Verify using assert_mint_action
        assert_mint_action(
            &mut rpc,
            compressed_mint_address,
            pre_compressed_mint,
            vec![light_compressed_token_sdk::instructions::mint_action::MintActionType::MintToCToken {
                account: recipient_ata,
                amount: 2000u64,
            }],
        )
        .await;
    }

    // 9. UpdateMetadataField with invalid metadata authority
    {
        let result = light_token_client::actions::mint_action(
            &mut rpc,
            light_token_client::instructions::mint_action::MintActionParams {
                compressed_mint_address,
                mint_seed: mint_seed.pubkey(),
                authority: invalid_metadata_authority.pubkey(), // Invalid authority
                payer: payer.pubkey(),
                actions: vec![light_compressed_token_sdk::instructions::mint_action::MintActionType::UpdateMetadataField {
                    extension_index: 0,
                    field_type: 0, // 0 = Name field
                    key: vec![],   // Empty for Name field
                    value: "New Name".as_bytes().to_vec(),
                }],
                new_mint: None,
            },
            &invalid_metadata_authority,
            &payer,
            None,
        )
        .await;

        assert_rpc_error(
            result, 0, 18, // light_compressed_token::ErrorCode::InvalidAuthorityMint.into(),
        )
        .unwrap();
    }

    // 10. SUCCEED - UpdateMetadataField with valid metadata authority
    {
        // Get pre-transaction compressed mint state
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

        let actions = vec![light_compressed_token_sdk::instructions::mint_action::MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 0, // 0 = Name field
            key: vec![],   // Empty for Name field
            value: "Updated Token Name".as_bytes().to_vec(),
        }];

        let result = light_token_client::actions::mint_action(
            &mut rpc,
            light_token_client::instructions::mint_action::MintActionParams {
                compressed_mint_address,
                mint_seed: mint_seed.pubkey(),
                authority: metadata_authority.pubkey(), // Valid metadata authority
                payer: payer.pubkey(),
                actions: actions.clone(),
                new_mint: None,
            },
            &metadata_authority,
            &payer,
            None,
        )
        .await;

        assert!(
            result.is_ok(),
            "Should succeed with valid metadata authority"
        );

        // Verify using assert_mint_action
        assert_mint_action(
            &mut rpc,
            compressed_mint_address,
            pre_compressed_mint,
            actions,
        )
        .await;
    }

    // 11. UpdateMetadataAuthority with invalid metadata authority
    {
        let result = light_token_client::actions::mint_action(
            &mut rpc,
            light_token_client::instructions::mint_action::MintActionParams {
                compressed_mint_address,
                mint_seed: mint_seed.pubkey(),
                authority: invalid_metadata_authority.pubkey(), // Invalid authority
                payer: payer.pubkey(),
                actions: vec![light_compressed_token_sdk::instructions::mint_action::MintActionType::UpdateMetadataAuthority {
                    extension_index: 0,
                    new_authority: Keypair::new().pubkey(),
                }],
                new_mint: None,
            },
            &invalid_metadata_authority,
            &payer,
            None,
        )
        .await;

        assert_rpc_error(
            result, 0, 18, // light_compressed_token::ErrorCode::InvalidAuthorityMint.into(),
        )
        .unwrap();
    }

    // 12. SUCCEED - UpdateMetadataAuthority with valid metadata authority
    {
        // Get pre-transaction compressed mint state
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

        let actions = vec![light_compressed_token_sdk::instructions::mint_action::MintActionType::UpdateMetadataAuthority {
            extension_index: 0,
            new_authority: new_metadata_authority.pubkey(),
        }];

        let result = light_token_client::actions::mint_action(
            &mut rpc,
            light_token_client::instructions::mint_action::MintActionParams {
                compressed_mint_address,
                mint_seed: mint_seed.pubkey(),
                authority: metadata_authority.pubkey(), // Valid current metadata authority
                payer: payer.pubkey(),
                actions: actions.clone(),
                new_mint: None,
            },
            &metadata_authority,
            &payer,
            None,
        )
        .await;

        assert!(
            result.is_ok(),
            "Should succeed with valid metadata authority"
        );

        // Verify using assert_mint_action
        assert_mint_action(
            &mut rpc,
            compressed_mint_address,
            pre_compressed_mint,
            actions,
        )
        .await;
    }

    // 13. RemoveMetadataKey with invalid metadata authority
    {
        let result = light_token_client::actions::mint_action(
            &mut rpc,
            light_token_client::instructions::mint_action::MintActionParams {
                compressed_mint_address,
                mint_seed: mint_seed.pubkey(),
                authority: invalid_metadata_authority.pubkey(), // Invalid authority
                payer: payer.pubkey(),
                actions: vec![light_compressed_token_sdk::instructions::mint_action::MintActionType::RemoveMetadataKey {
                    extension_index: 0,
                    key: vec![1,2,3,4], // The key we added in additional_metadata
                    idempotent: 0, // 0 = false
                }],
                new_mint: None,
            },
            &invalid_metadata_authority,
            &payer,
            None,
        )
        .await;

        assert_rpc_error(
            result, 0, 18, // light_compressed_token::ErrorCode::InvalidAuthorityMint.into(),
        )
        .unwrap();
    }

    // 14. SUCCEED - RemoveMetadataKey with valid metadata authority
    {
        // Get pre-transaction compressed mint state
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

        let actions = vec![light_compressed_token_sdk::instructions::mint_action::MintActionType::RemoveMetadataKey {
            extension_index: 0,
            key: vec![1,2,3,4], // The key we added in additional_metadata
            idempotent: 0, // 0 = false
        }];

        let result = light_token_client::actions::mint_action(
            &mut rpc,
            light_token_client::instructions::mint_action::MintActionParams {
                compressed_mint_address,
                mint_seed: mint_seed.pubkey(),
                authority: new_metadata_authority.pubkey(), // Valid NEW metadata authority after update
                payer: payer.pubkey(),
                actions: actions.clone(),
                new_mint: None,
            },
            &new_metadata_authority,
            &payer,
            None,
        )
        .await;

        assert!(
            result.is_ok(),
            "Should succeed with valid metadata authority"
        );

        // Verify using assert_mint_action
        assert_mint_action(
            &mut rpc,
            compressed_mint_address,
            pre_compressed_mint,
            actions,
        )
        .await;
    }

    // 15. SUCCEED - RemoveMetadataKey idempotent (try to remove same key again)
    {
        // Get pre-transaction compressed mint state
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

        let actions = vec![light_compressed_token_sdk::instructions::mint_action::MintActionType::RemoveMetadataKey {
            extension_index: 0,
            key: vec![1,2,3,4], // Same key, already removed
            idempotent: 1, // 1 = true (won't error if key doesn't exist)
        }];

        let result = light_token_client::actions::mint_action(
            &mut rpc,
            light_token_client::instructions::mint_action::MintActionParams {
                compressed_mint_address,
                mint_seed: mint_seed.pubkey(),
                authority: new_metadata_authority.pubkey(), // Valid NEW metadata authority
                payer: payer.pubkey(),
                actions: actions.clone(),
                new_mint: None,
            },
            &new_metadata_authority,
            &payer,
            None,
        )
        .await;

        assert!(
            result.is_ok(),
            "Should succeed with idempotent=true even when key doesn't exist"
        );

        // Verify using assert_mint_action (no state change expected since key doesn't exist)
        assert_mint_action(
            &mut rpc,
            compressed_mint_address,
            pre_compressed_mint,
            actions,
        )
        .await;
    }
}

/// Functional test that uses multiple mint actions in a single instruction:
/// 1. MintToCompressed - mint to compressed account
/// 2. MintToCToken - mint to decompressed account
/// 3. UpdateMintAuthority
/// 4. UpdateFreezeAuthority
/// 5-8. UpdateMetadataField (Name, Symbol, URI, and add custom field)
/// 9. RemoveMetadataKey - remove original additional metadata
/// 10. UpdateMetadataAuthority
/// Note: all authorities must be the same else it cannot work.
#[tokio::test]
#[serial]
async fn functional_all_in_one_instruction() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();

    let payer = Keypair::new();
    rpc.airdrop_lamports(&payer.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let mint_seed = Keypair::new();
    let authority = Keypair::new();
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    // Derive compressed mint address for verification
    let compressed_mint_address =
        derive_compressed_mint_address(&mint_seed.pubkey(), &address_tree_pubkey);

    // Find mint PDA for the rest of the test
    let (spl_mint_pda, _) = find_spl_mint_address(&mint_seed.pubkey());
    // 1. Create compressed mint with both authorities
    {
        create_mint(
        &mut rpc,
        &mint_seed,
        8, // decimals
        &authority,
        Some(authority.pubkey()),
        Some(light_ctoken_types::instructions::extensions::token_metadata::TokenMetadataInstructionData {
            update_authority: Some(authority.pubkey().into()),
            name: "Test Token".as_bytes().to_vec(),
            symbol: "TEST".as_bytes().to_vec(),
            uri: "https://example.com/token.json".as_bytes().to_vec(),
            additional_metadata: Some(vec![
                AdditionalMetadata {
                    key: vec![1,2,3,4],
                    value: vec![2u8;5]
                },
                AdditionalMetadata {
                    key: vec![4,5,6,7],
                    value: vec![3u8;32]
                },
                AdditionalMetadata {
                    key: vec![4,5],
                    value: vec![4u8;32]
                },
                AdditionalMetadata {
                    key: vec![4,7],
                    value: vec![5u8;32]
                },
                AdditionalMetadata {
                    key: vec![8],
                    value: vec![6u8;32]
                }
            ]),
        }),
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
        8,
        authority.pubkey(),
        authority.pubkey(),
        Some(light_ctoken_types::instructions::extensions::token_metadata::TokenMetadataInstructionData {
            update_authority: Some(authority.pubkey().into()),
            name: "Test Token".as_bytes().to_vec(),
            symbol: "TEST".as_bytes().to_vec(),
            uri: "https://example.com/token.json".as_bytes().to_vec(),
            additional_metadata: Some(vec![
                AdditionalMetadata {
                    key: vec![1,2,3,4],
                    value: vec![2u8;5]
                },
                AdditionalMetadata {
                    key: vec![4,5,6,7],
                    value: vec![3u8;32]
                },
                AdditionalMetadata {
                    key: vec![4,5],
                    value: vec![4u8;32]
                },
                AdditionalMetadata {
                    key: vec![4,7],
                    value: vec![5u8;32]
                },
                AdditionalMetadata {
                    key: vec![8],
                    value: vec![6u8;32]
                }
            ]),
        }),
    );
    }

    // Fund authority
    rpc.airdrop_lamports(&authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // Create new authorities to update to
    let new_mint_authority = Keypair::new();
    let new_freeze_authority = Keypair::new();
    let new_metadata_authority = Keypair::new();

    // Create a ctoken account for MintToCToken
    let recipient = Keypair::new();
    let create_ata_ix = light_compressed_token_sdk::instructions::create_associated_token_account(
        payer.pubkey(),
        recipient.pubkey(),
        spl_mint_pda,
    )
    .unwrap();

    rpc.create_and_send_transaction(&[create_ata_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Build all actions for a single instruction
    let actions = vec![
        // 1. MintToCompressed - mint to compressed account
        light_compressed_token_sdk::instructions::mint_action::MintActionType::MintTo {
            recipients: vec![light_compressed_token_sdk::instructions::mint_action::MintToRecipient {
                recipient: Keypair::new().pubkey(),
                amount: 1000u64,
            }],
            token_account_version: 2,
        },
        // 2. MintToCToken - mint to decompressed account
        light_compressed_token_sdk::instructions::mint_action::MintActionType::MintToCToken {
            account: light_compressed_token_sdk::instructions::derive_ctoken_ata(
                &recipient.pubkey(),
                &spl_mint_pda,
            ).0,
            amount: 2000u64,
        },
        // 3. UpdateMintAuthority
        light_compressed_token_sdk::instructions::mint_action::MintActionType::UpdateMintAuthority {
            new_authority: Some(new_mint_authority.pubkey()),
        },
        // 4. UpdateFreezeAuthority
        light_compressed_token_sdk::instructions::mint_action::MintActionType::UpdateFreezeAuthority {
            new_authority: Some(new_freeze_authority.pubkey()),
        },
        // 5. UpdateMetadataField - update the name
        light_compressed_token_sdk::instructions::mint_action::MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 0, // Name field
            key: vec![],
            value: "Updated Token Name".as_bytes().to_vec(),
        },
        // 6. UpdateMetadataField - update the symbol
        light_compressed_token_sdk::instructions::mint_action::MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 1, // Symbol field
            key: vec![],
            value: "UPDATED".as_bytes().to_vec(),
        },
        // 7. UpdateMetadataField - update the URI
        light_compressed_token_sdk::instructions::mint_action::MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 2, // URI field
            key: vec![],
            value: "https://updated.example.com/token.json".as_bytes().to_vec(),
        },
        // 8. UpdateMetadataField - update the first additional metadata field
        light_compressed_token_sdk::instructions::mint_action::MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 3, // Custom key field
            key: vec![1, 2, 3, 4],
            value: "updated_value".as_bytes().to_vec(),
        },
        // 9. RemoveMetadataKey - remove the second additional metadata key
        light_compressed_token_sdk::instructions::mint_action::MintActionType::RemoveMetadataKey {
            extension_index: 0,
            key: vec![4, 5, 6, 7],
            idempotent: 0,
        },
        // 10. UpdateMetadataAuthority
        light_compressed_token_sdk::instructions::mint_action::MintActionType::UpdateMetadataAuthority {
            extension_index: 0,
            new_authority: new_metadata_authority.pubkey(),
        },
    ];

    // Get pre-state compressed mint
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

    // Execute all actions in a single instruction
    let result = light_token_client::actions::mint_action(
        &mut rpc,
        light_token_client::instructions::mint_action::MintActionParams {
            compressed_mint_address,
            mint_seed: mint_seed.pubkey(),
            authority: authority.pubkey(),
            payer: payer.pubkey(),
            actions: actions.clone(),
            new_mint: None,
        },
        &authority,
        &payer,
        None,
    )
    .await;

    assert!(result.is_ok(), "All-in-one mint action should succeed");

    // Use the new assert_mint_action function (now also validates CToken account state)
    assert_mint_action(
        &mut rpc,
        compressed_mint_address,
        pre_compressed_mint,
        actions,
    )
    .await;
}

/// Functional test that uses multiple mint actions in a single instruction:
/// - MintToCompressed - mint to compressed account
/// - MintToCToken - mint to decompressed account
/// - UpdateMetadataField (Name, Symbol, URI, and add custom field)
/// Any number, in any order, no authority updates, no key removal.
#[tokio::test]
#[serial]
async fn test_random_mint_action() {
    // Setup randomness
    use rand::{
        rngs::{StdRng, ThreadRng},
        Rng, RngCore, SeedableRng,
    };
    let mut thread_rng = ThreadRng::default();
    let seed = thread_rng.next_u64();
    // Keep this print so that in case the test fails
    // we can use the seed to reproduce the error.
    println!("\n\ntest seed {}\n\n", seed);
    let mut rng = StdRng::seed_from_u64(seed);

    // Generate random custom metadata keys (max 20)
    let num_keys = rng.gen_range(1..=20);
    let mut available_keys = Vec::new();
    let mut initial_metadata = Vec::new();
    for i in 0..num_keys {
        let key_len = rng.gen_range(1..=8); // Random key length 1-8 bytes
        let key: Vec<u8> = (0..key_len).map(|_| rng.gen()).collect();
        let value_len = rng.gen_range(5..=32); // Random value length
        let value = vec![(i + 2) as u8; value_len];

        available_keys.push(key.clone());
        initial_metadata.push(AdditionalMetadata { key, value });
    }
    let mut config = ProgramTestConfig::new_v2(false, None);
    let params = InitStateTreeAccountsInstructionData::default(); // larger queue for the batched state merkle tree
    config.v2_state_tree_config = Some(params);
    let mut rpc = LightProgramTest::new(config).await.unwrap();

    let payer = Keypair::new();
    rpc.airdrop_lamports(&payer.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let mint_seed = Keypair::new();
    let authority = Keypair::new();
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    // Derive compressed mint address for verification
    let compressed_mint_address =
        derive_compressed_mint_address(&mint_seed.pubkey(), &address_tree_pubkey);

    // Find mint PDA for the rest of the test
    let (spl_mint_pda, _) = find_spl_mint_address(&mint_seed.pubkey());

    // Fund authority first
    rpc.airdrop_lamports(&authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // 1. Create compressed mint with both authorities
    create_mint(
        &mut rpc,
        &mint_seed,
        8, // decimals
        &authority,
        Some(authority.pubkey()),
        Some(light_ctoken_types::instructions::extensions::token_metadata::TokenMetadataInstructionData {
            update_authority: Some(authority.pubkey().into()),
            name: "Test Token".as_bytes().to_vec(),
            symbol: "TEST".as_bytes().to_vec(),
            uri: "https://example.com/token.json".as_bytes().to_vec(),
            additional_metadata: Some(initial_metadata.clone()),
        }),
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
        8,
        authority.pubkey(),
        authority.pubkey(),
        Some(light_ctoken_types::instructions::extensions::token_metadata::TokenMetadataInstructionData {
            update_authority: Some(authority.pubkey().into()),
            name: "Test Token".as_bytes().to_vec(),
            symbol: "TEST".as_bytes().to_vec(),
            uri: "https://example.com/token.json".as_bytes().to_vec(),
            additional_metadata: Some(initial_metadata.clone()),
        }),
    );

    // Fund authority
    rpc.airdrop_lamports(&authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // Create 5 CToken ATAs upfront for MintToCToken actions
    let mut ctoken_recipients = Vec::new();
    let mut ctoken_atas = Vec::new();

    for _ in 0..5 {
        let recipient = Keypair::new();
        let create_ata_ix =
            light_compressed_token_sdk::instructions::create_associated_token_account(
                payer.pubkey(),
                recipient.pubkey(),
                spl_mint_pda,
            )
            .unwrap();

        rpc.create_and_send_transaction(&[create_ata_ix], &payer.pubkey(), &[&payer])
            .await
            .unwrap();

        let ata = light_compressed_token_sdk::instructions::derive_ctoken_ata(
            &recipient.pubkey(),
            &spl_mint_pda,
        )
        .0;

        ctoken_recipients.push(recipient);
        ctoken_atas.push(ata);
    }

    // Helper functions for random generation
    fn random_bytes(rng: &mut StdRng, min: usize, max: usize) -> Vec<u8> {
        use rand::Rng;
        let len = rng.gen_range(min..=max);
        (0..len).map(|_| rng.gen()).collect()
    }

    fn random_string(rng: &mut StdRng, min: usize, max: usize) -> Vec<u8> {
        use rand::Rng;
        let len = rng.gen_range(min..=max);
        let chars: Vec<u8> = (0..len)
            .map(|_| {
                let choice = rng.gen_range(0..62);
                match choice {
                    0..=25 => b'a' + (choice as u8),         // a-z
                    26..=51 => b'A' + ((choice - 26) as u8), // A-Z
                    _ => b'0' + ((choice - 52) as u8),       // 0-9
                }
            })
            .collect();
        chars
    }

    for _i in 0..1000 {
        println!("available_keys {:?}", available_keys);
        // Build random actions for a single instruction
        let mut actions = vec![];
        let mut total_recipients = 0;

        // Random total number of actions (1-20)
        let total_actions = rng.gen_range(1..=20);

        for _ in 0..total_actions {
            // Weighted random selection of action type
            let action_type = rng.gen_range(0..1000);

            match action_type {
                // 30% chance: MintToCompressed
                0..=299 => {
                    // Random number of recipients (1-5), but respect the 29 total limit
                    let max_additional = (29 - total_recipients).min(5);
                    if max_additional > 0 {
                        let num_recipients = rng.gen_range(1..=max_additional);
                        let mut recipients = Vec::new();

                        for _ in 0..num_recipients {
                            recipients.push(
                                light_compressed_token_sdk::instructions::mint_action::MintToRecipient {
                                    recipient: Keypair::new().pubkey(),
                                    amount: rng.gen_range(1..=100000),
                                }
                            );
                        }

                        total_recipients += num_recipients;

                        actions.push(
                            light_compressed_token_sdk::instructions::mint_action::MintActionType::MintTo {
                                recipients,
                                token_account_version: rng.gen_range(1..=3),
                            }
                        );
                    }
                }
                // 30% chance: MintToCToken
                300..=599 => {
                    // Randomly select one of the 5 pre-created ATAs
                    let ata_index = rng.gen_range(0..ctoken_atas.len());
                    actions.push(
                        light_compressed_token_sdk::instructions::mint_action::MintActionType::MintToCToken {
                            account: ctoken_atas[ata_index],
                            amount: rng.gen_range(1..=100000),
                        }
                    );
                }
                // 10% chance: Update Name
                600..=699 => {
                    let name = random_string(&mut rng, 1, 32);
                    actions.push(
                        light_compressed_token_sdk::instructions::mint_action::MintActionType::UpdateMetadataField {
                            extension_index: 0,
                            field_type: 0, // Name field
                            key: vec![],
                            value: name,
                        }
                    );
                }
                // 10% chance: Update Symbol
                700..=799 => {
                    let symbol = random_string(&mut rng, 1, 10);
                    actions.push(
                        light_compressed_token_sdk::instructions::mint_action::MintActionType::UpdateMetadataField {
                            extension_index: 0,
                            field_type: 1, // Symbol field
                            key: vec![],
                            value: symbol,
                        }
                    );
                }
                // 10% chance: Update URI
                800..=899 => {
                    let uri = random_string(&mut rng, 10, 200);
                    actions.push(
                        light_compressed_token_sdk::instructions::mint_action::MintActionType::UpdateMetadataField {
                            extension_index: 0,
                            field_type: 2, // URI field
                            key: vec![],
                            value: uri,
                        }
                    );
                }
                // 9.9% chance: Update Custom Metadata
                900..=998 => {
                    if !available_keys.is_empty() {
                        // Randomly select one of the available keys
                        let key_index = rng.gen_range(0..available_keys.len());
                        let key = available_keys[key_index].clone();
                        let value = random_bytes(&mut rng, 1, 64);

                        actions.push(
                            light_compressed_token_sdk::instructions::mint_action::MintActionType::UpdateMetadataField {
                                extension_index: 0,
                                field_type: 3, // Custom field
                                key,
                                value,
                            }
                        );
                    }
                }
                // 0.1% chance: Remove Custom Metadata Key
                999 => {
                    if !available_keys.is_empty() {
                        // Randomly select and remove one of the available keys
                        let key_index = rng.gen_range(0..available_keys.len());
                        let key = available_keys.remove(key_index);

                        actions.push(
                            light_compressed_token_sdk::instructions::mint_action::MintActionType::RemoveMetadataKey {
                                extension_index: 0,
                                key,
                                idempotent: if available_keys.is_empty() { 1 } else { rng.gen_bool(0.5) as u8 }, // 50% chance idempotent when keys exist, always when none left
                            }
                        );
                    } else {
                        // No keys left, try to remove a random key (always idempotent)
                        let random_key = vec![rng.gen::<u8>(), rng.gen::<u8>()];

                        actions.push(
                            light_compressed_token_sdk::instructions::mint_action::MintActionType::RemoveMetadataKey {
                                extension_index: 0, // Only TokenMetadata extension exists (index 0)
                                key: random_key,
                                idempotent: 1, // Always idempotent when no keys exist
                            }
                        );
                    }
                }
                // This should never happen since we generate 0..1000, but added for completeness
                _ => {
                    // Skip this iteration if we somehow get an invalid range
                    continue;
                }
            }
        }

        // Shuffle the actions to randomize order
        use rand::seq::SliceRandom;
        actions.shuffle(&mut rng);

        // Get pre-state compressed mint
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
        println!("actions {:?}", actions);
        // Execute all actions in a single instruction
        let result = light_token_client::actions::mint_action(
            &mut rpc,
            light_token_client::instructions::mint_action::MintActionParams {
                compressed_mint_address,
                mint_seed: mint_seed.pubkey(),
                authority: authority.pubkey(),
                payer: payer.pubkey(),
                actions: actions.clone(),
                new_mint: None,
            },
            &authority,
            &payer,
            None,
        )
        .await;

        assert!(result.is_ok(), "All-in-one mint action should succeed");

        // Use the new assert_mint_action function (now also validates CToken account state)
        assert_mint_action(
            &mut rpc,
            compressed_mint_address,
            pre_compressed_mint,
            actions,
        )
        .await;
    }
}

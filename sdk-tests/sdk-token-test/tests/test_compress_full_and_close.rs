use anchor_lang::{
    prelude::{AccountMeta, Pubkey},
    InstructionData,
};
use light_compressed_token_sdk::instructions::{
    create_associated_token_account, create_compressed_mint, create_mint_to_compressed_instruction,
    derive_ctoken_ata, CreateCompressedMintInputs, MintToCompressedInputs,
};
use light_ctoken_types::{
    instructions::mint_action::{CompressedMintWithContext, Recipient},
    state::{BaseMint, CompressedMint, CompressedMintMetadata},
    COMPRESSED_MINT_SEED, COMPRESSED_TOKEN_PROGRAM_ID,
};
use light_program_test::{Indexer, LightProgramTest, ProgramTestConfig, Rpc};
use light_sdk::instruction::{PackedAccounts, SystemAccountMetaConfig};
use light_token_client::instructions::transfer2::create_decompress_instruction;
use sdk_token_test::instruction;
use serial_test::serial;
use solana_sdk::{
    instruction::Instruction, signature::Keypair, signer::Signer, transaction::Transaction,
};

#[tokio::test]
#[serial]
async fn test_compress_full_and_close() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("sdk_token_test", sdk_token_test::ID)]),
    ))
    .await
    .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    println!("🔧 Setting up compressed mint and tokens...");

    // Step 1: Create a compressed mint
    let decimals = 6u8;
    let mint_authority_keypair = Keypair::new();
    let mint_authority = mint_authority_keypair.pubkey();
    let freeze_authority = Pubkey::new_unique();
    let mint_signer = Keypair::new();

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    let compressed_token_program_id =
        Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID);
    let (mint_pda, mint_bump) = Pubkey::find_program_address(
        &[COMPRESSED_MINT_SEED, mint_signer.pubkey().as_ref()],
        &compressed_token_program_id,
    );

    let address_seed = mint_pda.to_bytes();
    let compressed_mint_address = light_compressed_account::address::derive_address(
        &address_seed,
        &address_tree_pubkey.to_bytes(),
        &compressed_token_program_id.to_bytes(),
    );

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

    let instruction = create_compressed_mint(CreateCompressedMintInputs {
        version: 3,
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
    })
    .unwrap();

    rpc.create_and_send_transaction(
        &[instruction],
        &payer.pubkey(),
        &[&payer, &mint_signer, &mint_authority_keypair],
    )
    .await
    .unwrap();

    println!("✅ Created compressed mint: {}", mint_pda);

    // Step 2: Mint compressed tokens
    let mint_amount = 1000u64;
    let recipient_keypair = Keypair::new();
    let recipient = recipient_keypair.pubkey();

    let compressed_mint_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value
        .ok_or("Compressed mint account not found")
        .unwrap();

    let expected_compressed_mint = CompressedMint {
        base: BaseMint {
            mint_authority: Some(mint_authority.into()),
            supply: 0,
            decimals,
            is_initialized: true,
            freeze_authority: Some(freeze_authority.into()),
        },
        metadata: CompressedMintMetadata {
            version: 3,
            mint: mint_pda.into(),
            spl_mint_initialized: false,
        },
        extensions: None,
    };

    let compressed_mint_inputs = CompressedMintWithContext {
        prove_by_index: true,
        leaf_index: compressed_mint_account.leaf_index,
        root_index: 0,
        address: compressed_mint_address,
        mint: expected_compressed_mint.try_into().unwrap(),
    };

    let mint_instruction = create_mint_to_compressed_instruction(
        MintToCompressedInputs {
            cpi_context_pubkey: None,
            proof: None,
            compressed_mint_inputs,
            recipients: vec![Recipient {
                recipient: recipient.into(),
                amount: mint_amount,
            }],
            mint_authority,
            payer: payer.pubkey(),
            state_merkle_tree: compressed_mint_account.tree_info.tree,
            input_queue: compressed_mint_account.tree_info.queue,
            output_queue_cmint: compressed_mint_account.tree_info.queue,
            output_queue_tokens: compressed_mint_account.tree_info.queue,
            decompressed_mint_config: None,
            token_account_version: 2,
            token_pool: None,
        },
        None,
    )
    .unwrap();

    rpc.create_and_send_transaction(
        &[mint_instruction],
        &payer.pubkey(),
        &[&payer, &mint_authority_keypair],
    )
    .await
    .unwrap();

    println!("✅ Minted {} compressed tokens to recipient", mint_amount);

    // Step 4: Create associated token account for decompression
    let (ctoken_ata_pubkey, _bump) = derive_ctoken_ata(&recipient, &mint_pda);
    let create_ata_instruction =
        create_associated_token_account(payer.pubkey(), recipient, mint_pda).unwrap();

    rpc.create_and_send_transaction(&[create_ata_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    println!("✅ Created associated token account: {}", ctoken_ata_pubkey);

    // Step 5: Decompress compressed tokens to the token account
    let decompress_amount = mint_amount; // Decompress all tokens

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

    let decompress_instruction = create_decompress_instruction(
        &mut rpc,
        std::slice::from_ref(&compressed_token_accounts[0]),
        decompress_amount,
        ctoken_ata_pubkey,
        payer.pubkey(),
    )
    .await
    .unwrap();

    rpc.create_and_send_transaction(
        &[decompress_instruction],
        &payer.pubkey(),
        &[&payer, &recipient_keypair],
    )
    .await
    .unwrap();

    println!(
        "✅ Decompressed {} tokens to SPL token account",
        decompress_amount
    );

    // Verify the token account has the expected balance by checking it exists and has data
    let token_account_info = rpc.get_account(ctoken_ata_pubkey).await.unwrap().unwrap();
    assert!(
        token_account_info.lamports > 0,
        "Token account should exist with lamports"
    );
    assert!(
        !token_account_info.data.is_empty(),
        "Token account should have data"
    );

    // Step 6: Now test our compress_full_and_close instruction
    println!("🧪 Testing compress_full_and_close instruction...");

    let final_recipient = Keypair::new();
    let final_recipient_pubkey = final_recipient.pubkey();
    let close_recipient = Keypair::new();
    let close_recipient_pubkey = close_recipient.pubkey();

    // Airdrop lamports to close recipient
    rpc.context
        .airdrop(&close_recipient_pubkey, 1_000_000)
        .unwrap();

    // Create remaining accounts following four_multi_transfer pattern
    let mut remaining_accounts = PackedAccounts::default();
    remaining_accounts.add_pre_accounts_meta(AccountMeta::new_readonly(
        Pubkey::new_from_array(COMPRESSED_TOKEN_PROGRAM_ID),
        false,
    ));
    remaining_accounts
        .add_system_accounts_v2(SystemAccountMetaConfig::new(Pubkey::new_from_array(
            COMPRESSED_TOKEN_PROGRAM_ID,
        )))
        .unwrap();

    remaining_accounts.insert_or_get(rpc.get_random_state_tree_info().unwrap().queue);
    // Pack accounts using insert_or_get (following four_multi_transfer pattern)
    let recipient_index = remaining_accounts.insert_or_get(final_recipient_pubkey);
    let mint_index = remaining_accounts.insert_or_get(mint_pda);
    let source_index = remaining_accounts.insert_or_get(ctoken_ata_pubkey); // Token account to compress
    let authority_index = remaining_accounts.insert_or_get(recipient_keypair.pubkey()); // Authority
    let close_recipient_index = remaining_accounts.insert_or_get(close_recipient_pubkey); // Close recipient

    // Get remaining accounts and create instruction
    let (account_metas, system_accounts_offset, _packed_accounts_offset) =
        remaining_accounts.to_account_metas();

    let instruction_data = instruction::CompressFullAndClose {
        recipient_index,
        mint_index,
        source_index,
        authority_index,
        close_recipient_index,
        system_accounts_offset: system_accounts_offset as u8,
    };
    rpc.airdrop_lamports(&recipient_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    // Prepend signer as first account (for Generic<'info> struct)
    let accounts = [
        vec![solana_sdk::instruction::AccountMeta::new(
            recipient_keypair.pubkey(),
            true,
        )],
        account_metas,
    ]
    .concat();

    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts,
        data: instruction_data.data(),
    };

    println!("📤 Executing compress_full_and_close instruction...");

    // Execute the instruction
    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer, &recipient_keypair],
        blockhash,
    );

    let result = rpc.process_transaction(transaction).await;

    match result {
        Ok(_) => {
            println!("✅ compress_full_and_close instruction executed successfully!");

            // Verify the token account was closed
            let closed_account = rpc.get_account(ctoken_ata_pubkey).await.unwrap();
            if let Some(account) = closed_account {
                assert_eq!(
                    account.lamports, 0,
                    "Token account should have 0 lamports after closing"
                );
                assert!(
                    account.data.iter().all(|&b| b == 0),
                    "Token account data should be cleared"
                );
            }

            // Verify compressed tokens were created for the final recipient
            let final_compressed_tokens = rpc
                .indexer()
                .unwrap()
                .get_compressed_token_accounts_by_owner(&final_recipient_pubkey, None, None)
                .await
                .unwrap()
                .value
                .items;

            assert_eq!(
                final_compressed_tokens.len(),
                1,
                "Should have exactly one compressed token account for final recipient"
            );

            let final_compressed_token = &final_compressed_tokens[0].token;
            assert_eq!(
                final_compressed_token.amount, decompress_amount,
                "Final compressed token should have the full original amount"
            );
            assert_eq!(
                final_compressed_token.owner, final_recipient_pubkey,
                "Final compressed token should have correct owner"
            );
            assert_eq!(
                final_compressed_token.mint, mint_pda,
                "Final compressed token should have correct mint"
            );

            println!("✅ All verifications passed!");
            println!("   - Original amount: {} tokens", mint_amount);
            println!("   - Decompressed: {} tokens", decompress_amount);
            println!(
                "   - Compressed full and closed: {} tokens",
                final_compressed_token.amount
            );
            println!("   - Token account closed successfully");
            println!("   - Lamports transferred to close recipient");
        }
        Err(e) => {
            panic!("❌ compress_full_and_close instruction failed: {:?}", e);
        }
    }
}

use anchor_lang::prelude::Pubkey;
use anchor_lang::prelude::{
    borsh::{BorshDeserialize, BorshSerialize},
    AccountMeta,
};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_account::compressed_account::PackedMerkleContext;
use light_compressed_token_sdk::instructions::{
    derive_compressed_mint_address, find_spl_mint_address,
    transfer2::account_metas::{
        get_transfer2_instruction_account_metas, Transfer2AccountsMetaConfig,
    },
};
use light_ctoken_types::{instructions::mint_action::Recipient, state::TokenDataVersion};
use light_ctoken_types::{
    instructions::transfer2::{
        CompressedTokenInstructionDataTransfer2, Compression, CompressionMode,
        MultiInputTokenDataWithContext, MultiTokenTransferOutputData,
    },
    COMPRESSED_TOKEN_PROGRAM_ID,
};
use light_program_test::{utils::assert::assert_rpc_error, LightProgramTest, ProgramTestConfig};
use light_sdk::instruction::PackedAccounts;
use light_test_utils::{
    airdrop_lamports,
    assert_transfer2::{assert_transfer2, assert_transfer2_with_delegate},
    mint_assert::assert_compressed_mint_account,
};
use light_token_client::{
    actions::{
        create_compressible_token_account, create_mint, mint_to_compressed, transfer2,
        CreateCompressibleTokenAccountInputs,
    },
    instructions::transfer2::{Transfer2InstructionType, TransferInput},
};
use serial_test::serial;
use solana_sdk::{signature::Keypair, signer::Signer};

#[tokio::test]
#[serial]
async fn test_transfer2_delegated_partial() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    // Test parameters
    let decimals = 6u8;
    let mint_authority_keypair = Keypair::new(); // Create keypair so we can sign
    let mint_seed = Keypair::new();
    // Find mint PDA for the rest of the test
    let (spl_mint_pda, _) = find_spl_mint_address(&mint_seed.pubkey());

    create_mint(
        &mut rpc,
        &mint_seed,
        decimals,
        &mint_authority_keypair,
        None,
        None, // No metadata
        &payer,
    )
    .await
    .unwrap();

    let recipient_keypair = Keypair::new();
    let recipient = recipient_keypair.pubkey();
    let mint_amount = 1000u64;

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

    // Get the compressed token account
    let compressed_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&recipient, None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(compressed_accounts.len(), 1);
    assert_eq!(compressed_accounts[0].token.amount, mint_amount);
    assert_eq!(compressed_accounts[0].token.delegate, None);

    // Create a delegate
    let delegate_keypair = Keypair::new();
    let delegate = delegate_keypair.pubkey();
    airdrop_lamports(&mut rpc, &delegate, 10_000_000_000)
        .await
        .unwrap();

    // Approve delegation using the new approve action
    let delegate_amount = 600u64;
    transfer2::approve(
        &mut rpc,
        &compressed_accounts,
        delegate,
        delegate_amount,
        &recipient_keypair,
        &payer,
    )
    .await
    .unwrap();

    // Get updated compressed accounts after approval
    let compressed_accounts_after_approve = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&recipient, None, None)
        .await
        .unwrap()
        .value
        .items;

    // Should have 2 accounts now: change account and delegated account
    assert_eq!(compressed_accounts_after_approve.len(), 2);

    // Find the delegated account
    let delegated_account = compressed_accounts_after_approve
        .iter()
        .find(|acc| acc.token.delegate == Some(delegate))
        .expect("Should find delegated account");

    assert_eq!(delegated_account.token.amount, delegate_amount);
    assert_eq!(delegated_account.token.delegate, Some(delegate));

    // Find the change account
    let change_account = compressed_accounts_after_approve
        .iter()
        .find(|acc| acc.token.delegate.is_none())
        .expect("Should find change account");

    assert_eq!(change_account.token.amount, mint_amount - delegate_amount);

    // Now delegate transfers partial amount using transfer2
    let transfer_recipient = Keypair::new().pubkey();
    let transfer_amount = 200u64;

    transfer2::transfer_delegated(
        &mut rpc,
        &[delegated_account.clone()],
        transfer_recipient,
        transfer_amount,
        &delegate_keypair,
        &payer,
    )
    .await
    .unwrap();

    // Verify the transfer using assert_transfer2_with_delegate
    assert_transfer2_with_delegate(
        &mut rpc,
        vec![Transfer2InstructionType::Transfer(TransferInput {
            compressed_token_account: vec![delegated_account.clone()],
            to: transfer_recipient,
            amount: transfer_amount,
            is_delegate_transfer: true, // This was a delegate transfer
            mint: None,
            change_amount: None,
        })],
        Some(delegate),
    )
    .await;

    // Get the remaining delegated account after delegate's transfer
    // The change account should still have the delegate set
    let accounts_after_delegate = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&recipient, None, None)
        .await
        .unwrap()
        .value
        .items;

    let remaining_delegated_account = accounts_after_delegate
        .into_iter()
        .find(|acc| {
            acc.token.delegate == Some(delegate)
                && acc.token.amount == (delegate_amount - transfer_amount)
        })
        .expect("Should find remaining delegated account with delegate still set");

    // Now have the OWNER transfer the remaining delegated tokens
    let owner_transfer_recipient = Keypair::new().pubkey();
    let owner_transfer_amount = 150u64;

    transfer2::transfer(
        &mut rpc,
        &[remaining_delegated_account.clone()],
        owner_transfer_recipient,
        owner_transfer_amount,
        &recipient_keypair, // Owner is signing
        &payer,
    )
    .await
    .unwrap();

    // Verify the owner's transfer
    assert_transfer2(
        &mut rpc,
        vec![Transfer2InstructionType::Transfer(TransferInput {
            compressed_token_account: vec![remaining_delegated_account.clone()],
            to: owner_transfer_recipient,
            amount: owner_transfer_amount,
            is_delegate_transfer: false, // Owner is transferring, not delegate
            mint: None,
            change_amount: None,
        })],
    )
    .await;

    println!("✅ Test passed: Both delegate and owner can transfer delegated tokens!");
}

#[tokio::test]
#[serial]
async fn test_transfer2_sha_flat() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    // Test parameters
    let decimals = 6u8;
    let mint_authority_keypair = Keypair::new(); // Create keypair so we can sign
    let mint_seed = Keypair::new();
    // Find mint PDA for the rest of the test
    let (spl_mint_pda, _) = find_spl_mint_address(&mint_seed.pubkey());

    create_mint(
        &mut rpc,
        &mint_seed,
        decimals,
        &mint_authority_keypair,
        None,
        None, // No metadata
        &payer,
    )
    .await
    .unwrap();

    let recipient_keypair = Keypair::new();
    let recipient = recipient_keypair.pubkey();
    let mint_amount = 1000u64;

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

    // Get the compressed token account
    let compressed_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&recipient, None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(compressed_accounts.len(), 1);
    assert_eq!(compressed_accounts[0].token.amount, mint_amount);
    assert_eq!(compressed_accounts[0].token.delegate, None);

    // Now have the OWNER transfer the remaining delegated tokens
    let owner_transfer_recipient = Keypair::new().pubkey();
    let owner_transfer_amount = 150u64;

    transfer2::transfer(
        &mut rpc,
        &[compressed_accounts[0].clone()],
        owner_transfer_recipient,
        owner_transfer_amount,
        &recipient_keypair, // Owner is signing
        &payer,
    )
    .await
    .unwrap();

    // Verify the owner's transfer
    assert_transfer2(
        &mut rpc,
        vec![Transfer2InstructionType::Transfer(TransferInput {
            compressed_token_account: vec![compressed_accounts[0].clone()],
            to: owner_transfer_recipient,
            amount: owner_transfer_amount,
            is_delegate_transfer: false, // Owner is transferring, not delegate
            mint: None,
            change_amount: None,
        })],
    )
    .await;

    println!("✅ Test passed: Both delegate and owner can transfer delegated tokens!");
}

/// Failing Transfer2 tests:
/// 1. FAIL - Compress  - ctoken invalid owner
/// 2. FAIL - Compress - ctoken insufficient amount
/// 3. FAIL - Compress - invalid ctoken account
/// 3. FAIL - Decompress - ctoken invalid owner
/// 4. FAIL - Decompress - insufficient amount
/// 5. FAIL - Decompress - invalid ctoken account
/// 6. FAIL - Transfer2 - invalid owner of compressed account
/// 7. FAIL - Transfer2 - invalid delegate signer
/// 8. FAIL - Transfer2 - insufficient amount
/// 9. FAIL - CompressAndClose - invalid owner
/// 10. FAIL - CompressAndClose - invalid ctoken account
/// 11. FAIL - CompressAndClose - invalid compression authority
#[tokio::test]
#[serial]
async fn transfer2_failing_tests() {
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
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    // Derive compressed mint address for verification
    let compressed_mint_address =
        derive_compressed_mint_address(&mint_seed.pubkey(), &address_tree_pubkey);

    // Find mint PDA for the rest of the test
    let (spl_mint_pda, _) = find_spl_mint_address(&mint_seed.pubkey());
    // Test setup:
    // 1. cmint
    // 2. mint compressed tokens
    // 3. create ctoken ata
    // 4. decompress some tokens to the ctoken ata
    {
        create_mint(
            &mut rpc,
            &mint_seed,
            8, // decimals
            &mint_authority,
            Some(freeze_authority.pubkey()),
            None,
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
            None, // No metadata
        );
        // Create a recipient for compressed tokens
        let recipient_keypair = Keypair::new();
        let mint_amount = 1000u64;

        light_token_client::actions::mint_to_compressed(
            &mut rpc,
            spl_mint_pda,
            vec![light_ctoken_types::instructions::mint_action::Recipient {
                recipient: recipient_keypair.pubkey().to_bytes().into(),
                amount: mint_amount,
            }],
            light_ctoken_types::state::TokenDataVersion::V2,
            &mint_authority,
            &payer,
        )
        .await
        .unwrap();

        // Create ctoken account for decompression tests
        let ctoken_account_keypair = Keypair::new();
        let ctoken_account_pubkey = create_compressible_token_account(
            &mut rpc,
            CreateCompressibleTokenAccountInputs {
                owner: recipient_keypair.pubkey(),
                mint: spl_mint_pda,
                num_prepaid_epochs: 10,
                payer: &payer,
                token_account_keypair: Some(&ctoken_account_keypair),
                lamports_per_write: None,
                token_account_version: TokenDataVersion::ShaFlat,
            },
        )
        .await
        .unwrap();

        // Get compressed token accounts for decompression
        let compressed_accounts = rpc
            .indexer()
            .unwrap()
            .get_compressed_token_accounts_by_owner(&recipient_keypair.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items;

        // Decompress some tokens to the ctoken account
        let decompress_amount = 500u64;
        transfer2::decompress(
            &mut rpc,
            &compressed_accounts,
            decompress_amount,
            ctoken_account_pubkey,
            &recipient_keypair,
            &payer,
        )
        .await
        .unwrap();

        // Create invalid authorities for failing tests
        let invalid_owner = Keypair::new();
        airdrop_lamports(&mut rpc, &invalid_owner.pubkey(), 10_000_000_000)
            .await
            .unwrap();

        // Now we have setup:
        // - compressed_accounts[0] has 500 tokens remaining (recipient_keypair is owner)
        // - ctoken_account_pubkey has 500 tokens (recipient_keypair is owner)

        // FAIL Test 1: Compress - ctoken invalid owner (manual instruction)
        {
            // Create packed accounts properly using SDK
            let mut packed_accounts = PackedAccounts::default();

            // Get state tree info
            let state_tree_info = rpc.get_random_state_tree_info().unwrap();

            // Add accounts in proper order for Transfer2
            let merkle_tree_idx = packed_accounts.insert_or_get(state_tree_info.tree);
            let mint_idx = packed_accounts.insert_or_get_read_only(spl_mint_pda);
            let invalid_owner_idx = packed_accounts.insert_or_get_read_only(invalid_owner.pubkey()); // Invalid owner NOT as signer
            let recipient_idx = packed_accounts.insert_or_get_read_only(recipient_keypair.pubkey());
            let ctoken_account_idx = packed_accounts.insert_or_get(ctoken_account_pubkey);

            // Build instruction data for compression with invalid owner
            let instruction_data = CompressedTokenInstructionDataTransfer2 {
                with_transaction_hash: false,
                with_lamports_change_account_merkle_tree_index: false,
                lamports_change_account_merkle_tree_index: 0,
                lamports_change_account_owner_index: 0,
                cpi_context: None,
                compressions: Some(vec![Compression::compress_ctoken(
                    100,
                    mint_idx,
                    ctoken_account_idx,
                    invalid_owner_idx, // Invalid owner as authority
                )]),
                proof: None,
                in_token_data: vec![], // No input compressed accounts
                out_token_data: vec![MultiTokenTransferOutputData {
                    owner: recipient_idx as u8,
                    amount: 100,
                    has_delegate: false,
                    delegate: 0,
                    mint: mint_idx as u8,
                    version: 3, // ShaFlat version
                    merkle_tree: merkle_tree_idx as u8,
                }],
                in_lamports: None,
                out_lamports: None,
                in_tlv: None,
                out_tlv: None,
            };

            // Use SDK to create proper account metas
            let meta_config = Transfer2AccountsMetaConfig::new(
                payer.pubkey(),
                packed_accounts.to_account_metas().0, // Get Vec<AccountMeta> from tuple
            );
            let account_metas = get_transfer2_instruction_account_metas(meta_config);

            let ix = solana_sdk::instruction::Instruction {
                program_id: COMPRESSED_TOKEN_PROGRAM_ID.into(),
                accounts: account_metas,
                data: {
                    let mut data = vec![104]; // Transfer2 discriminator
                    data.extend_from_slice(&instruction_data.try_to_vec().unwrap());
                    data
                },
            };

            let signers = vec![&payer]; // Only payer signs, invalid owner should fail validation
            let result = rpc
                .create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
                .await;

            // Should fail with program-level error for invalid owner
            assert_rpc_error(
                result, 0, 12015, // InvalidSigner - invalid owner not signing
            )
            .unwrap();
            println!("✅ FAIL Test 1 passed: Compress with invalid owner (program-level)");
        }

        // FAIL Test 2: Compress - ctoken insufficient amount (manual instruction)
        {
            // Create packed accounts properly using SDK
            let mut packed_accounts = PackedAccounts::default();

            // Get state tree info
            let state_tree_info = rpc.get_random_state_tree_info().unwrap();

            // Add accounts in proper order for Transfer2
            let merkle_tree_idx = packed_accounts.insert_or_get(state_tree_info.tree);
            let queue_idx = packed_accounts.insert_or_get(state_tree_info.queue);
            let mint_idx = packed_accounts.insert_or_get_read_only(spl_mint_pda);
            let owner_idx = packed_accounts.insert_or_get_read_only(recipient_keypair.pubkey()); // Owner NOT as signer
            let recipient_idx = packed_accounts.insert_or_get_read_only(recipient_keypair.pubkey());
            let ctoken_account_idx = packed_accounts.insert_or_get(ctoken_account_pubkey);

            // Build instruction data for compression with excessive amount
            let instruction_data = CompressedTokenInstructionDataTransfer2 {
                with_transaction_hash: false,
                with_lamports_change_account_merkle_tree_index: false,
                lamports_change_account_merkle_tree_index: 0,
                lamports_change_account_owner_index: 0,
                cpi_context: None,
                compressions: Some(vec![Compression::compress_ctoken(
                    1000, // More than the 500 tokens in the account
                    mint_idx,
                    ctoken_account_idx,
                    owner_idx,
                )]),
                proof: None,
                in_token_data: vec![], // No input compressed accounts
                out_token_data: vec![MultiTokenTransferOutputData {
                    owner: recipient_idx as u8,
                    amount: 1000, // More than available (500)
                    has_delegate: false,
                    delegate: 0,
                    mint: mint_idx as u8,
                    version: 3, // ShaFlat version
                    merkle_tree: merkle_tree_idx as u8,
                }],
                in_lamports: None,
                out_lamports: None,
                in_tlv: None,
                out_tlv: None,
            };

            // Use SDK to create proper account metas
            let meta_config = Transfer2AccountsMetaConfig::new(
                payer.pubkey(),
                packed_accounts.to_account_metas().0, // Get Vec<AccountMeta> from tuple
            );
            let account_metas = get_transfer2_instruction_account_metas(meta_config);

            let ix = solana_sdk::instruction::Instruction {
                program_id: COMPRESSED_TOKEN_PROGRAM_ID.into(),
                accounts: account_metas,
                data: {
                    let mut data = vec![104]; // Transfer2 discriminator
                    data.extend_from_slice(&instruction_data.try_to_vec().unwrap());
                    data
                },
            };

            let signers = vec![&payer]; // Only payer signs, owner should fail validation
            let result = rpc
                .create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
                .await;

            // Should fail with program-level error for missing signer
            assert_rpc_error(
                result, 0, 12015, // InvalidSigner - owner not signing
            )
            .unwrap();
            println!("✅ FAIL Test 2 passed: Compress with insufficient amount (program-level)");
        }

        // FAIL Test 3: Transfer2 - missing signer authority (manual instruction)
        {
            // Create packed accounts properly using SDK
            let mut packed_accounts = PackedAccounts::default();

            // Get state tree info
            let state_tree_info = rpc.get_random_state_tree_info().unwrap();

            // Add accounts in proper order for Transfer2
            let merkle_tree_idx = packed_accounts.insert_or_get(state_tree_info.tree);
            let queue_idx = packed_accounts.insert_or_get(state_tree_info.queue);
            let mint_idx = packed_accounts.insert_or_get_read_only(spl_mint_pda);
            let owner_idx = packed_accounts.insert_or_get_read_only(recipient_keypair.pubkey()); // Owner NOT as signer - this is the test
            let recipient_idx = packed_accounts.insert_or_get_read_only(Keypair::new().pubkey());

            // Build instruction data for a simple transfer that requires signer
            let instruction_data = CompressedTokenInstructionDataTransfer2 {
                with_transaction_hash: false,
                with_lamports_change_account_merkle_tree_index: false,
                lamports_change_account_merkle_tree_index: 0,
                lamports_change_account_owner_index: 0,
                cpi_context: None,
                compressions: None, // No compressions, just transfer
                proof: None,
                in_token_data: vec![MultiInputTokenDataWithContext {
                    owner: owner_idx as u8,
                    amount: 100, // Small amount
                    has_delegate: false,
                    delegate: 0,
                    mint: mint_idx as u8,
                    version: 3, // ShaFlat version
                    merkle_context: PackedMerkleContext {
                        merkle_tree_pubkey_index: merkle_tree_idx as u8,
                        queue_pubkey_index: queue_idx as u8,
                        leaf_index: 0, // Dummy leaf index
                        prove_by_index: true,
                    },
                    root_index: 0,
                }],
                out_token_data: vec![MultiTokenTransferOutputData {
                    owner: recipient_idx as u8,
                    amount: 100,
                    has_delegate: false,
                    delegate: 0,
                    mint: mint_idx as u8,
                    version: 3, // ShaFlat version
                    merkle_tree: merkle_tree_idx as u8,
                }],
                in_lamports: None,
                out_lamports: None,
                in_tlv: None,
                out_tlv: None,
            };

            // Use SDK to create proper account metas
            let meta_config = Transfer2AccountsMetaConfig::new(
                payer.pubkey(),
                packed_accounts.to_account_metas().0, // Get Vec<AccountMeta> from tuple
            );
            let account_metas = get_transfer2_instruction_account_metas(meta_config);

            let ix = solana_sdk::instruction::Instruction {
                program_id: COMPRESSED_TOKEN_PROGRAM_ID.into(),
                accounts: account_metas,
                data: {
                    let mut data = vec![104]; // Transfer2 discriminator
                    data.extend_from_slice(&instruction_data.try_to_vec().unwrap());
                    data
                },
            };

            let signers = vec![&payer]; // Only payer signs, authority should fail validation
            let result = rpc
                .create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
                .await;

            // Should fail with program-level error for missing signer
            assert_rpc_error(
                result, 0, 12015, // InvalidSigner - authority not signing
            )
            .unwrap();
            println!(
                "✅ FAIL Test 3 passed: Transfer2 with missing signer authority (program-level)"
            );
        }

        // Get fresh compressed accounts for decompress tests
        let fresh_compressed_accounts = rpc
            .indexer()
            .unwrap()
            .get_compressed_token_accounts_by_owner(&recipient_keypair.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items;

        // FAIL Test 4: Decompress - invalid owner of compressed account
        // We need to manually create an instruction to bypass client-side validation
        {
            // Create a modified compressed account with wrong owner in the token data
            let mut modified_account = fresh_compressed_accounts[0].clone();
            modified_account.token.owner = invalid_owner.pubkey(); // Override the owner

            let result = transfer2::decompress(
                &mut rpc,
                &[modified_account], // Use modified account with invalid owner
                100u64,
                ctoken_account_pubkey,
                &invalid_owner, // Authority matches the modified owner
                &payer,
            )
            .await;

            assert_rpc_error(
                result, 0,
                14307, // Hash mismatch error (0x37e3) - program correctly detects modified compressed account
            )
            .unwrap();
            println!("✅ FAIL Test 4 passed: Decompress with invalid owner");
        }

        // FAIL Test 4b: Decompress - owner not signer (manual instruction)
        {
            // Create packed accounts properly using SDK
            let mut packed_accounts = PackedAccounts::default();

            // Get state tree info
            let state_tree_info = rpc.get_random_state_tree_info().unwrap();

            // Add accounts in proper order for Transfer2
            let merkle_tree_idx = packed_accounts.insert_or_get(state_tree_info.tree);
            let queue_idx = packed_accounts.insert_or_get(state_tree_info.queue);
            let mint_idx = packed_accounts.insert_or_get_read_only(spl_mint_pda);
            let owner_idx = packed_accounts.insert_or_get_read_only(recipient_keypair.pubkey()); // Owner NOT as signer - this is the test
            let ctoken_account_idx = packed_accounts.insert_or_get(ctoken_account_pubkey);

            // Build instruction data for decompression requiring owner signature
            let instruction_data = CompressedTokenInstructionDataTransfer2 {
                with_transaction_hash: false,
                with_lamports_change_account_merkle_tree_index: false,
                lamports_change_account_merkle_tree_index: 0,
                lamports_change_account_owner_index: 0,
                cpi_context: None,
                compressions: Some(vec![Compression::decompress_ctoken(
                    100,
                    mint_idx,
                    ctoken_account_idx,
                )]),
                proof: None,
                in_token_data: vec![MultiInputTokenDataWithContext {
                    owner: owner_idx as u8,
                    amount: 100, // Available amount in compressed account
                    has_delegate: false,
                    delegate: 0,
                    mint: mint_idx as u8,
                    version: 3, // ShaFlat version
                    merkle_context: PackedMerkleContext {
                        merkle_tree_pubkey_index: merkle_tree_idx as u8,
                        queue_pubkey_index: queue_idx as u8,
                        leaf_index: fresh_compressed_accounts[0].account.leaf_index,
                        prove_by_index: true,
                    },
                    root_index: 0,
                }],
                out_token_data: vec![], // No compressed outputs
                in_lamports: None,
                out_lamports: None,
                in_tlv: None,
                out_tlv: None,
            };

            // Use SDK to create proper account metas
            let meta_config = Transfer2AccountsMetaConfig::new(
                payer.pubkey(),
                packed_accounts.to_account_metas().0, // Get Vec<AccountMeta> from tuple
            );
            let account_metas = get_transfer2_instruction_account_metas(meta_config);

            let ix = solana_sdk::instruction::Instruction {
                program_id: COMPRESSED_TOKEN_PROGRAM_ID.into(),
                accounts: account_metas,
                data: {
                    let mut data = vec![104]; // Transfer2 discriminator
                    data.extend_from_slice(&instruction_data.try_to_vec().unwrap());
                    data
                },
            };

            let signers = vec![&payer]; // Only payer signs, owner should fail validation
            let result = rpc
                .create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
                .await;

            // Should fail with program-level error for missing owner signer
            assert_rpc_error(
                result, 0, 12015, // InvalidSigner - owner not signing
            )
            .unwrap();
            println!("✅ FAIL Test 4b passed: Decompress with owner not signer (program-level)");
        }

        // FAIL Test 5: Decompress - insufficient amount (raw instruction)
        {
            // Create packed accounts properly using SDK
            let mut packed_accounts = PackedAccounts::default();

            // Get state tree info
            let state_tree_info = rpc.get_random_state_tree_info().unwrap();

            // Add accounts in proper order for Transfer2
            let merkle_tree_idx = packed_accounts.insert_or_get(state_tree_info.tree);
            let queue_idx = packed_accounts.insert_or_get(state_tree_info.queue);
            let mint_idx = packed_accounts.insert_or_get_read_only(spl_mint_pda);
            let owner_idx =
                packed_accounts.insert_or_get_config(recipient_keypair.pubkey(), true, false);
            let ctoken_account_idx = packed_accounts.insert_or_get(ctoken_account_pubkey);

            // Manually construct instruction data with invalid amount
            let instruction_data = CompressedTokenInstructionDataTransfer2 {
                with_transaction_hash: false,
                with_lamports_change_account_merkle_tree_index: false,
                lamports_change_account_merkle_tree_index: 0,
                lamports_change_account_owner_index: 0,
                cpi_context: None,
                compressions: Some(vec![Compression {
                    mode: CompressionMode::Decompress,
                    amount: 1000, // Invalid: requesting more than available (only 500)
                    mint: mint_idx as u8,
                    source_or_recipient: ctoken_account_idx as u8, // ctoken account index
                    authority: owner_idx as u8,                    // authority index
                    pool_account_index: 0,
                    pool_index: 0,
                    bump: 0,
                }]),
                proof: None,
                in_token_data: vec![MultiInputTokenDataWithContext {
                    owner: owner_idx as u8, // owner index
                    amount: 500,            // Only 500 available
                    has_delegate: false,
                    delegate: 0,
                    mint: mint_idx as u8, // mint index
                    version: 3,           // ShaFlat version
                    merkle_context: PackedMerkleContext {
                        merkle_tree_pubkey_index: merkle_tree_idx as u8,
                        queue_pubkey_index: queue_idx as u8,
                        leaf_index: fresh_compressed_accounts[0].account.leaf_index,
                        prove_by_index: true,
                    },
                    root_index: 0,
                }],
                out_token_data: vec![MultiTokenTransferOutputData {
                    owner: owner_idx as u8,
                    amount: 0, // Change amount (500 in - 1000 decompress would be negative, should cause underflow)
                    has_delegate: false,
                    delegate: 0,
                    mint: mint_idx as u8,
                    version: 3, // ShaFlat version
                    merkle_tree: merkle_tree_idx as u8,
                }],
                in_lamports: None,
                out_lamports: None,
                in_tlv: None,
                out_tlv: None,
            };

            // Use SDK to create proper account metas
            let meta_config = Transfer2AccountsMetaConfig::new(
                payer.pubkey(),
                packed_accounts.to_account_metas().0, // Get Vec<AccountMeta> from tuple
            );
            let account_metas = get_transfer2_instruction_account_metas(meta_config);

            let ix = solana_sdk::instruction::Instruction {
                program_id: COMPRESSED_TOKEN_PROGRAM_ID.into(),
                accounts: account_metas,
                data: {
                    let mut data = vec![104]; // Transfer2 discriminator
                    data.extend_from_slice(&instruction_data.try_to_vec().unwrap());
                    data
                },
            };

            let signers = vec![&payer, &recipient_keypair];
            let result = rpc
                .create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
                .await;

            assert_rpc_error(
                result, 0, 6005, // ErrorCode::SumCheckFailed = 5 with 6000 prefix
            )
            .unwrap();
            println!("✅ FAIL Test 5 passed: Decompress with insufficient amount");
        }

        // FAIL Test 6: Decompress - invalid ctoken account (destination) (raw instruction)
        {
            let invalid_account = Keypair::new().pubkey(); // Non-existent account

            // Create packed accounts properly using SDK
            let mut packed_accounts = PackedAccounts::default();

            // Get state tree info
            let state_tree_info = rpc.get_random_state_tree_info().unwrap();

            // Add accounts in proper order for Transfer2
            let merkle_tree_idx = packed_accounts.insert_or_get(state_tree_info.tree);
            let queue_idx = packed_accounts.insert_or_get(state_tree_info.queue);
            let mint_idx = packed_accounts.insert_or_get_read_only(spl_mint_pda);
            let owner_idx =
                packed_accounts.insert_or_get_config(recipient_keypair.pubkey(), true, false);
            let invalid_ctoken_account_idx = packed_accounts.insert_or_get(invalid_account); // Invalid account

            // Manually construct instruction data with invalid destination
            let instruction_data = CompressedTokenInstructionDataTransfer2 {
                with_transaction_hash: false,
                with_lamports_change_account_merkle_tree_index: false,
                lamports_change_account_merkle_tree_index: 0,
                lamports_change_account_owner_index: 0,
                cpi_context: None,
                compressions: Some(vec![Compression {
                    mode: CompressionMode::Decompress,
                    amount: 100, // Valid amount
                    mint: mint_idx as u8,
                    source_or_recipient: invalid_ctoken_account_idx as u8, // Invalid ctoken account index
                    authority: owner_idx as u8,
                    pool_account_index: 0,
                    pool_index: 0,
                    bump: 0,
                }]),
                proof: None,
                in_token_data: vec![MultiInputTokenDataWithContext {
                    owner: owner_idx as u8,
                    amount: 500, // Available amount
                    has_delegate: false,
                    delegate: 0,
                    mint: mint_idx as u8,
                    version: 3, // ShaFlat version
                    merkle_context: PackedMerkleContext {
                        merkle_tree_pubkey_index: merkle_tree_idx as u8,
                        queue_pubkey_index: queue_idx as u8,
                        leaf_index: fresh_compressed_accounts[0].account.leaf_index,
                        prove_by_index: true,
                    },
                    root_index: 0,
                }],
                out_token_data: vec![MultiTokenTransferOutputData {
                    owner: owner_idx as u8,
                    amount: 400, // Change amount (500 - 100)
                    has_delegate: false,
                    delegate: 0,
                    mint: mint_idx as u8,
                    version: 3, // ShaFlat version
                    merkle_tree: merkle_tree_idx as u8,
                }],
                in_lamports: None,
                out_lamports: None,
                in_tlv: None,
                out_tlv: None,
            };

            // Use SDK to create proper account metas
            let meta_config = Transfer2AccountsMetaConfig::new(
                payer.pubkey(),
                packed_accounts.to_account_metas().0, // Get Vec<AccountMeta> from tuple
            );
            let account_metas = get_transfer2_instruction_account_metas(meta_config);

            let ix = solana_sdk::instruction::Instruction {
                program_id: COMPRESSED_TOKEN_PROGRAM_ID.into(),
                accounts: account_metas,
                data: {
                    let mut data = vec![104]; // Transfer2 discriminator
                    data.extend_from_slice(&instruction_data.try_to_vec().unwrap());
                    data
                },
            };

            let signers = vec![&payer, &recipient_keypair];
            let result = rpc
                .create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
                .await;

            // Check that it fails with the expected error message about invalid token program ID
            assert!(
                result.is_err(),
                "Expected decompress to fail with invalid account"
            );
            let error_str = format!("{:?}", result.unwrap_err());
            assert!(
                error_str.contains("TransactionError(InstructionError(0, InvalidInstructionData))"),
                "Expected 'Invalid token program ID' error, got: {}",
                error_str
            );
            println!("✅ FAIL Test 6 passed: Decompress with invalid account");
        }

        // FAIL Test 6b: Transfer2 - owner not signer (manual instruction)
        {
            let transfer_recipient = Keypair::new().pubkey();

            // Create packed accounts properly using SDK
            let mut packed_accounts = PackedAccounts::default();

            // Get state tree info
            let state_tree_info = rpc.get_random_state_tree_info().unwrap();

            // Add accounts in proper order for Transfer2
            let merkle_tree_idx = packed_accounts.insert_or_get(state_tree_info.tree);
            let queue_idx = packed_accounts.insert_or_get(state_tree_info.queue);
            let mint_idx = packed_accounts.insert_or_get_read_only(spl_mint_pda);
            let owner_idx = packed_accounts.insert_or_get_read_only(recipient_keypair.pubkey()); // Owner NOT as signer - this is the test
            let recipient_idx = packed_accounts.insert_or_get_read_only(transfer_recipient);

            // Build instruction data for transfer requiring owner signature
            let instruction_data = CompressedTokenInstructionDataTransfer2 {
                with_transaction_hash: false,
                with_lamports_change_account_merkle_tree_index: false,
                lamports_change_account_merkle_tree_index: 0,
                lamports_change_account_owner_index: 0,
                cpi_context: None,
                compressions: None, // No compressions, just transfer
                proof: None,
                in_token_data: vec![MultiInputTokenDataWithContext {
                    owner: owner_idx as u8,
                    amount: 100, // Available amount in compressed account
                    has_delegate: false,
                    delegate: 0,
                    mint: mint_idx as u8,
                    version: 3, // ShaFlat version
                    merkle_context: PackedMerkleContext {
                        merkle_tree_pubkey_index: merkle_tree_idx as u8,
                        queue_pubkey_index: queue_idx as u8,
                        leaf_index: fresh_compressed_accounts[0].account.leaf_index,
                        prove_by_index: true,
                    },
                    root_index: 0,
                }],
                out_token_data: vec![MultiTokenTransferOutputData {
                    owner: recipient_idx as u8,
                    amount: 100,
                    has_delegate: false,
                    delegate: 0,
                    mint: mint_idx as u8,
                    version: 3, // ShaFlat version
                    merkle_tree: merkle_tree_idx as u8,
                }],
                in_lamports: None,
                out_lamports: None,
                in_tlv: None,
                out_tlv: None,
            };

            // Use SDK to create proper account metas
            let meta_config = Transfer2AccountsMetaConfig::new(
                payer.pubkey(),
                packed_accounts.to_account_metas().0, // Get Vec<AccountMeta> from tuple
            );
            let account_metas = get_transfer2_instruction_account_metas(meta_config);

            let ix = solana_sdk::instruction::Instruction {
                program_id: COMPRESSED_TOKEN_PROGRAM_ID.into(),
                accounts: account_metas,
                data: {
                    let mut data = vec![104]; // Transfer2 discriminator
                    data.extend_from_slice(&instruction_data.try_to_vec().unwrap());
                    data
                },
            };

            let signers = vec![&payer]; // Only payer signs, owner should fail validation
            let result = rpc
                .create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
                .await;

            // Should fail with program-level error for missing owner signer
            assert_rpc_error(
                result, 0, 12015, // InvalidSigner - owner not signing
            )
            .unwrap();
            println!("✅ FAIL Test 6b passed: Transfer with owner not signer (program-level)");
        }

        // FAIL Test 7: Transfer2 - invalid owner of compressed account (raw instruction)
        {
            let transfer_recipient = Keypair::new().pubkey();

            // Create packed accounts properly using SDK
            let mut packed_accounts = PackedAccounts::default();

            // Get state tree info
            let state_tree_info = rpc.get_random_state_tree_info().unwrap();

            // Add accounts in proper order for Transfer2
            let merkle_tree_idx = packed_accounts.insert_or_get(state_tree_info.tree);
            let queue_idx = packed_accounts.insert_or_get(state_tree_info.queue);
            let mint_idx = packed_accounts.insert_or_get_read_only(spl_mint_pda);
            let invalid_owner_idx =
                packed_accounts.insert_or_get_config(invalid_owner.pubkey(), true, false); // Invalid owner as signer
            let recipient_idx = packed_accounts.insert_or_get_read_only(transfer_recipient);

            // Manually construct instruction data with invalid owner
            let instruction_data = CompressedTokenInstructionDataTransfer2 {
                with_transaction_hash: false,
                with_lamports_change_account_merkle_tree_index: false,
                lamports_change_account_merkle_tree_index: 0,
                lamports_change_account_owner_index: 0,
                cpi_context: None,
                compressions: None, // No compressions, just transfer
                proof: None,
                in_token_data: vec![MultiInputTokenDataWithContext {
                    owner: invalid_owner_idx as u8, // Use invalid owner index (but real owner in data)
                    amount: 500,                    // Available amount in compressed account
                    has_delegate: false,
                    delegate: 0,
                    mint: mint_idx as u8,
                    version: 3, // ShaFlat version
                    merkle_context: PackedMerkleContext {
                        merkle_tree_pubkey_index: merkle_tree_idx as u8,
                        queue_pubkey_index: queue_idx as u8,
                        leaf_index: fresh_compressed_accounts[0].account.leaf_index,
                        prove_by_index: true,
                    },
                    root_index: 0,
                }],
                out_token_data: vec![
                    MultiTokenTransferOutputData {
                        owner: recipient_idx as u8, // Transfer to recipient
                        amount: 100,                // Transfer amount
                        has_delegate: false,
                        delegate: 0,
                        mint: mint_idx as u8,
                        version: 3, // ShaFlat version
                        merkle_tree: merkle_tree_idx as u8,
                    },
                    MultiTokenTransferOutputData {
                        owner: invalid_owner_idx as u8, // Change back to "owner" (but wrong owner)
                        amount: 400,                    // Change amount (500 - 100)
                        has_delegate: false,
                        delegate: 0,
                        mint: mint_idx as u8,
                        version: 3, // ShaFlat version
                        merkle_tree: merkle_tree_idx as u8,
                    },
                ],
                in_lamports: None,
                out_lamports: None,
                in_tlv: None,
                out_tlv: None,
            };

            // Use SDK to create proper account metas
            let meta_config = Transfer2AccountsMetaConfig::new(
                payer.pubkey(),
                packed_accounts.to_account_metas().0, // Get Vec<AccountMeta> from tuple
            );
            let account_metas = get_transfer2_instruction_account_metas(meta_config);

            let ix = solana_sdk::instruction::Instruction {
                program_id: COMPRESSED_TOKEN_PROGRAM_ID.into(),
                accounts: account_metas,
                data: {
                    let mut data = vec![104]; // Transfer2 discriminator
                    data.extend_from_slice(&instruction_data.try_to_vec().unwrap());
                    data
                },
            };

            let signers = vec![&payer, &invalid_owner]; // Invalid owner signing
            let result = rpc
                .create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
                .await;

            assert_rpc_error(
                result, 0,
                6042, // Error 6042 - hash mismatch due to invalid owner in compressed account
            )
            .unwrap();
            println!("✅ FAIL Test 7 passed: Transfer with invalid owner");
        }

        // FAIL Test 8: Transfer2 - invalid delegate signer (raw instruction)
        {
            // First create a delegation for testing
            let delegate_keypair = Keypair::new();
            airdrop_lamports(&mut rpc, &delegate_keypair.pubkey(), 10_000_000_000)
                .await
                .unwrap();

            transfer2::approve(
                &mut rpc,
                &fresh_compressed_accounts[..1],
                delegate_keypair.pubkey(),
                200u64,
                &recipient_keypair,
                &payer,
            )
            .await
            .unwrap();

            // Get updated accounts with delegation
            let delegated_accounts = rpc
                .indexer()
                .unwrap()
                .get_compressed_token_accounts_by_owner(&recipient_keypair.pubkey(), None, None)
                .await
                .unwrap()
                .value
                .items;

            let delegated_account = delegated_accounts
                .iter()
                .find(|acc| acc.token.delegate == Some(delegate_keypair.pubkey()))
                .expect("Should find delegated account");

            // Try to transfer with invalid delegate - manual instruction
            let invalid_delegate = Keypair::new();
            airdrop_lamports(&mut rpc, &invalid_delegate.pubkey(), 10_000_000_000)
                .await
                .unwrap();

            let transfer_recipient = Keypair::new().pubkey();

            // Create packed accounts properly using SDK
            let mut packed_accounts = PackedAccounts::default();

            // Get state tree info
            let state_tree_info = rpc.get_random_state_tree_info().unwrap();

            // Add accounts in proper order for Transfer2
            let merkle_tree_idx = packed_accounts.insert_or_get(state_tree_info.tree);
            let queue_idx = packed_accounts.insert_or_get(state_tree_info.queue);
            let mint_idx = packed_accounts.insert_or_get_read_only(spl_mint_pda);
            let owner_idx = packed_accounts.insert_or_get_read_only(recipient_keypair.pubkey()); // Real owner (not signer)
            let recipient_idx = packed_accounts.insert_or_get_read_only(transfer_recipient);
            let real_delegate_idx =
                packed_accounts.insert_or_get_read_only(delegate_keypair.pubkey()); // Real delegate (not signer)

            // Manually construct instruction data with invalid delegate signing
            let instruction_data = CompressedTokenInstructionDataTransfer2 {
                with_transaction_hash: false,
                with_lamports_change_account_merkle_tree_index: false,
                lamports_change_account_merkle_tree_index: 0,
                lamports_change_account_owner_index: 0,
                cpi_context: None,
                compressions: None, // No compressions, just transfer
                proof: None,
                in_token_data: vec![MultiInputTokenDataWithContext {
                    owner: owner_idx as u8, // Real owner (not signing)
                    amount: 200,            // Available delegate amount
                    has_delegate: true,
                    delegate: real_delegate_idx as u8, // Keep real delegate in data (for correct hash)
                    mint: mint_idx as u8,
                    version: 3, // ShaFlat version
                    merkle_context: PackedMerkleContext {
                        merkle_tree_pubkey_index: merkle_tree_idx as u8,
                        queue_pubkey_index: queue_idx as u8,
                        leaf_index: delegated_account.account.leaf_index,
                        prove_by_index: true,
                    },
                    root_index: 0,
                }],
                out_token_data: vec![
                    MultiTokenTransferOutputData {
                        owner: recipient_idx as u8, // Transfer to recipient
                        amount: 100,                // Transfer amount
                        has_delegate: false,
                        delegate: 0,
                        mint: mint_idx as u8,
                        version: 3, // ShaFlat version
                        merkle_tree: merkle_tree_idx as u8,
                    },
                    MultiTokenTransferOutputData {
                        owner: owner_idx as u8, // Change back to owner
                        amount: 100,            // Remaining delegate amount (200 - 100)
                        has_delegate: true,
                        delegate: real_delegate_idx as u8, // Keep real delegate
                        mint: mint_idx as u8,
                        version: 3, // ShaFlat version
                        merkle_tree: merkle_tree_idx as u8,
                    },
                ],
                in_lamports: None,
                out_lamports: None,
                in_tlv: None,
                out_tlv: None,
            };

            // Use SDK to create proper account metas
            let meta_config = Transfer2AccountsMetaConfig::new(
                payer.pubkey(),
                packed_accounts.to_account_metas().0, // Get Vec<AccountMeta> from tuple
            );
            let account_metas = get_transfer2_instruction_account_metas(meta_config);

            let ix = solana_sdk::instruction::Instruction {
                program_id: COMPRESSED_TOKEN_PROGRAM_ID.into(),
                accounts: account_metas,
                data: {
                    let mut data = vec![104]; // Transfer2 discriminator
                    data.extend_from_slice(&instruction_data.try_to_vec().unwrap());
                    data
                },
            };

            let signers = vec![&payer]; // Only payer signs, real delegate should fail validation
            let result = rpc
                .create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
                .await;

            assert_rpc_error(
                result, 0, 12015, // Error 12015 (0x2eef) - InvalidSigner for delegate
            )
            .unwrap();
            println!("✅ FAIL Test 8 passed: Transfer with invalid delegate");
        }

        // FAIL Test 9: Transfer2 - insufficient amount (manual instruction)
        {
            let transfer_recipient = Keypair::new().pubkey();

            // Create packed accounts properly using SDK
            let mut packed_accounts = PackedAccounts::default();

            // Get state tree info
            let state_tree_info = rpc.get_random_state_tree_info().unwrap();

            // Add accounts in proper order for Transfer2
            let merkle_tree_idx = packed_accounts.insert_or_get(state_tree_info.tree);
            let queue_idx = packed_accounts.insert_or_get(state_tree_info.queue);
            let mint_idx =
                packed_accounts.insert_or_get_read_only(fresh_compressed_accounts[0].token.mint);
            let owner_idx =
                packed_accounts.insert_or_get_config(recipient_keypair.pubkey(), true, false); // Owner as signer
            let recipient_idx = packed_accounts.insert_or_get_read_only(transfer_recipient);

            // Build instruction data with amount exceeding available balance
            let instruction_data = CompressedTokenInstructionDataTransfer2 {
                with_transaction_hash: false,
                with_lamports_change_account_merkle_tree_index: false,
                lamports_change_account_merkle_tree_index: 0,
                lamports_change_account_owner_index: 0,
                cpi_context: None,
                compressions: None, // No compressions, just transfer
                proof: None,
                in_token_data: vec![MultiInputTokenDataWithContext {
                    owner: owner_idx as u8,
                    amount: 500, // Available amount in compressed account
                    has_delegate: false,
                    delegate: 0,
                    mint: mint_idx as u8,
                    version: 3, // ShaFlat version
                    merkle_context: PackedMerkleContext {
                        merkle_tree_pubkey_index: merkle_tree_idx as u8,
                        queue_pubkey_index: queue_idx as u8,
                        leaf_index: fresh_compressed_accounts[0].account.leaf_index,
                        prove_by_index: true,
                    },
                    root_index: 0,
                }],
                out_token_data: vec![MultiTokenTransferOutputData {
                    owner: recipient_idx as u8,
                    amount: 1000u64, // More than available (500) - this should cause SumCheckFailed
                    has_delegate: false,
                    delegate: 0,
                    mint: mint_idx as u8,
                    version: 3, // ShaFlat version
                    merkle_tree: merkle_tree_idx as u8,
                }],
                in_lamports: None,
                out_lamports: None,
                in_tlv: None,
                out_tlv: None,
            };

            // Use SDK to create proper account metas
            let meta_config = Transfer2AccountsMetaConfig::new(
                payer.pubkey(),
                packed_accounts.to_account_metas().0, // Get Vec<AccountMeta> from tuple
            );
            let account_metas = get_transfer2_instruction_account_metas(meta_config);

            let ix = solana_sdk::instruction::Instruction {
                program_id: COMPRESSED_TOKEN_PROGRAM_ID.into(),
                accounts: account_metas,
                data: {
                    let mut data = vec![104]; // Transfer2 discriminator
                    data.extend_from_slice(&instruction_data.try_to_vec().unwrap());
                    data
                },
            };

            let signers = vec![&payer, &recipient_keypair]; // Real owner signing
            let result = rpc
                .create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
                .await;

            assert_rpc_error(
                result, 0, 6002, // Got error code 6002 in test output
            )
            .unwrap();
            println!("✅ FAIL Test 9 passed: Transfer with insufficient amount (program-level)");
        }

        // FAIL Test 10: CompressAndClose - without signer (account meta modification pattern)
        {
            use light_token_client::instructions::transfer2::{
                create_generic_transfer2_instruction, CompressAndCloseInput,
                Transfer2InstructionType,
            };

            // Create the instruction using the SDK first with valid signer
            let output_queue = rpc
                .get_random_state_tree_info()
                .unwrap()
                .get_output_pubkey()
                .unwrap();
            let mut ix = create_generic_transfer2_instruction(
                &mut rpc,
                vec![Transfer2InstructionType::CompressAndClose(
                    CompressAndCloseInput {
                        solana_ctoken_account: ctoken_account_pubkey,
                        authority: recipient_keypair.pubkey(),
                        output_queue,
                        destination: Some(recipient_keypair.pubkey()),
                    },
                )],
                payer.pubkey(),
            )
            .await
            .unwrap(); // This should succeed in creating the instruction

            // Apply account meta modification pattern:
            // Mark authority as non-signer to test program validation
            for account_meta in &mut ix.accounts {
                if account_meta.pubkey == recipient_keypair.pubkey() && account_meta.is_signer {
                    account_meta.is_signer = false; // Mark as non-signer to test program validation
                }
            }

            let signers = vec![&payer]; // Only payer signs, not the authority
            let result = rpc
                .create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
                .await;

            assert_rpc_error(
                result, 0, 12015, // Authority signer check failed: InvalidSigner
            )
            .unwrap();
            println!("✅ FAIL Test 10 passed: CompressAndClose without signer (program-level)");
        }

        println!("✅ All failing tests completed successfully!");
    }
}

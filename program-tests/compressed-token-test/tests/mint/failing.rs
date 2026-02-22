// #![cfg(feature = "test-sbf")]

use anchor_lang::prelude::borsh::BorshDeserialize;
use light_client::indexer::Indexer;
use light_compressed_token::MINT_CREATION_FEE;
use light_compressed_token_sdk::compressed_token::create_compressed_mint::{
    derive_mint_compressed_address, find_mint_address,
};
use light_program_test::{utils::assert::assert_rpc_error, LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    actions::{
        create_mint,
        legacy::instructions::mint_action::{MintActionType, MintToRecipient},
    },
    assert_mint_action::assert_mint_action,
    assert_mint_creation_fee,
    mint_assert::assert_compressed_mint_account,
    Rpc,
};
use light_token::instruction::{CompressibleParams, CreateAssociatedTokenAccount};
use light_token_interface::state::{extensions::AdditionalMetadata, Mint};
use serial_test::serial;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};

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
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree_pubkey);

    // Find mint PDA for the rest of the test
    let (spl_mint_pda, _) = find_mint_address(&mint_seed.pubkey());
    // 1. Create compressed mint with both authorities
    {
        create_mint(
        &mut rpc,
        &mint_seed,
        8, // decimals
        &mint_authority,
        Some(freeze_authority.pubkey()),
        Some(light_token_interface::instructions::extensions::token_metadata::TokenMetadataInstructionData {
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
            .value
            .unwrap();
        assert_compressed_mint_account(
        &compressed_mint_account,
        compressed_mint_address,
        spl_mint_pda,
        8,
        mint_authority.pubkey(),
        freeze_authority.pubkey(),
        Some(light_token_interface::instructions::extensions::token_metadata::TokenMetadataInstructionData {
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

    // 2. FAIL - Create mint with duplicate metadata keys
    {
        let duplicate_mint_seed = Keypair::new();
        let result = light_test_utils::actions::create_mint(
            &mut rpc,
            &duplicate_mint_seed, // Use new mint seed
            8, // decimals
            &mint_authority,
            Some(freeze_authority.pubkey()),
            Some(light_token_interface::instructions::extensions::token_metadata::TokenMetadataInstructionData {
                update_authority: Some(metadata_authority.pubkey().into()),
                name: "Test Token".as_bytes().to_vec(),
                symbol: "TEST".as_bytes().to_vec(),
                uri: "https://example.com/token.json".as_bytes().to_vec(),
                additional_metadata: Some(vec![
                    AdditionalMetadata {
                        key: vec![1, 2, 3, 4], // First key
                        value: vec![2u8; 5]
                    },
                    AdditionalMetadata {
                        key: vec![5, 6, 7, 8], // Different key
                        value: vec![3u8; 10]
                    },
                    AdditionalMetadata {
                        key: vec![1, 2, 3, 4], // DUPLICATE of first key
                        value: vec![4u8; 15]
                    }
                ]),
            }),
            &payer,
        )
        .await;

        assert_rpc_error(
            result, 0, 18040, // TokenError::DuplicateMetadataKey = 18040
        )
        .unwrap();
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

    // 3. MintToCompressed with invalid mint authority
    {
        let result = light_test_utils::actions::mint_to_compressed(
            &mut rpc,
            spl_mint_pda,
            vec![
                light_token_interface::instructions::mint_action::Recipient::new(
                    Keypair::new().pubkey(),
                    1000u64,
                ),
            ],
            light_token_interface::state::TokenDataVersion::V2,
            &invalid_mint_authority, // Invalid authority
            &payer,
        )
        .await;

        assert_rpc_error(
            result, 0, 6018, // light_compressed_token::ErrorCode::InvalidAuthorityMint
        )
        .unwrap();
    }

    // 4. SUCCEED - MintToCompressed with valid mint authority
    {
        // Get pre-transaction compressed mint state
        let pre_compressed_mint_account = rpc
            .indexer()
            .unwrap()
            .get_compressed_account(compressed_mint_address, None)
            .await
            .unwrap()
            .value
            .unwrap();
        let pre_compressed_mint: Mint = BorshDeserialize::deserialize(
            &mut pre_compressed_mint_account.data.unwrap().data.as_slice(),
        )
        .unwrap();

        let recipient = Keypair::new().pubkey();
        let result = light_test_utils::actions::mint_to_compressed(
            &mut rpc,
            spl_mint_pda,
            vec![
                light_token_interface::instructions::mint_action::Recipient::new(
                    recipient, 1000u64,
                ),
            ],
            light_token_interface::state::TokenDataVersion::V2,
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
            vec![MintActionType::MintTo {
                recipients: vec![MintToRecipient {
                    recipient,
                    amount: 1000u64,
                }],
                token_account_version: light_token_interface::state::TokenDataVersion::V2 as u8,
            }],
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
        .value
        .unwrap();

    // 5. UpdateMintAuthority with invalid mint authority
    {
        let result = light_test_utils::actions::update_mint_authority(
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
            result, 0, 6018, // light_compressed_token::ErrorCode::InvalidAuthorityMint
        )
        .unwrap();
    }

    // 6. SUCCEED - UpdateMintAuthority with valid mint authority
    {
        // Get fresh compressed mint account
        let compressed_mint_account = rpc
            .indexer()
            .unwrap()
            .get_compressed_account(compressed_mint_address, None)
            .await
            .unwrap()
            .value
            .unwrap();
        let pre_compressed_mint: Mint = BorshDeserialize::deserialize(
            &mut compressed_mint_account.data.unwrap().data.as_slice(),
        )
        .unwrap();

        let result = light_test_utils::actions::update_mint_authority(
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
            vec![MintActionType::UpdateMintAuthority {
                new_authority: Some(new_mint_authority.pubkey()),
            }],
        )
        .await;
    }

    // 7. UpdateFreezeAuthority with invalid freeze authority
    {
        // Get fresh compressed mint account after mint authority update
        let compressed_mint_account = rpc
            .indexer()
            .unwrap()
            .get_compressed_account(compressed_mint_address, None)
            .await
            .unwrap()
            .value
            .unwrap();

        let result = light_test_utils::actions::update_freeze_authority(
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
            result, 0, 6018, // InvalidAuthorityMint error code
        )
        .unwrap();
    }

    // 8. SUCCEED - UpdateFreezeAuthority with valid freeze authority
    {
        // Get fresh compressed mint account
        let compressed_mint_account = rpc
            .indexer()
            .unwrap()
            .get_compressed_account(compressed_mint_address, None)
            .await
            .unwrap()
            .value
            .unwrap();
        let pre_compressed_mint: Mint = BorshDeserialize::deserialize(
            &mut compressed_mint_account.data.unwrap().data.as_slice(),
        )
        .unwrap();

        let result = light_test_utils::actions::update_freeze_authority(
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
            vec![MintActionType::UpdateFreezeAuthority {
                new_authority: Some(new_freeze_authority.pubkey()),
            }],
        )
        .await;
    }

    // 9. MintToCToken with invalid mint authority
    {
        // Decompress mint first so CToken ATAs can be created
        light_test_utils::actions::mint_action_comprehensive(
            &mut rpc,
            &mint_seed,
            &new_mint_authority, // Use new_mint_authority since we updated it in step 6
            &payer,
            Some(light_test_utils::actions::legacy::instructions::mint_action::DecompressMintParams::default()),
            false,
            vec![],
            vec![],
            None,
            None,
            None,
        )
        .await
        .unwrap();

        // Create a ctoken account first
        let recipient = Keypair::new();

        let create_ata_ix =
            CreateAssociatedTokenAccount::new(payer.pubkey(), recipient.pubkey(), spl_mint_pda)
                .instruction()
                .unwrap();

        rpc.create_and_send_transaction(&[create_ata_ix], &payer.pubkey(), &[&payer])
            .await
            .unwrap();

        // Try to mint with invalid authority
        let result = light_test_utils::actions::mint_action_comprehensive(
            &mut rpc,
            &mint_seed,
            &invalid_mint_authority, // Invalid authority
            &payer,
            None,   // decompress_mint
            false,  // compress_and_close_mint
            vec![], // No compressed recipients
            vec![
                light_token_interface::instructions::mint_action::Recipient::new(
                    recipient.pubkey(),
                    1000u64,
                ),
            ], // Mint to decompressed
            None,   // No mint authority update
            None,   // No freeze authority update
            None,   // Not creating new mint
        )
        .await;

        assert_rpc_error(
            result, 0, 6018, // light_compressed_token::ErrorCode::InvalidAuthorityMint
        )
        .unwrap();
    }

    // 10. SUCCEED - MintToCToken with valid mint authority
    {
        // Get pre-transaction state from on-chain CMint (since mint was decompressed)
        let cmint_account_data = rpc
            .get_account(spl_mint_pda)
            .await
            .unwrap()
            .expect("CMint should exist after decompression");
        let pre_compressed_mint: Mint =
            BorshDeserialize::deserialize(&mut cmint_account_data.data.as_slice()).unwrap();

        // Create a new recipient for successful mint
        let recipient2 = Keypair::new();

        let create_ata_ix2 =
            CreateAssociatedTokenAccount::new(payer.pubkey(), recipient2.pubkey(), spl_mint_pda)
                .instruction()
                .unwrap();

        rpc.create_and_send_transaction(&[create_ata_ix2], &payer.pubkey(), &[&payer])
            .await
            .unwrap();

        let recipient_ata =
            light_token::instruction::derive_token_ata(&recipient2.pubkey(), &spl_mint_pda);

        // Try to mint with valid NEW authority (since we updated it)
        let result = light_test_utils::actions::mint_action_comprehensive(
            &mut rpc,
            &mint_seed,
            &new_mint_authority, // Valid NEW authority after update
            &payer,
            None,   // decompress_mint
            false,  // compress_and_close_mint
            vec![], // No compressed recipients
            vec![
                light_token_interface::instructions::mint_action::Recipient::new(
                    recipient2.pubkey(),
                    2000u64,
                ),
            ], // Mint to decompressed
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
            vec![MintActionType::MintToCToken {
                account: recipient_ata,
                amount: 2000u64,
            }],
        )
        .await;
    }

    // 11. UpdateMetadataField with invalid metadata authority
    {
        let result = light_test_utils::actions::mint_action(
            &mut rpc,
            light_test_utils::actions::legacy::instructions::mint_action::MintActionParams {
                compressed_mint_address,
                mint_seed: mint_seed.pubkey(),
                authority: invalid_metadata_authority.pubkey(), // Invalid authority
                payer: payer.pubkey(),
                actions: vec![MintActionType::UpdateMetadataField {
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
            result, 0, 6018, // light_compressed_token::ErrorCode::InvalidAuthorityMint
        )
        .unwrap();
    }

    // 12. SUCCEED - UpdateMetadataField with valid metadata authority
    {
        // Get pre-transaction state from on-chain CMint (since mint was decompressed)
        let cmint_account_data = rpc
            .get_account(spl_mint_pda)
            .await
            .unwrap()
            .expect("CMint should exist");
        let pre_compressed_mint: Mint =
            BorshDeserialize::deserialize(&mut cmint_account_data.data.as_slice()).unwrap();

        let actions = vec![MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 0, // 0 = Name field
            key: vec![],   // Empty for Name field
            value: "Updated Token Name".as_bytes().to_vec(),
        }];

        let result = light_test_utils::actions::mint_action(
            &mut rpc,
            light_test_utils::actions::legacy::instructions::mint_action::MintActionParams {
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

    // 13. UpdateMetadataAuthority with invalid metadata authority
    {
        let result = light_test_utils::actions::mint_action(
            &mut rpc,
            light_test_utils::actions::legacy::instructions::mint_action::MintActionParams {
                compressed_mint_address,
                mint_seed: mint_seed.pubkey(),
                authority: invalid_metadata_authority.pubkey(), // Invalid authority
                payer: payer.pubkey(),
                actions: vec![MintActionType::UpdateMetadataAuthority {
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
            result, 0, 6018, // light_compressed_token::ErrorCode::InvalidAuthorityMint
        )
        .unwrap();
    }

    // 14. SUCCEED - UpdateMetadataAuthority with valid metadata authority
    {
        // Get pre-transaction state from on-chain CMint (since mint was decompressed)
        let cmint_account_data = rpc
            .get_account(spl_mint_pda)
            .await
            .unwrap()
            .expect("CMint should exist");
        let pre_compressed_mint: Mint =
            BorshDeserialize::deserialize(&mut cmint_account_data.data.as_slice()).unwrap();

        let actions = vec![MintActionType::UpdateMetadataAuthority {
            extension_index: 0,
            new_authority: new_metadata_authority.pubkey(),
        }];

        let result = light_test_utils::actions::mint_action(
            &mut rpc,
            light_test_utils::actions::legacy::instructions::mint_action::MintActionParams {
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

    // 15. RemoveMetadataKey with invalid metadata authority
    {
        let result = light_test_utils::actions::mint_action(
            &mut rpc,
            light_test_utils::actions::legacy::instructions::mint_action::MintActionParams {
                compressed_mint_address,
                mint_seed: mint_seed.pubkey(),
                authority: invalid_metadata_authority.pubkey(), // Invalid authority
                payer: payer.pubkey(),
                actions: vec![MintActionType::RemoveMetadataKey {
                    extension_index: 0,
                    key: vec![1, 2, 3, 4], // The key we added in additional_metadata
                    idempotent: 0,         // 0 = false
                }],
                new_mint: None,
            },
            &invalid_metadata_authority,
            &payer,
            None,
        )
        .await;

        assert_rpc_error(
            result, 0, 6018, // light_compressed_token::ErrorCode::InvalidAuthorityMint
        )
        .unwrap();
    }

    // 16. SUCCEED - RemoveMetadataKey with valid metadata authority
    {
        // Get pre-transaction state from on-chain CMint (since mint was decompressed)
        let cmint_account_data = rpc
            .get_account(spl_mint_pda)
            .await
            .unwrap()
            .expect("CMint should exist");
        let pre_compressed_mint: Mint =
            BorshDeserialize::deserialize(&mut cmint_account_data.data.as_slice()).unwrap();

        let actions = vec![MintActionType::RemoveMetadataKey {
            extension_index: 0,
            key: vec![1, 2, 3, 4], // The key we added in additional_metadata
            idempotent: 0,         // 0 = false
        }];

        let result = light_test_utils::actions::mint_action(
            &mut rpc,
            light_test_utils::actions::legacy::instructions::mint_action::MintActionParams {
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

    // 17. SUCCEED - RemoveMetadataKey idempotent (try to remove same key again)
    {
        // Get pre-transaction state from on-chain CMint (since mint was decompressed)
        let cmint_account_data = rpc
            .get_account(spl_mint_pda)
            .await
            .unwrap()
            .expect("CMint should exist");
        let pre_compressed_mint: Mint =
            BorshDeserialize::deserialize(&mut cmint_account_data.data.as_slice()).unwrap();

        let actions = vec![MintActionType::RemoveMetadataKey {
            extension_index: 0,
            key: vec![1, 2, 3, 4], // Same key, already removed
            idempotent: 1,         // 1 = true (won't error if key doesn't exist)
        }];

        let result = light_test_utils::actions::mint_action(
            &mut rpc,
            light_test_utils::actions::legacy::instructions::mint_action::MintActionParams {
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

/// Test that mint_action fails when max_top_up is exceeded during MintToCToken.
/// Creates a compressible Light Token ATA with pre_pay_num_epochs = 0 (no prepaid rent),
/// which requires rent top-up on any mint write. Setting max_top_up = 1 (too low)
/// should trigger MaxTopUpExceeded error (18043).
#[tokio::test]
#[serial]
async fn test_mint_to_ctoken_max_top_up_exceeded() {
    use light_compressed_account::instruction_data::traits::LightInstructionData;
    use light_compressed_token_sdk::compressed_token::{
        create_compressed_mint::derive_mint_compressed_address, mint_action::MintActionMetaConfig,
    };
    use light_token_interface::{
        instructions::mint_action::{
            MintActionCompressedInstructionData, MintToAction, MintWithContext,
        },
        state::TokenDataVersion,
        LIGHT_TOKEN_PROGRAM_ID,
    };

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();

    let payer = Keypair::new();
    rpc.airdrop_lamports(&payer.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let mint_seed = Keypair::new();
    let mint_authority = Keypair::new();
    rpc.airdrop_lamports(&mint_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree_pubkey);
    let (spl_mint_pda, _) = find_mint_address(&mint_seed.pubkey());

    // 1. Create compressed mint
    create_mint(
        &mut rpc,
        &mint_seed,
        8, // decimals
        &mint_authority,
        None, // no freeze authority
        None, // no metadata
        &payer,
    )
    .await
    .unwrap();

    // 1b. Decompress mint so CToken ATA can be created
    light_test_utils::actions::mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &mint_authority,
        &payer,
        Some(light_test_utils::actions::legacy::instructions::mint_action::DecompressMintParams::default()),
        false,
        vec![],
        vec![],
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // 2. Create compressible Light Token ATA with pre_pay_num_epochs = 0 (NO prepaid rent)
    let recipient = Keypair::new();

    let compressible_params = CompressibleParams {
        compressible_config: rpc
            .test_accounts
            .funding_pool_config
            .compressible_config_pda,
        rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
        pre_pay_num_epochs: 0, // NO prepaid epochs - needs top-up immediately
        lamports_per_write: Some(1000),
        compress_to_account_pubkey: None,
        token_account_version: TokenDataVersion::ShaFlat,
        compression_only: true,
    };

    let create_ata_ix =
        CreateAssociatedTokenAccount::new(payer.pubkey(), recipient.pubkey(), spl_mint_pda)
            .with_compressible(compressible_params)
            .instruction()
            .unwrap();

    rpc.create_and_send_transaction(&[create_ata_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    let ctoken_ata = light_token::instruction::derive_token_ata(&recipient.pubkey(), &spl_mint_pda);

    // 3. Build MintToCToken instruction with max_top_up = 1 (too low)
    // Get current mint state from on-chain CMint (since mint was decompressed)
    let cmint_account_data = rpc
        .get_account(spl_mint_pda)
        .await
        .unwrap()
        .expect("CMint should exist after decompression");

    let compressed_mint: light_token_interface::state::Mint =
        BorshDeserialize::deserialize(&mut cmint_account_data.data.as_slice()).unwrap();

    // Get compressed account for proof (still exists but with mint_decompressed=true)
    let compressed_mint_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    // Get validity proof
    let rpc_proof_result = rpc
        .get_validity_proof(vec![compressed_mint_account.hash], vec![], None)
        .await
        .unwrap()
        .value;

    let compressed_mint_inputs = MintWithContext {
        prove_by_index: rpc_proof_result.accounts[0].root_index.proof_by_index(),
        leaf_index: compressed_mint_account.leaf_index,
        root_index: rpc_proof_result.accounts[0]
            .root_index
            .root_index()
            .unwrap_or_default(),
        address: compressed_mint_address,
        mint: Some(compressed_mint.try_into().unwrap()),
    };

    // Build instruction data with max_top_up = 1 (too low to cover rent top-up)
    let instruction_data =
        MintActionCompressedInstructionData::new(compressed_mint_inputs, rpc_proof_result.proof.0)
            .with_mint_to(MintToAction {
                account_index: 0,
                amount: 1000u64,
            })
            .with_max_top_up(1); // max_top_up = 1 (1,000 lamports budget, still too low for rent top-up)

    // Build account metas
    let config = MintActionMetaConfig::new(
        payer.pubkey(),
        mint_authority.pubkey(),
        rpc_proof_result.accounts[0].tree_info.tree,
        rpc_proof_result.accounts[0].tree_info.queue,
        rpc_proof_result.accounts[0].tree_info.queue,
    )
    .with_token_accounts(vec![ctoken_ata]);

    let account_metas = config.to_account_metas();

    // Serialize instruction data
    let data = instruction_data.data().unwrap();

    // Build final instruction
    let ix = Instruction {
        program_id: LIGHT_TOKEN_PROGRAM_ID.into(),
        accounts: account_metas,
        data,
    };

    // 4. Execute and expect MaxTopUpExceeded (18043)
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &mint_authority])
        .await;

    assert_rpc_error(
        result, 0, 18043, // TokenError::MaxTopUpExceeded = 18043
    )
    .unwrap();
}

/// Test that mint_signer must be a signer when creating a compressed mint
#[tokio::test]
#[serial]
async fn test_create_mint_non_signer_mint_signer() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();

    let payer = Keypair::new();
    rpc.airdrop_lamports(&payer.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let mint_seed = Keypair::new();
    let mint_authority = Keypair::new();

    // Create the instruction using the helper function
    let mut instruction =
        light_test_utils::actions::legacy::instructions::create_mint::create_compressed_mint_instruction(
            &mut rpc,
            &mint_seed,
            8, // decimals
            mint_authority.pubkey(),
            None, // freeze authority
            payer.pubkey(),
            None, // metadata
        )
        .await
        .unwrap();

    // Manually override the account metas to make mint_signer a non-signer
    // Account ordering: [0] light_system_program, [1] mint_signer, [2] authority, ...
    // Find and modify the mint_signer account meta at index 1
    // The SDK creates it as AccountMeta::new_readonly(mint_signer, true)
    // We want to change it to AccountMeta::new_readonly(mint_signer, false)
    if let Some(mint_signer_meta) = instruction.accounts.get_mut(1) {
        // Verify it's the mint_seed
        assert_eq!(mint_signer_meta.pubkey, mint_seed.pubkey());
        // Change is_signer from true to false to bypass runtime checks
        *mint_signer_meta = AccountMeta::new_readonly(mint_seed.pubkey(), false);
    }

    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer, &mint_authority], // Note: NOT signing with mint_seed
        blockhash,
    );

    let result = rpc.process_transaction(transaction).await;

    // Should fail with AccountError::InvalidSigner (error code 20009)
    assert_rpc_error(
        result, 0, 20009, // AccountError::InvalidSigner = 20009
    )
    .unwrap();
}

/// Test that CompressAndCloseMint must be the only action in the instruction.
/// Attempting to combine CompressAndCloseMint with UpdateMintAuthority should fail.
#[tokio::test]
#[serial]
async fn test_compress_and_close_mint_must_be_only_action() {
    use light_compressed_token_sdk::compressed_token::create_compressed_mint::derive_mint_compressed_address;
    use light_compressible::rent::SLOTS_PER_EPOCH;
    use light_program_test::program_test::TestRpc;
    use light_test_utils::actions::legacy::instructions::mint_action::DecompressMintParams;

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();

    let payer = Keypair::new();
    rpc.airdrop_lamports(&payer.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let mint_seed = Keypair::new();
    let mint_authority = Keypair::new();
    rpc.airdrop_lamports(&mint_authority.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree_pubkey);

    // 1. Create compressed mint with Mint (decompressed)
    light_test_utils::actions::mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &mint_authority,
        &payer,
        Some(DecompressMintParams::default()),
        false,
        vec![],
        vec![],
        None,
        None,
        Some(
            light_test_utils::actions::legacy::instructions::mint_action::NewMint {
                decimals: 9,
                supply: 0,
                mint_authority: mint_authority.pubkey(),
                freeze_authority: None,
                metadata: None,
                version: 3,
            },
        ),
    )
    .await
    .unwrap();

    // Warp to epoch 2 so that rent expires
    rpc.warp_to_slot(SLOTS_PER_EPOCH * 2).unwrap();

    // 2. Try to combine CompressAndCloseMint with UpdateMintAuthority
    let new_authority = Keypair::new();
    let result = light_test_utils::actions::mint_action(
        &mut rpc,
        light_test_utils::actions::legacy::instructions::mint_action::MintActionParams {
            compressed_mint_address,
            mint_seed: mint_seed.pubkey(),
            authority: mint_authority.pubkey(),
            payer: payer.pubkey(),
            actions: vec![
                MintActionType::CompressAndCloseMint { idempotent: false },
                MintActionType::UpdateMintAuthority {
                    new_authority: Some(new_authority.pubkey()),
                },
            ],
            new_mint: None,
        },
        &mint_authority,
        &payer,
        None,
    )
    .await;

    // Should fail with CompressAndCloseMintMustBeOnlyAction (error code 6169)
    assert_rpc_error(
        result, 0, 6169, // CompressAndCloseMintMustBeOnlyAction
    )
    .unwrap();
}

/// Tests that the mint creation fee is charged from fee_payer to rent_sponsor.
/// Also tests that the fee is charged even without any actions (compressed-only mint).
#[tokio::test]
#[serial]
async fn test_mint_creation_fee_charged() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let rent_sponsor = rpc.test_accounts.funding_pool_config.rent_sponsor_pda;
    let mint_seed = Keypair::new();
    let mint_authority = Keypair::new();

    // Capture balances before
    let rent_sponsor_before = rpc
        .get_account(rent_sponsor)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let fee_payer_before = rpc
        .get_account(payer.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Create compressed mint (no actions)
    create_mint(
        &mut rpc,
        &mint_seed,
        6, // decimals
        &mint_authority,
        None,
        None,
        &payer,
    )
    .await
    .unwrap();

    // Capture balances after
    let rent_sponsor_after = rpc
        .get_account(rent_sponsor)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let fee_payer_after = rpc
        .get_account(payer.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Assert fee was credited to rent_sponsor
    assert_mint_creation_fee(rent_sponsor_before, rent_sponsor_after);

    // Assert fee was debited from fee_payer (use <= because tx base fees are also deducted)
    assert!(
        fee_payer_after <= fee_payer_before - MINT_CREATION_FEE,
        "Fee payer should have paid at least {} lamports mint creation fee (before={}, after={})",
        MINT_CREATION_FEE,
        fee_payer_before,
        fee_payer_after,
    );
}

#![cfg(feature = "test-sbf")]

use anchor_lang::prelude::borsh::BorshDeserialize;
use light_client::indexer::Indexer;
use light_compressed_token_sdk::instructions::{
    derive_compressed_mint_address, find_spl_mint_address,
};
use light_ctoken_types::state::{extensions::AdditionalMetadata, CompressedMint};
use light_program_test::{utils::assert::assert_rpc_error, LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    assert_mint_action::assert_mint_action, mint_assert::assert_compressed_mint_account, Rpc,
};
use light_token_client::actions::create_mint;
use serial_test::serial;
use solana_sdk::{
    instruction::AccountMeta, signature::Keypair, signer::Signer, transaction::Transaction,
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
            .value
            .unwrap();
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

    // 2. FAIL - Create mint with duplicate metadata keys
    {
        let duplicate_mint_seed = Keypair::new();
        let result = create_mint(
            &mut rpc,
            &duplicate_mint_seed, // Use new mint seed
            8, // decimals
            &mint_authority,
            Some(freeze_authority.pubkey()),
            Some(light_ctoken_types::instructions::extensions::token_metadata::TokenMetadataInstructionData {
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
            result, 0, 18040, // CTokenError::DuplicateMetadataKey = 18040
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
        .value
        .unwrap();

    // 5. UpdateMintAuthority with invalid mint authority
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

    // 9. MintToCToken with invalid mint authority
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

    // 10. SUCCEED - MintToCToken with valid mint authority
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

    // 11. UpdateMetadataField with invalid metadata authority
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

    // 12. SUCCEED - UpdateMetadataField with valid metadata authority
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

    // 13. UpdateMetadataAuthority with invalid metadata authority
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

    // 14. SUCCEED - UpdateMetadataAuthority with valid metadata authority
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

    // 15. RemoveMetadataKey with invalid metadata authority
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

    // 16. SUCCEED - RemoveMetadataKey with valid metadata authority
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

    // 17. SUCCEED - RemoveMetadataKey idempotent (try to remove same key again)
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
        light_token_client::instructions::create_mint::create_compressed_mint_instruction(
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

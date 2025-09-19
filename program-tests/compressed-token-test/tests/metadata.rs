// #![cfg(feature = "test-sbf")]

use light_compressed_token_sdk::instructions::{
    derive_compressed_mint_address, mint_action::MintActionType,
};
use light_ctoken_types::{
    instructions::extensions::token_metadata::TokenMetadataInstructionData, state::ExtensionStruct,
};
use light_program_test::{LightProgramTest, ProgramTestConfig, Rpc};
use light_test_utils::assert_metadata::{
    assert_metadata_error, assert_metadata_not_exists, assert_metadata_state,
    assert_mint_operation_result, create_additional_metadata, create_expected_metadata_state,
    get_actual_mint_state,
};
use light_token_client::{
    actions::{create_mint, mint_action},
    instructions::mint_action::MintActionParams,
};
use serial_test::serial;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};

/// Shared test context for metadata tests
struct MetadataTestContext {
    pub payer: Keypair,
    pub mint_seed: Keypair,
    pub mint_authority: Keypair,
    pub freeze_authority: Pubkey,
    pub compressed_mint_address: [u8; 32],
}

/// Set up a test environment for metadata operations
async fn setup_metadata_test() -> (LightProgramTest, MetadataTestContext) {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Test parameters
    let mint_seed = Keypair::new();
    let mint_authority = Keypair::new();
    let freeze_authority = Pubkey::new_unique();
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_compressed_mint_address(&mint_seed.pubkey(), &address_tree_pubkey);

    // Fund all signers upfront (following established pattern)
    rpc.airdrop_lamports(&mint_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let context = MetadataTestContext {
        payer,
        mint_seed,
        mint_authority,
        freeze_authority,
        compressed_mint_address,
    };

    (rpc, context)
}

/// Create a mint with metadata for testing
async fn create_mint_with_metadata(
    rpc: &mut LightProgramTest,
    context: &MetadataTestContext,
    metadata: TokenMetadataInstructionData,
) -> Result<Signature, light_client::rpc::RpcError> {
    create_mint(
        rpc,
        &context.mint_seed,
        6u8, // decimals
        &context.mint_authority,
        Some(context.freeze_authority),
        Some(metadata),
        &context.payer,
    )
    .await
}

/// Create standard test metadata with 4 additional keys
fn create_test_metadata(update_authority: Option<Pubkey>) -> TokenMetadataInstructionData {
    let additional_metadata = vec![
        create_additional_metadata("website", "https://mytoken.com"),
        create_additional_metadata("category", "DeFi"),
        create_additional_metadata("creator", "TokenMaker Inc."),
        create_additional_metadata("license", "MIT"),
    ];

    TokenMetadataInstructionData {
        update_authority: update_authority.map(|auth| auth.into()),
        name: b"Test Token".to_vec(),
        symbol: b"TEST".to_vec(),
        uri: b"https://example.com/token.json".to_vec(),
        additional_metadata: Some(additional_metadata),
    }
}

// ============================================================================
// FUNCTIONAL TESTS
// ============================================================================

/// Test:
/// 1. SUCCESS: Create mint with additional metadata keys
/// 2. SUCCESS: Verify all metadata fields and additional keys are correctly stored
#[tokio::test]
#[serial]
async fn test_metadata_create_with_additional_keys() -> Result<(), light_client::rpc::RpcError> {
    let (mut rpc, context) = setup_metadata_test().await;

    // Create mint with metadata
    let metadata = create_test_metadata(Some(context.mint_authority.pubkey()));
    create_mint_with_metadata(&mut rpc, &context, metadata).await?;

    // Assert complete metadata state matches expected
    let expected_state = create_expected_metadata_state(
        Some(context.mint_authority.pubkey()),
        "Test Token",
        "TEST",
        "https://example.com/token.json",
        vec![
            create_additional_metadata("website", "https://mytoken.com"),
            create_additional_metadata("category", "DeFi"),
            create_additional_metadata("creator", "TokenMaker Inc."),
            create_additional_metadata("license", "MIT"),
        ],
    );

    let actual_metadata =
        assert_metadata_state(&mut rpc, context.compressed_mint_address, &expected_state).await;

    // Verify specific properties that should be true after creation
    assert_eq!(
        actual_metadata.additional_metadata.len(),
        4,
        "Should have exactly 4 additional metadata entries"
    );
    assert!(
        actual_metadata.update_authority != light_compressed_account::Pubkey::from([0u8; 32]),
        "Update authority should be set (non-zero)"
    );
    Ok(())
}

/// Test:
/// 1. SUCCESS: Update metadata name field
/// 2. SUCCESS: Update metadata symbol field
/// 3. SUCCESS: Verify field updates are applied correctly
#[tokio::test]
#[serial]
async fn test_metadata_field_updates() -> Result<(), light_client::rpc::RpcError> {
    let (mut rpc, context) = setup_metadata_test().await;

    // === ARRANGE ===
    let metadata = create_test_metadata(Some(context.mint_authority.pubkey()));
    create_mint_with_metadata(&mut rpc, &context, metadata).await?;

    // Capture complete mint state before operation
    let mint_before = get_actual_mint_state(&mut rpc, context.compressed_mint_address).await;

    // === ACT & ASSERT - Update name field ===
    let update_name_actions = vec![MintActionType::UpdateMetadataField {
        extension_index: 0,
        field_type: 0, // Name field
        key: vec![],
        value: b"Updated Test Token".to_vec(),
    }];

    let params = MintActionParams {
        compressed_mint_address: context.compressed_mint_address,
        mint_seed: context.mint_seed.pubkey(),
        authority: context.mint_authority.pubkey(),
        payer: context.payer.pubkey(),
        actions: update_name_actions,
        new_mint: None,
    };

    let _name_update_result = mint_action(
        &mut rpc,
        params,
        &context.mint_authority,
        &context.payer,
        None,
    )
    .await?;

    // Assert complete mint state equals before state + expected name change
    assert_mint_operation_result(
        &mut rpc,
        context.compressed_mint_address,
        &mint_before,
        |mint| {
            // Apply expected change: update name field in metadata
            if let Some(ref mut extensions) = mint.extensions {
                if let Some(ExtensionStruct::TokenMetadata(ref mut metadata)) =
                    extensions.get_mut(0)
                {
                    metadata.name = b"Updated Test Token".to_vec();
                }
            }
        },
    )
    .await;

    // === ACT & ASSERT - Update symbol field ===
    // Capture mint state after name update (for second operation)
    let mint_after_name_update =
        get_actual_mint_state(&mut rpc, context.compressed_mint_address).await;

    let update_symbol_actions = vec![MintActionType::UpdateMetadataField {
        extension_index: 0,
        field_type: 1, // Symbol field
        key: vec![],
        value: b"UPDT".to_vec(),
    }];

    let params = MintActionParams {
        compressed_mint_address: context.compressed_mint_address,
        mint_seed: context.mint_seed.pubkey(),
        authority: context.mint_authority.pubkey(),
        payer: context.payer.pubkey(),
        actions: update_symbol_actions,
        new_mint: None,
    };

    let _symbol_update_result = mint_action(
        &mut rpc,
        params,
        &context.mint_authority,
        &context.payer,
        None,
    )
    .await?;

    // Assert complete mint state equals after-name-update state + symbol change
    assert_mint_operation_result(
        &mut rpc,
        context.compressed_mint_address,
        &mint_after_name_update,
        |mint| {
            // Apply expected change: update symbol field in metadata
            if let Some(ref mut extensions) = mint.extensions {
                if let Some(ExtensionStruct::TokenMetadata(ref mut metadata)) =
                    extensions.get_mut(0)
                {
                    metadata.symbol = b"UPDT".to_vec();
                }
            }
        },
    )
    .await;
    Ok(())
}

/// Test:
/// 1. SUCCESS: Update metadata authority from A to B
/// 2. SUCCESS: Update metadata authority from B to C
/// 3. SUCCESS: Revoke metadata authority (C to None)
/// 4. SUCCESS: Verify authority changes are applied correctly
#[tokio::test]
#[serial]
async fn test_metadata_authority_management() -> Result<(), light_client::rpc::RpcError> {
    let (mut rpc, context) = setup_metadata_test().await;

    // === ARRANGE ===
    let metadata = create_test_metadata(Some(context.mint_authority.pubkey()));
    create_mint_with_metadata(&mut rpc, &context, metadata).await?;

    // Capture complete mint state before operations
    let mint_before_authority_changes =
        get_actual_mint_state(&mut rpc, context.compressed_mint_address).await;

    // Create additional authorities for testing
    let second_authority = Keypair::new();
    let third_authority = Keypair::new();
    rpc.airdrop_lamports(&second_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    rpc.airdrop_lamports(&third_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // === ACT & ASSERT - Update authority from A to B ===
    let update_authority_actions = vec![MintActionType::UpdateMetadataAuthority {
        extension_index: 0,
        new_authority: second_authority.pubkey(),
    }];

    let params = MintActionParams {
        compressed_mint_address: context.compressed_mint_address,
        mint_seed: context.mint_seed.pubkey(),
        authority: context.mint_authority.pubkey(),
        payer: context.payer.pubkey(),
        actions: update_authority_actions,
        new_mint: None,
    };

    mint_action(
        &mut rpc,
        params,
        &context.mint_authority,
        &context.payer,
        None,
    )
    .await
    .unwrap();

    // Assert complete mint state equals before state + authority change
    assert_mint_operation_result(
        &mut rpc,
        context.compressed_mint_address,
        &mint_before_authority_changes,
        |mint| {
            // Apply expected change: update authority
            if let Some(ref mut extensions) = mint.extensions {
                if let Some(ExtensionStruct::TokenMetadata(ref mut metadata)) =
                    extensions.get_mut(0)
                {
                    metadata.update_authority = second_authority.pubkey().into();
                }
            }
        },
    )
    .await;

    // === ACT & ASSERT - Update authority from B to C ===
    let update_authority_actions = vec![MintActionType::UpdateMetadataAuthority {
        extension_index: 0,
        new_authority: third_authority.pubkey(),
    }];

    let params = MintActionParams {
        compressed_mint_address: context.compressed_mint_address,
        mint_seed: context.mint_seed.pubkey(),
        authority: second_authority.pubkey(), // Use second authority
        payer: context.payer.pubkey(),
        actions: update_authority_actions,
        new_mint: None,
    };

    mint_action(&mut rpc, params, &second_authority, &context.payer, None)
        .await
        .unwrap();

    // Verify authority updated to third_authority
    let expected_after_second_update = create_expected_metadata_state(
        Some(third_authority.pubkey()), // Updated
        "Test Token",
        "TEST",
        "https://example.com/token.json",
        vec![
            create_additional_metadata("website", "https://mytoken.com"),
            create_additional_metadata("category", "DeFi"),
            create_additional_metadata("creator", "TokenMaker Inc."),
            create_additional_metadata("license", "MIT"),
        ],
    );

    assert_metadata_state(
        &mut rpc,
        context.compressed_mint_address,
        &expected_after_second_update,
    )
    .await;

    // === ACT & ASSERT - Revoke authority (C to None) ===
    let revoke_authority_actions = vec![MintActionType::UpdateMetadataAuthority {
        extension_index: 0,
        new_authority: solana_sdk::pubkey::Pubkey::from([0u8; 32]), // Zero pubkey for None
    }];

    let params = MintActionParams {
        compressed_mint_address: context.compressed_mint_address,
        mint_seed: context.mint_seed.pubkey(),
        authority: third_authority.pubkey(), // Use third authority
        payer: context.payer.pubkey(),
        actions: revoke_authority_actions,
        new_mint: None,
    };

    mint_action(&mut rpc, params, &third_authority, &context.payer, None)
        .await
        .unwrap();

    // Verify authority revoked to None
    let expected_after_revocation = create_expected_metadata_state(
        None, // Revoked
        "Test Token",
        "TEST",
        "https://example.com/token.json",
        vec![
            create_additional_metadata("website", "https://mytoken.com"),
            create_additional_metadata("category", "DeFi"),
            create_additional_metadata("creator", "TokenMaker Inc."),
            create_additional_metadata("license", "MIT"),
        ],
    );

    assert_metadata_state(
        &mut rpc,
        context.compressed_mint_address,
        &expected_after_revocation,
    )
    .await;
    Ok(())
}

/// Test:
/// 1. SUCCESS: Remove single metadata key
/// 2. SUCCESS: Remove multiple metadata keys in batch
/// 3. SUCCESS: Remove last remaining metadata key
/// 4. SUCCESS: Verify key removal operations are applied correctly
#[tokio::test]
#[serial]
async fn test_metadata_key_removal_operations() -> Result<(), light_client::rpc::RpcError> {
    let (mut rpc, context) = setup_metadata_test().await;

    // === ARRANGE ===
    let metadata = create_test_metadata(Some(context.mint_authority.pubkey()));
    create_mint_with_metadata(&mut rpc, &context, metadata).await?;

    // === ACT & ASSERT - Remove single key ===
    let remove_single_key_actions = vec![MintActionType::RemoveMetadataKey {
        extension_index: 0,
        key: b"license".to_vec(),
        idempotent: 0, // Not idempotent
    }];

    let params = MintActionParams {
        compressed_mint_address: context.compressed_mint_address,
        mint_seed: context.mint_seed.pubkey(),
        authority: context.mint_authority.pubkey(),
        payer: context.payer.pubkey(),
        actions: remove_single_key_actions,
        new_mint: None,
    };

    mint_action(
        &mut rpc,
        params,
        &context.mint_authority,
        &context.payer,
        None,
    )
    .await
    .unwrap();

    // Verify "license" key was removed
    let expected_after_single_removal = create_expected_metadata_state(
        Some(context.mint_authority.pubkey()),
        "Test Token",
        "TEST",
        "https://example.com/token.json",
        vec![
            create_additional_metadata("website", "https://mytoken.com"),
            create_additional_metadata("category", "DeFi"),
            create_additional_metadata("creator", "TokenMaker Inc."),
            // "license" removed
        ],
    );

    assert_metadata_state(
        &mut rpc,
        context.compressed_mint_address,
        &expected_after_single_removal,
    )
    .await;

    // === ACT & ASSERT - Remove multiple keys ===
    let remove_multiple_keys_actions = vec![
        MintActionType::RemoveMetadataKey {
            extension_index: 0,
            key: b"website".to_vec(),
            idempotent: 0,
        },
        MintActionType::RemoveMetadataKey {
            extension_index: 0,
            key: b"category".to_vec(),
            idempotent: 0,
        },
    ];

    let params = MintActionParams {
        compressed_mint_address: context.compressed_mint_address,
        mint_seed: context.mint_seed.pubkey(),
        authority: context.mint_authority.pubkey(),
        payer: context.payer.pubkey(),
        actions: remove_multiple_keys_actions,
        new_mint: None,
    };

    mint_action(
        &mut rpc,
        params,
        &context.mint_authority,
        &context.payer,
        None,
    )
    .await
    .unwrap();

    // Verify both keys were removed
    let expected_after_multiple_removal = create_expected_metadata_state(
        Some(context.mint_authority.pubkey()),
        "Test Token",
        "TEST",
        "https://example.com/token.json",
        vec![
            create_additional_metadata("creator", "TokenMaker Inc."),
            // "website" and "category" removed
        ],
    );

    assert_metadata_state(
        &mut rpc,
        context.compressed_mint_address,
        &expected_after_multiple_removal,
    )
    .await;

    // === ACT & ASSERT - Remove last key ===
    let remove_last_key_actions = vec![MintActionType::RemoveMetadataKey {
        extension_index: 0,
        key: b"creator".to_vec(),
        idempotent: 0,
    }];

    let params = MintActionParams {
        compressed_mint_address: context.compressed_mint_address,
        mint_seed: context.mint_seed.pubkey(),
        authority: context.mint_authority.pubkey(),
        payer: context.payer.pubkey(),
        actions: remove_last_key_actions,
        new_mint: None,
    };

    mint_action(
        &mut rpc,
        params,
        &context.mint_authority,
        &context.payer,
        None,
    )
    .await
    .unwrap();

    // Verify all additional metadata keys are gone
    let expected_after_all_removal = create_expected_metadata_state(
        Some(context.mint_authority.pubkey()),
        "Test Token",
        "TEST",
        "https://example.com/token.json",
        vec![], // All additional metadata removed
    );

    assert_metadata_state(
        &mut rpc,
        context.compressed_mint_address,
        &expected_after_all_removal,
    )
    .await;
    Ok(())
}

/// Test:
/// 1. SUCCESS: Remove metadata key in single transaction
/// 2. SUCCESS: Update metadata field in same transaction
/// 3. SUCCESS: Update metadata authority in same transaction
/// 4. SUCCESS: Verify all operations completed atomically
#[tokio::test]
#[serial]
async fn test_metadata_combined_operations() -> Result<(), light_client::rpc::RpcError> {
    let (mut rpc, context) = setup_metadata_test().await;

    // === ARRANGE ===
    let metadata = create_test_metadata(Some(context.mint_authority.pubkey()));
    create_mint_with_metadata(&mut rpc, &context, metadata).await?;

    let new_authority = Keypair::new();
    rpc.airdrop_lamports(&new_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // === ACT - Combined operations: remove key + update field + update authority ===
    let combined_actions = vec![
        // Remove the "license" key first
        MintActionType::RemoveMetadataKey {
            extension_index: 0,
            key: b"license".to_vec(),
            idempotent: 0,
        },
        // Update the name field
        MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 0, // Name field
            key: vec![],
            value: b"Combined Update Token".to_vec(),
        },
        // Update metadata authority (must be last since new authority can't be used in same tx)
        MintActionType::UpdateMetadataAuthority {
            extension_index: 0,
            new_authority: new_authority.pubkey(),
        },
    ];

    let params = MintActionParams {
        compressed_mint_address: context.compressed_mint_address,
        mint_seed: context.mint_seed.pubkey(),
        authority: context.mint_authority.pubkey(),
        payer: context.payer.pubkey(),
        actions: combined_actions,
        new_mint: None,
    };

    mint_action(
        &mut rpc,
        params,
        &context.mint_authority,
        &context.payer,
        None,
    )
    .await
    .unwrap();

    // === ASSERT - Verify all operations completed atomically ===
    let expected_after_combined = create_expected_metadata_state(
        Some(new_authority.pubkey()), // Authority updated
        "Combined Update Token",      // Name updated
        "TEST",
        "https://example.com/token.json",
        vec![
            create_additional_metadata("website", "https://mytoken.com"),
            create_additional_metadata("category", "DeFi"),
            create_additional_metadata("creator", "TokenMaker Inc."),
            // "license" removed
        ],
    );

    assert_metadata_state(
        &mut rpc,
        context.compressed_mint_address,
        &expected_after_combined,
    )
    .await;
    Ok(())
}

/// Test:
/// 1. SUCCESS: Create mint with metadata
/// 2. SUCCESS: Setup multiple authorities for workflow
/// 3. SUCCESS: Verify complete end-to-end metadata lifecycle
#[tokio::test]
#[serial]
async fn test_metadata_comprehensive_workflow() -> Result<(), light_client::rpc::RpcError> {
    let (mut rpc, context) = setup_metadata_test().await;

    // === STEP 1: Create mint with metadata ===
    let metadata = create_test_metadata(Some(context.mint_authority.pubkey()));
    create_mint_with_metadata(&mut rpc, &context, metadata).await?;

    let expected_initial_state = create_expected_metadata_state(
        Some(context.mint_authority.pubkey()),
        "Test Token",
        "TEST",
        "https://example.com/token.json",
        vec![
            create_additional_metadata("website", "https://mytoken.com"),
            create_additional_metadata("category", "DeFi"),
            create_additional_metadata("creator", "TokenMaker Inc."),
            create_additional_metadata("license", "MIT"),
        ],
    );

    assert_metadata_state(
        &mut rpc,
        context.compressed_mint_address,
        &expected_initial_state,
    )
    .await;

    // === STEP 2-8: Follow the comprehensive workflow pattern ===
    // This test verifies that the complete workflow from the original test
    // now works with proper assertions instead of debug prints

    // Create authorities for the workflow
    let second_authority = Keypair::new();
    let third_authority = Keypair::new();
    let fourth_authority = Keypair::new();

    rpc.airdrop_lamports(&second_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    rpc.airdrop_lamports(&third_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    rpc.airdrop_lamports(&fourth_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // === STEP 2: Combined operations - Remove key, update field, change authority ===
    let combined_step2_actions = vec![
        MintActionType::RemoveMetadataKey {
            extension_index: 0,
            key: b"license".to_vec(),
            idempotent: 0,
        },
        MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 0, // Name
            key: vec![],
            value: b"Workflow Token".to_vec(),
        },
        MintActionType::UpdateMetadataAuthority {
            extension_index: 0,
            new_authority: second_authority.pubkey(),
        },
    ];

    mint_action(
        &mut rpc,
        MintActionParams {
            compressed_mint_address: context.compressed_mint_address,
            mint_seed: context.mint_seed.pubkey(),
            authority: context.mint_authority.pubkey(),
            payer: context.payer.pubkey(),
            actions: combined_step2_actions,
            new_mint: None,
        },
        &context.mint_authority,
        &context.payer,
        None,
    )
    .await?;

    // Assert: authority changed, name updated, "license" removed
    let expected_after_step2 = create_expected_metadata_state(
        Some(second_authority.pubkey()),
        "Workflow Token",
        "TEST",
        "https://example.com/token.json",
        vec![
            create_additional_metadata("website", "https://mytoken.com"),
            create_additional_metadata("category", "DeFi"),
            create_additional_metadata("creator", "TokenMaker Inc."),
        ],
    );
    assert_metadata_state(
        &mut rpc,
        context.compressed_mint_address,
        &expected_after_step2,
    )
    .await;

    // === STEP 3: Update symbol field with second authority ===
    mint_action(
        &mut rpc,
        MintActionParams {
            compressed_mint_address: context.compressed_mint_address,
            mint_seed: context.mint_seed.pubkey(),
            authority: second_authority.pubkey(),
            payer: context.payer.pubkey(),
            actions: vec![MintActionType::UpdateMetadataField {
                extension_index: 0,
                field_type: 1, // Symbol
                key: vec![],
                value: b"WF".to_vec(),
            }],
            new_mint: None,
        },
        &second_authority,
        &context.payer,
        None,
    )
    .await?;

    // === STEP 4: Transfer authority to third authority ===
    mint_action(
        &mut rpc,
        MintActionParams {
            compressed_mint_address: context.compressed_mint_address,
            mint_seed: context.mint_seed.pubkey(),
            authority: second_authority.pubkey(),
            payer: context.payer.pubkey(),
            actions: vec![MintActionType::UpdateMetadataAuthority {
                extension_index: 0,
                new_authority: third_authority.pubkey(),
            }],
            new_mint: None,
        },
        &second_authority,
        &context.payer,
        None,
    )
    .await?;

    // === STEP 5: Update URI field with third authority ===
    mint_action(
        &mut rpc,
        MintActionParams {
            compressed_mint_address: context.compressed_mint_address,
            mint_seed: context.mint_seed.pubkey(),
            authority: third_authority.pubkey(),
            payer: context.payer.pubkey(),
            actions: vec![MintActionType::UpdateMetadataField {
                extension_index: 0,
                field_type: 2, // URI
                key: vec![],
                value: b"https://workflow.example.com/token.json".to_vec(),
            }],
            new_mint: None,
        },
        &third_authority,
        &context.payer,
        None,
    )
    .await?;

    // === STEP 6: Remove another metadata key ===
    mint_action(
        &mut rpc,
        MintActionParams {
            compressed_mint_address: context.compressed_mint_address,
            mint_seed: context.mint_seed.pubkey(),
            authority: third_authority.pubkey(),
            payer: context.payer.pubkey(),
            actions: vec![MintActionType::RemoveMetadataKey {
                extension_index: 0,
                key: b"website".to_vec(),
                idempotent: 0,
            }],
            new_mint: None,
        },
        &third_authority,
        &context.payer,
        None,
    )
    .await?;

    // === STEP 7: Transfer to fourth authority, then immediately revoke ===
    let combined_step7_actions = vec![MintActionType::UpdateMetadataAuthority {
        extension_index: 0,
        new_authority: fourth_authority.pubkey(),
    }];

    mint_action(
        &mut rpc,
        MintActionParams {
            compressed_mint_address: context.compressed_mint_address,
            mint_seed: context.mint_seed.pubkey(),
            authority: third_authority.pubkey(),
            payer: context.payer.pubkey(),
            actions: combined_step7_actions,
            new_mint: None,
        },
        &third_authority,
        &context.payer,
        None,
    )
    .await?;

    // === STEP 8: Revoke authority entirely ===
    mint_action(
        &mut rpc,
        MintActionParams {
            compressed_mint_address: context.compressed_mint_address,
            mint_seed: context.mint_seed.pubkey(),
            authority: fourth_authority.pubkey(),
            payer: context.payer.pubkey(),
            actions: vec![MintActionType::UpdateMetadataAuthority {
                extension_index: 0,
                new_authority: Pubkey::default(), // Revoke authority
            }],
            new_mint: None,
        },
        &fourth_authority,
        &context.payer,
        None,
    )
    .await?;

    // Verify final state where authority is None and metadata exists
    let expected_final = create_expected_metadata_state(
        None, // Authority revoked
        "Workflow Token",
        "WF",
        "https://workflow.example.com/token.json",
        vec![
            create_additional_metadata("category", "DeFi"),
            create_additional_metadata("creator", "TokenMaker Inc."),
        ],
    );
    assert_metadata_state(&mut rpc, context.compressed_mint_address, &expected_final).await;

    // This validates the complete end-to-end workflow
    Ok(())
}

// ============================================================================
// ERROR TESTS
// ============================================================================

/// Test:
/// 1. FAIL: Update metadata field with invalid authority
/// 2. FAIL: Update metadata authority with invalid authority
/// 3. FAIL: Remove metadata key with invalid authority
#[tokio::test]
#[serial]
async fn test_metadata_invalid_authority_fails() -> Result<(), light_client::rpc::RpcError> {
    let (mut rpc, context) = setup_metadata_test().await;

    // === ARRANGE ===
    let metadata = create_test_metadata(Some(context.mint_authority.pubkey()));
    create_mint_with_metadata(&mut rpc, &context, metadata).await?;

    let wrong_authority = Keypair::new();
    rpc.airdrop_lamports(&wrong_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // === ACT & ASSERT - Field update with wrong authority should fail ===
    let field_update_actions = vec![MintActionType::UpdateMetadataField {
        extension_index: 0,
        field_type: 0,
        key: vec![],
        value: b"Should Fail".to_vec(),
    }];

    let params = MintActionParams {
        compressed_mint_address: context.compressed_mint_address,
        mint_seed: context.mint_seed.pubkey(),
        authority: wrong_authority.pubkey(), // Wrong authority
        payer: context.payer.pubkey(),
        actions: field_update_actions,
        new_mint: None,
    };

    let result = mint_action(&mut rpc, params, &wrong_authority, &context.payer, None).await;
    assert_metadata_error(result, 18); // MintActionInvalidMintAuthority

    // === ACT & ASSERT - Authority update with wrong authority should fail ===
    let authority_update_actions = vec![MintActionType::UpdateMetadataAuthority {
        extension_index: 0,
        new_authority: wrong_authority.pubkey(),
    }];

    let params = MintActionParams {
        compressed_mint_address: context.compressed_mint_address,
        mint_seed: context.mint_seed.pubkey(),
        authority: wrong_authority.pubkey(), // Wrong authority
        payer: context.payer.pubkey(),
        actions: authority_update_actions,
        new_mint: None,
    };

    let result = mint_action(&mut rpc, params, &wrong_authority, &context.payer, None).await;
    assert_metadata_error(result, 18); // MintActionInvalidMintAuthority

    // === ACT & ASSERT - Key removal with wrong authority should fail ===
    let key_removal_actions = vec![MintActionType::RemoveMetadataKey {
        extension_index: 0,
        key: b"website".to_vec(),
        idempotent: 0,
    }];

    let params = MintActionParams {
        compressed_mint_address: context.compressed_mint_address,
        mint_seed: context.mint_seed.pubkey(),
        authority: wrong_authority.pubkey(), // Wrong authority
        payer: context.payer.pubkey(),
        actions: key_removal_actions,
        new_mint: None,
    };

    let result = mint_action(&mut rpc, params, &wrong_authority, &context.payer, None).await;
    assert_metadata_error(result, 18); // MintActionInvalidMintAuthority
    Ok(())
}

/// Test:
/// 1. SUCCESS: Revoke metadata authority to None
/// 2. FAIL: Attempt metadata field update after authority revocation
#[tokio::test]
#[serial]
async fn test_metadata_operations_after_authority_revocation_fail(
) -> Result<(), light_client::rpc::RpcError> {
    let (mut rpc, context) = setup_metadata_test().await;

    // === ARRANGE ===
    let metadata = create_test_metadata(Some(context.mint_authority.pubkey()));
    create_mint_with_metadata(&mut rpc, &context, metadata).await?;

    // Revoke authority to None
    let revoke_authority_actions = vec![MintActionType::UpdateMetadataAuthority {
        extension_index: 0,
        new_authority: Pubkey::default(), // None
    }];

    let params = MintActionParams {
        compressed_mint_address: context.compressed_mint_address,
        mint_seed: context.mint_seed.pubkey(),
        authority: context.mint_authority.pubkey(),
        payer: context.payer.pubkey(),
        actions: revoke_authority_actions,
        new_mint: None,
    };

    mint_action(
        &mut rpc,
        params,
        &context.mint_authority,
        &context.payer,
        None,
    )
    .await
    .unwrap();

    // === ACT & ASSERT - Any operation should fail after revocation ===
    let field_update_actions = vec![MintActionType::UpdateMetadataField {
        extension_index: 0,
        field_type: 0,
        key: vec![],
        value: b"Should Fail".to_vec(),
    }];

    let params = MintActionParams {
        compressed_mint_address: context.compressed_mint_address,
        mint_seed: context.mint_seed.pubkey(),
        authority: context.mint_authority.pubkey(), // Even original authority should fail
        payer: context.payer.pubkey(),
        actions: field_update_actions,
        new_mint: None,
    };

    let result = mint_action(
        &mut rpc,
        params,
        &context.mint_authority,
        &context.payer,
        None,
    )
    .await;
    assert_metadata_error(result, 18); // MintActionInvalidMintAuthority
    Ok(())
}

/// Test:
/// 1. FAIL: Remove nonexistent key with non-idempotent setting
/// 2. SUCCESS: Remove nonexistent key with idempotent setting
#[tokio::test]
#[serial]
async fn test_metadata_remove_nonexistent_key_scenarios() -> Result<(), light_client::rpc::RpcError>
{
    let (mut rpc, context) = setup_metadata_test().await;

    // === ARRANGE ===
    let metadata = create_test_metadata(Some(context.mint_authority.pubkey()));
    create_mint_with_metadata(&mut rpc, &context, metadata).await?;

    // === ACT & ASSERT - Non-idempotent removal of nonexistent key should fail ===
    let remove_nonexistent_key_actions = vec![MintActionType::RemoveMetadataKey {
        extension_index: 0,
        key: b"nonexistent".to_vec(),
        idempotent: 0, // Not idempotent - should fail
    }];

    let params = MintActionParams {
        compressed_mint_address: context.compressed_mint_address,
        mint_seed: context.mint_seed.pubkey(),
        authority: context.mint_authority.pubkey(),
        payer: context.payer.pubkey(),
        actions: remove_nonexistent_key_actions,
        new_mint: None,
    };

    let result = mint_action(
        &mut rpc,
        params,
        &context.mint_authority,
        &context.payer,
        None,
    )
    .await;
    // This should fail with some error (exact error code depends on implementation)
    assert!(
        result.is_err(),
        "Expected removal of nonexistent key to fail"
    );

    // === ACT & ASSERT - Idempotent removal of nonexistent key should succeed ===
    let remove_nonexistent_key_idempotent_actions = vec![MintActionType::RemoveMetadataKey {
        extension_index: 0,
        key: b"nonexistent".to_vec(),
        idempotent: 1, // Idempotent - should succeed
    }];

    let params = MintActionParams {
        compressed_mint_address: context.compressed_mint_address,
        mint_seed: context.mint_seed.pubkey(),
        authority: context.mint_authority.pubkey(),
        payer: context.payer.pubkey(),
        actions: remove_nonexistent_key_idempotent_actions,
        new_mint: None,
    };

    let result = mint_action(
        &mut rpc,
        params,
        &context.mint_authority,
        &context.payer,
        None,
    )
    .await;
    assert!(
        result.is_ok(),
        "Expected idempotent removal of nonexistent key to succeed"
    );
    Ok(())
}

/// Test:
/// 1. FAIL: Update metadata field with out-of-bounds extension index
#[tokio::test]
#[serial]
async fn test_metadata_invalid_extension_index_fails() -> Result<(), light_client::rpc::RpcError> {
    let (mut rpc, context) = setup_metadata_test().await;

    // === ARRANGE ===
    let metadata = create_test_metadata(Some(context.mint_authority.pubkey()));
    create_mint_with_metadata(&mut rpc, &context, metadata).await?;

    // === ACT & ASSERT - Operation with out-of-bounds extension index should fail ===
    let invalid_index_actions = vec![MintActionType::UpdateMetadataField {
        extension_index: 99, // Invalid index
        field_type: 0,
        key: vec![],
        value: b"Should Fail".to_vec(),
    }];

    let params = MintActionParams {
        compressed_mint_address: context.compressed_mint_address,
        mint_seed: context.mint_seed.pubkey(),
        authority: context.mint_authority.pubkey(),
        payer: context.payer.pubkey(),
        actions: invalid_index_actions,
        new_mint: None,
    };

    let result = mint_action(
        &mut rpc,
        params,
        &context.mint_authority,
        &context.payer,
        None,
    )
    .await;
    // Should fail with invalid extension index error
    assert!(
        result.is_err(),
        "Expected operation with invalid extension index to fail"
    );
    Ok(())
}

/// Test:
/// 1. SUCCESS: Create mint without metadata extensions
/// 2. FAIL: Attempt metadata operation on mint without extensions
#[tokio::test]
#[serial]
async fn test_metadata_operations_without_extensions_fail(
) -> Result<(), light_client::rpc::RpcError> {
    let (mut rpc, context) = setup_metadata_test().await;

    // === ARRANGE - Create mint WITHOUT metadata ===
    create_mint(
        &mut rpc,
        &context.mint_seed,
        6u8,
        &context.mint_authority,
        Some(context.freeze_authority),
        None, // No metadata
        &context.payer,
    )
    .await?;

    // Verify no metadata exists
    assert_metadata_not_exists(&mut rpc, context.compressed_mint_address).await;

    // === ACT & ASSERT - Metadata operation on mint without extensions should fail ===
    let field_update_actions = vec![MintActionType::UpdateMetadataField {
        extension_index: 0,
        field_type: 0,
        key: vec![],
        value: b"Should Fail".to_vec(),
    }];

    let params = MintActionParams {
        compressed_mint_address: context.compressed_mint_address,
        mint_seed: context.mint_seed.pubkey(),
        authority: context.mint_authority.pubkey(),
        payer: context.payer.pubkey(),
        actions: field_update_actions,
        new_mint: None,
    };

    let result = mint_action(
        &mut rpc,
        params,
        &context.mint_authority,
        &context.payer,
        None,
    )
    .await;
    // Should fail with missing extension error
    assert!(
        result.is_err(),
        "Expected metadata operation on mint without extensions to fail"
    );
    Ok(())
}

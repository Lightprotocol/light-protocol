//! Tests for CMint (decompressed mint) resize operations.
//!
//! These tests verify the resize path in `serialize_decompressed_mint` which handles:
//! - Account resize when metadata size changes
//! - Rent exemption calculation and deficit handling
//! - Compressible top-up calculation
//!
//! Two main scenarios:
//! - Scenario A: CMint already exists (decompressed at start of transaction)
//! - Scenario B: CMint decompressed in the same transaction (DecompressMint + other actions)

use anchor_lang::prelude::borsh::BorshDeserialize;
use light_client::indexer::Indexer;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::{assert_mint_action::assert_mint_action, Rpc};
use light_token::{
    compressed_token::create_compressed_mint::{derive_mint_compressed_address, find_mint_address},
    instruction::{derive_token_ata, CompressibleParams, CreateAssociatedTokenAccount},
};
use light_token_client::{
    actions::create_mint,
    instructions::mint_action::{
        DecompressMintParams, MintActionParams, MintActionType, MintToRecipient,
    },
};
use light_token_interface::{
    instructions::extensions::token_metadata::TokenMetadataInstructionData,
    state::{extensions::AdditionalMetadata, Mint, TokenDataVersion},
};
use serial_test::serial;
use solana_sdk::{signature::Keypair, signer::Signer};

// ============================================================================
// SCENARIO A: CMint Already Exists (Decompressed at Start)
// ============================================================================

/// Test UpdateMetadataField with longer value triggers resize grow on existing CMint.
#[tokio::test]
#[serial]
async fn test_cmint_update_metadata_grow() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let mint_seed = Keypair::new();
    let authority = Keypair::new();
    rpc.airdrop_lamports(&authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree_pubkey);
    let (spl_mint_pda, _cmint_bump) = find_mint_address(&mint_seed.pubkey());

    // 1. Create compressed mint with small metadata
    create_mint(
        &mut rpc,
        &mint_seed,
        8,
        &authority,
        Some(authority.pubkey()),
        Some(TokenMetadataInstructionData {
            update_authority: Some(authority.pubkey().into()),
            name: "A".as_bytes().to_vec(),
            symbol: "B".as_bytes().to_vec(),
            uri: "C".as_bytes().to_vec(),
            additional_metadata: None,
        }),
        &payer,
    )
    .await
    .unwrap();

    // 2. Decompress to CMint (creates on-chain account)
    light_token_client::actions::mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &authority,
        &payer,
        Some(DecompressMintParams::default()),
        false,
        vec![],
        vec![],
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // 3. Get pre-state from CMint on-chain account
    let cmint_account_data = rpc
        .get_account(spl_mint_pda)
        .await
        .unwrap()
        .expect("CMint should exist");
    let pre_mint: Mint =
        BorshDeserialize::deserialize(&mut cmint_account_data.data.as_slice()).unwrap();

    // 4. UpdateMetadataField with LONGER value (triggers resize grow)
    let actions = vec![MintActionType::UpdateMetadataField {
        extension_index: 0,
        field_type: 0, // Name field
        key: vec![],
        value: "Much Longer Token Name That Will Cause Account Resize"
            .as_bytes()
            .to_vec(),
    }];

    light_token_client::actions::mint_action(
        &mut rpc,
        MintActionParams {
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
    .await
    .unwrap();

    // 5. Verify with assert_mint_action
    assert_mint_action(&mut rpc, compressed_mint_address, pre_mint, actions).await;
}

/// Test UpdateMetadataField with shorter value triggers resize shrink on existing CMint.
#[tokio::test]
#[serial]
async fn test_cmint_update_metadata_shrink() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let mint_seed = Keypair::new();
    let authority = Keypair::new();
    rpc.airdrop_lamports(&authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree_pubkey);
    let (spl_mint_pda, _) = find_mint_address(&mint_seed.pubkey());

    // 1. Create compressed mint with large metadata
    create_mint(
        &mut rpc,
        &mint_seed,
        8,
        &authority,
        Some(authority.pubkey()),
        Some(TokenMetadataInstructionData {
            update_authority: Some(authority.pubkey().into()),
            name: "This Is A Very Long Token Name That Will Be Shortened"
                .as_bytes()
                .to_vec(),
            symbol: "LONGSYMBOL".as_bytes().to_vec(),
            uri: "https://example.com/very/long/path/to/token/metadata.json"
                .as_bytes()
                .to_vec(),
            additional_metadata: None,
        }),
        &payer,
    )
    .await
    .unwrap();

    // 2. Decompress to CMint
    light_token_client::actions::mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &authority,
        &payer,
        Some(DecompressMintParams::default()),
        false,
        vec![],
        vec![],
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // 3. Get pre-state
    let cmint_account_data = rpc
        .get_account(spl_mint_pda)
        .await
        .unwrap()
        .expect("CMint should exist");
    let pre_mint: Mint =
        BorshDeserialize::deserialize(&mut cmint_account_data.data.as_slice()).unwrap();

    // 4. UpdateMetadataField with SHORTER value (triggers resize shrink)
    let actions = vec![MintActionType::UpdateMetadataField {
        extension_index: 0,
        field_type: 0, // Name field
        key: vec![],
        value: "Short".as_bytes().to_vec(),
    }];

    light_token_client::actions::mint_action(
        &mut rpc,
        MintActionParams {
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
    .await
    .unwrap();

    // 5. Verify
    assert_mint_action(&mut rpc, compressed_mint_address, pre_mint, actions).await;
}

/// Test RemoveMetadataKey triggers resize shrink on existing CMint.
#[tokio::test]
#[serial]
async fn test_cmint_remove_metadata_key() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let mint_seed = Keypair::new();
    let authority = Keypair::new();
    rpc.airdrop_lamports(&authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree_pubkey);
    let (spl_mint_pda, _) = find_mint_address(&mint_seed.pubkey());

    // 1. Create compressed mint with additional metadata
    create_mint(
        &mut rpc,
        &mint_seed,
        8,
        &authority,
        Some(authority.pubkey()),
        Some(TokenMetadataInstructionData {
            update_authority: Some(authority.pubkey().into()),
            name: "Test Token".as_bytes().to_vec(),
            symbol: "TEST".as_bytes().to_vec(),
            uri: "https://example.com".as_bytes().to_vec(),
            additional_metadata: Some(vec![
                AdditionalMetadata {
                    key: vec![1, 2, 3, 4],
                    value: vec![10u8; 32],
                },
                AdditionalMetadata {
                    key: vec![5, 6, 7, 8],
                    value: vec![20u8; 32],
                },
            ]),
        }),
        &payer,
    )
    .await
    .unwrap();

    // 2. Decompress to CMint
    light_token_client::actions::mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &authority,
        &payer,
        Some(DecompressMintParams::default()),
        false,
        vec![],
        vec![],
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // 3. Get pre-state
    let cmint_account_data = rpc
        .get_account(spl_mint_pda)
        .await
        .unwrap()
        .expect("CMint should exist");
    let pre_mint: Mint =
        BorshDeserialize::deserialize(&mut cmint_account_data.data.as_slice()).unwrap();

    // 4. RemoveMetadataKey (triggers resize shrink)
    let actions = vec![MintActionType::RemoveMetadataKey {
        extension_index: 0,
        key: vec![1, 2, 3, 4],
        idempotent: 0,
    }];

    light_token_client::actions::mint_action(
        &mut rpc,
        MintActionParams {
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
    .await
    .unwrap();

    // 5. Verify
    assert_mint_action(&mut rpc, compressed_mint_address, pre_mint, actions).await;
}

/// Test multiple metadata changes on existing CMint.
#[tokio::test]
#[serial]
async fn test_cmint_multiple_metadata_changes() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let mint_seed = Keypair::new();
    let authority = Keypair::new();
    rpc.airdrop_lamports(&authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree_pubkey);
    let (spl_mint_pda, _) = find_mint_address(&mint_seed.pubkey());

    // 1. Create compressed mint with metadata
    create_mint(
        &mut rpc,
        &mint_seed,
        8,
        &authority,
        Some(authority.pubkey()),
        Some(TokenMetadataInstructionData {
            update_authority: Some(authority.pubkey().into()),
            name: "Original Name".as_bytes().to_vec(),
            symbol: "ORIG".as_bytes().to_vec(),
            uri: "https://original.com".as_bytes().to_vec(),
            additional_metadata: Some(vec![
                AdditionalMetadata {
                    key: vec![1, 2, 3],
                    value: vec![1u8; 16],
                },
                AdditionalMetadata {
                    key: vec![4, 5, 6],
                    value: vec![2u8; 16],
                },
            ]),
        }),
        &payer,
    )
    .await
    .unwrap();

    // 2. Decompress to CMint
    light_token_client::actions::mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &authority,
        &payer,
        Some(DecompressMintParams::default()),
        false,
        vec![],
        vec![],
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // 3. Get pre-state
    let cmint_account_data = rpc
        .get_account(spl_mint_pda)
        .await
        .unwrap()
        .expect("CMint should exist");
    let pre_mint: Mint =
        BorshDeserialize::deserialize(&mut cmint_account_data.data.as_slice()).unwrap();

    // 4. Multiple metadata changes (grow name, shrink symbol, remove key)
    let actions = vec![
        MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 0, // Name - grow
            key: vec![],
            value: "Much Longer Updated Token Name".as_bytes().to_vec(),
        },
        MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 1, // Symbol - shrink
            key: vec![],
            value: "UP".as_bytes().to_vec(),
        },
        MintActionType::RemoveMetadataKey {
            extension_index: 0,
            key: vec![1, 2, 3],
            idempotent: 0,
        },
    ];

    light_token_client::actions::mint_action(
        &mut rpc,
        MintActionParams {
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
    .await
    .unwrap();

    // 5. Verify
    assert_mint_action(&mut rpc, compressed_mint_address, pre_mint, actions).await;
}

/// Test ALL operations on existing CMint in a single transaction.
#[tokio::test]
#[serial]
async fn test_cmint_all_operations() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let mint_seed = Keypair::new();
    let authority = Keypair::new();
    rpc.airdrop_lamports(&authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    rpc.airdrop_lamports(&payer.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree_pubkey);
    let (spl_mint_pda, _) = find_mint_address(&mint_seed.pubkey());

    // 1. Create compressed mint with metadata and additional_metadata
    create_mint(
        &mut rpc,
        &mint_seed,
        8,
        &authority,
        Some(authority.pubkey()),
        Some(TokenMetadataInstructionData {
            update_authority: Some(authority.pubkey().into()),
            name: "Test Token".as_bytes().to_vec(),
            symbol: "TEST".as_bytes().to_vec(),
            uri: "https://example.com/token.json".as_bytes().to_vec(),
            additional_metadata: Some(vec![
                AdditionalMetadata {
                    key: vec![1, 2, 3, 4],
                    value: vec![10u8; 16],
                },
                AdditionalMetadata {
                    key: vec![5, 6, 7, 8],
                    value: vec![20u8; 16],
                },
            ]),
        }),
        &payer,
    )
    .await
    .unwrap();

    // 2. Decompress to CMint
    light_token_client::actions::mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &authority,
        &payer,
        Some(DecompressMintParams::default()),
        false,
        vec![],
        vec![],
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // 3. Create CToken ATA for MintToCToken
    let recipient = Keypair::new();
    let compressible_params = CompressibleParams {
        compressible_config: rpc
            .test_accounts
            .funding_pool_config
            .compressible_config_pda,
        rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
        pre_pay_num_epochs: 0,
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

    // 4. Get pre-state
    let cmint_account_data = rpc
        .get_account(spl_mint_pda)
        .await
        .unwrap()
        .expect("CMint should exist");
    let pre_mint: Mint =
        BorshDeserialize::deserialize(&mut cmint_account_data.data.as_slice()).unwrap();

    // New authorities
    let new_mint_authority = Keypair::new();
    let new_freeze_authority = Keypair::new();
    let new_metadata_authority = Keypair::new();

    // 5. ALL operations in one tx
    let actions = vec![
        // MintTo (compressed recipients)
        MintActionType::MintTo {
            recipients: vec![MintToRecipient {
                recipient: Keypair::new().pubkey(),
                amount: 1000,
            }],
            token_account_version: 2,
        },
        // MintToCToken (decompressed recipient)
        MintActionType::MintToCToken {
            account: derive_token_ata(&recipient.pubkey(), &spl_mint_pda).0,
            amount: 2000,
        },
        // UpdateMintAuthority
        MintActionType::UpdateMintAuthority {
            new_authority: Some(new_mint_authority.pubkey()),
        },
        // UpdateFreezeAuthority
        MintActionType::UpdateFreezeAuthority {
            new_authority: Some(new_freeze_authority.pubkey()),
        },
        // UpdateMetadataField - name (grow)
        MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 0,
            key: vec![],
            value: "Updated Token Name That Is Much Longer".as_bytes().to_vec(),
        },
        // UpdateMetadataField - symbol
        MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 1,
            key: vec![],
            value: "UPDATED".as_bytes().to_vec(),
        },
        // UpdateMetadataField - uri
        MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 2,
            key: vec![],
            value: "https://updated.example.com/token.json".as_bytes().to_vec(),
        },
        // UpdateMetadataField - custom key
        MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 3,
            key: vec![1, 2, 3, 4],
            value: "updated_custom_value".as_bytes().to_vec(),
        },
        // RemoveMetadataKey
        MintActionType::RemoveMetadataKey {
            extension_index: 0,
            key: vec![5, 6, 7, 8],
            idempotent: 0,
        },
        // UpdateMetadataAuthority (must be last metadata operation)
        MintActionType::UpdateMetadataAuthority {
            extension_index: 0,
            new_authority: new_metadata_authority.pubkey(),
        },
    ];

    light_token_client::actions::mint_action(
        &mut rpc,
        MintActionParams {
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
    .await
    .unwrap();

    // 6. Verify
    assert_mint_action(&mut rpc, compressed_mint_address, pre_mint, actions).await;
}

// ============================================================================
// SCENARIO B: CMint Decompressed in Transaction (DecompressMint + Other Actions)
// ============================================================================

/// Test DecompressMint + MintTo in same transaction.
#[tokio::test]
#[serial]
async fn test_decompress_with_mint_to() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let mint_seed = Keypair::new();
    let authority = Keypair::new();
    rpc.airdrop_lamports(&authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree_pubkey);
    // 1. Create compressed mint (no decompress yet)
    create_mint(
        &mut rpc,
        &mint_seed,
        8,
        &authority,
        Some(authority.pubkey()),
        None,
        &payer,
    )
    .await
    .unwrap();

    // 2. Get pre-state from compressed account
    let compressed_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    let pre_mint: Mint =
        BorshDeserialize::deserialize(&mut compressed_account.data.unwrap().data.as_slice())
            .unwrap();

    // 3. DecompressMint + MintTo in same tx
    let actions = vec![
        MintActionType::DecompressMint {
            rent_payment: 2,
            write_top_up: 0,
        },
        MintActionType::MintTo {
            recipients: vec![MintToRecipient {
                recipient: Keypair::new().pubkey(),
                amount: 5000,
            }],
            token_account_version: 2,
        },
    ];

    light_token_client::actions::mint_action(
        &mut rpc,
        MintActionParams {
            compressed_mint_address,
            mint_seed: mint_seed.pubkey(),
            authority: authority.pubkey(),
            payer: payer.pubkey(),
            actions: actions.clone(),
            new_mint: None,
        },
        &authority,
        &payer,
        Some(&mint_seed), // Required for DecompressMint
    )
    .await
    .unwrap();

    // 4. Verify
    assert_mint_action(&mut rpc, compressed_mint_address, pre_mint, actions).await;
}

/// Test DecompressMint + authority updates in same transaction.
#[tokio::test]
#[serial]
async fn test_decompress_with_authority_updates() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let mint_seed = Keypair::new();
    let authority = Keypair::new();
    rpc.airdrop_lamports(&authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree_pubkey);
    // 1. Create compressed mint
    create_mint(
        &mut rpc,
        &mint_seed,
        8,
        &authority,
        Some(authority.pubkey()),
        None,
        &payer,
    )
    .await
    .unwrap();

    // 2. Get pre-state
    let compressed_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    let pre_mint: Mint =
        BorshDeserialize::deserialize(&mut compressed_account.data.unwrap().data.as_slice())
            .unwrap();

    let new_mint_authority = Keypair::new();
    let new_freeze_authority = Keypair::new();

    // 3. DecompressMint + UpdateMintAuthority + UpdateFreezeAuthority
    let actions = vec![
        MintActionType::DecompressMint {
            rent_payment: 2,
            write_top_up: 0,
        },
        MintActionType::UpdateMintAuthority {
            new_authority: Some(new_mint_authority.pubkey()),
        },
        MintActionType::UpdateFreezeAuthority {
            new_authority: Some(new_freeze_authority.pubkey()),
        },
    ];

    light_token_client::actions::mint_action(
        &mut rpc,
        MintActionParams {
            compressed_mint_address,
            mint_seed: mint_seed.pubkey(),
            authority: authority.pubkey(),
            payer: payer.pubkey(),
            actions: actions.clone(),
            new_mint: None,
        },
        &authority,
        &payer,
        Some(&mint_seed), // Required for DecompressMint
    )
    .await
    .unwrap();

    // 4. Verify
    assert_mint_action(&mut rpc, compressed_mint_address, pre_mint, actions).await;
}

/// Test DecompressMint + UpdateMetadataField in same transaction.
#[tokio::test]
#[serial]
async fn test_decompress_with_metadata_update() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let mint_seed = Keypair::new();
    let authority = Keypair::new();
    rpc.airdrop_lamports(&authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree_pubkey);
    // 1. Create compressed mint with metadata
    create_mint(
        &mut rpc,
        &mint_seed,
        8,
        &authority,
        Some(authority.pubkey()),
        Some(TokenMetadataInstructionData {
            update_authority: Some(authority.pubkey().into()),
            name: "Original".as_bytes().to_vec(),
            symbol: "ORIG".as_bytes().to_vec(),
            uri: "https://original.com".as_bytes().to_vec(),
            additional_metadata: None,
        }),
        &payer,
    )
    .await
    .unwrap();

    // 2. Get pre-state
    let compressed_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    let pre_mint: Mint =
        BorshDeserialize::deserialize(&mut compressed_account.data.unwrap().data.as_slice())
            .unwrap();

    // 3. DecompressMint + UpdateMetadataField
    let actions = vec![
        MintActionType::DecompressMint {
            rent_payment: 2,
            write_top_up: 0,
        },
        MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 0, // Name
            key: vec![],
            value: "Updated Name During Decompress".as_bytes().to_vec(),
        },
    ];

    light_token_client::actions::mint_action(
        &mut rpc,
        MintActionParams {
            compressed_mint_address,
            mint_seed: mint_seed.pubkey(),
            authority: authority.pubkey(),
            payer: payer.pubkey(),
            actions: actions.clone(),
            new_mint: None,
        },
        &authority,
        &payer,
        Some(&mint_seed), // Required for DecompressMint
    )
    .await
    .unwrap();

    // 4. Verify
    assert_mint_action(&mut rpc, compressed_mint_address, pre_mint, actions).await;
}

/// Test DecompressMint + MintToCToken in same transaction.
#[tokio::test]
#[serial]
async fn test_decompress_with_mint_to_ctoken() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let mint_seed = Keypair::new();
    let authority = Keypair::new();
    rpc.airdrop_lamports(&authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    rpc.airdrop_lamports(&payer.pubkey(), 10_000_000_000)
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
        8,
        &authority,
        Some(authority.pubkey()),
        None,
        &payer,
    )
    .await
    .unwrap();

    // 2. Create CToken ATA for recipient
    let recipient = Keypair::new();
    let compressible_params = CompressibleParams {
        compressible_config: rpc
            .test_accounts
            .funding_pool_config
            .compressible_config_pda,
        rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
        pre_pay_num_epochs: 0,
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

    // 3. Get pre-state
    let compressed_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    let pre_mint: Mint =
        BorshDeserialize::deserialize(&mut compressed_account.data.unwrap().data.as_slice())
            .unwrap();

    // 4. DecompressMint + MintToCToken
    let actions = vec![
        MintActionType::DecompressMint {
            rent_payment: 2,
            write_top_up: 0,
        },
        MintActionType::MintToCToken {
            account: derive_token_ata(&recipient.pubkey(), &spl_mint_pda).0,
            amount: 5000,
        },
    ];

    light_token_client::actions::mint_action(
        &mut rpc,
        MintActionParams {
            compressed_mint_address,
            mint_seed: mint_seed.pubkey(),
            authority: authority.pubkey(),
            payer: payer.pubkey(),
            actions: actions.clone(),
            new_mint: None,
        },
        &authority,
        &payer,
        Some(&mint_seed), // Required for DecompressMint
    )
    .await
    .unwrap();

    // 5. Verify
    assert_mint_action(&mut rpc, compressed_mint_address, pre_mint, actions).await;
}

/// Test DecompressMint + ALL other operations in same transaction.
#[tokio::test]
#[serial]
async fn test_decompress_with_all_operations() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let mint_seed = Keypair::new();
    let authority = Keypair::new();
    rpc.airdrop_lamports(&authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    rpc.airdrop_lamports(&payer.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree_pubkey);
    let (spl_mint_pda, _) = find_mint_address(&mint_seed.pubkey());

    // 1. Create compressed mint with metadata and additional_metadata
    create_mint(
        &mut rpc,
        &mint_seed,
        8,
        &authority,
        Some(authority.pubkey()),
        Some(TokenMetadataInstructionData {
            update_authority: Some(authority.pubkey().into()),
            name: "Test Token".as_bytes().to_vec(),
            symbol: "TEST".as_bytes().to_vec(),
            uri: "https://example.com/token.json".as_bytes().to_vec(),
            additional_metadata: Some(vec![
                AdditionalMetadata {
                    key: vec![1, 2, 3, 4],
                    value: vec![10u8; 16],
                },
                AdditionalMetadata {
                    key: vec![5, 6, 7, 8],
                    value: vec![20u8; 16],
                },
            ]),
        }),
        &payer,
    )
    .await
    .unwrap();

    // 2. Create CToken ATA for MintToCToken
    let recipient = Keypair::new();
    let compressible_params = CompressibleParams {
        compressible_config: rpc
            .test_accounts
            .funding_pool_config
            .compressible_config_pda,
        rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
        pre_pay_num_epochs: 0,
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

    // 3. Get pre-state from compressed account
    let compressed_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    let pre_mint: Mint =
        BorshDeserialize::deserialize(&mut compressed_account.data.unwrap().data.as_slice())
            .unwrap();

    // New authorities
    let new_mint_authority = Keypair::new();
    let new_freeze_authority = Keypair::new();
    let new_metadata_authority = Keypair::new();

    // 4. DecompressMint + ALL other operations
    let actions = vec![
        // DecompressMint
        MintActionType::DecompressMint {
            rent_payment: 2,
            write_top_up: 0,
        },
        // MintTo (compressed recipients)
        MintActionType::MintTo {
            recipients: vec![MintToRecipient {
                recipient: Keypair::new().pubkey(),
                amount: 1000,
            }],
            token_account_version: 2,
        },
        // MintToCToken (decompressed recipient)
        MintActionType::MintToCToken {
            account: derive_token_ata(&recipient.pubkey(), &spl_mint_pda).0,
            amount: 2000,
        },
        // UpdateMintAuthority
        MintActionType::UpdateMintAuthority {
            new_authority: Some(new_mint_authority.pubkey()),
        },
        // UpdateFreezeAuthority
        MintActionType::UpdateFreezeAuthority {
            new_authority: Some(new_freeze_authority.pubkey()),
        },
        // UpdateMetadataField - name
        MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 0,
            key: vec![],
            value: "Updated Name".as_bytes().to_vec(),
        },
        // UpdateMetadataField - symbol
        MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 1,
            key: vec![],
            value: "UPDT".as_bytes().to_vec(),
        },
        // UpdateMetadataField - uri
        MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 2,
            key: vec![],
            value: "https://updated.com".as_bytes().to_vec(),
        },
        // RemoveMetadataKey
        MintActionType::RemoveMetadataKey {
            extension_index: 0,
            key: vec![5, 6, 7, 8],
            idempotent: 0,
        },
        // UpdateMetadataAuthority (must be last metadata operation)
        MintActionType::UpdateMetadataAuthority {
            extension_index: 0,
            new_authority: new_metadata_authority.pubkey(),
        },
    ];

    light_token_client::actions::mint_action(
        &mut rpc,
        MintActionParams {
            compressed_mint_address,
            mint_seed: mint_seed.pubkey(),
            authority: authority.pubkey(),
            payer: payer.pubkey(),
            actions: actions.clone(),
            new_mint: None,
        },
        &authority,
        &payer,
        Some(&mint_seed), // Required for DecompressMint
    )
    .await
    .unwrap();

    // 5. Verify
    assert_mint_action(&mut rpc, compressed_mint_address, pre_mint, actions).await;
}

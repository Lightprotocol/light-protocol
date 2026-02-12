use anchor_lang::prelude::borsh::BorshDeserialize;
use light_client::indexer::Indexer;
use light_compressed_token_sdk::compressed_token::create_compressed_mint::{
    derive_mint_compressed_address, find_mint_address,
};
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    actions::{
        create_mint,
        legacy::instructions::mint_action::{MintActionType, MintToRecipient},
    },
    assert_mint_action::assert_mint_action,
    mint_assert::assert_compressed_mint_account,
    Rpc,
};
use light_token::instruction::{CompressibleParams, CreateAssociatedTokenAccount};
use light_token_interface::state::{extensions::AdditionalMetadata, Mint, TokenDataVersion};
use serial_test::serial;
use solana_sdk::{signature::Keypair, signer::Signer};

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
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree_pubkey);

    // Find mint PDA for the rest of the test
    let (spl_mint_pda, _) = find_mint_address(&mint_seed.pubkey());
    // 1. Create compressed mint with both authorities
    {
        create_mint(
        &mut rpc,
        &mint_seed,
        8, // decimals
        &authority,
        Some(authority.pubkey()),
        Some(light_token_interface::instructions::extensions::token_metadata::TokenMetadataInstructionData {
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
            .value
            .unwrap();
        assert_compressed_mint_account(
        &compressed_mint_account,
        compressed_mint_address,
        spl_mint_pda,
        8,
        authority.pubkey(),
        authority.pubkey(),
        Some(light_token_interface::instructions::extensions::token_metadata::TokenMetadataInstructionData {
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

    // Create a compressible ctoken account for MintToCToken
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
        compression_only: true, // ATAs require compression_only=true
    };

    let create_compressible_ata_ix =
        CreateAssociatedTokenAccount::new(payer.pubkey(), recipient.pubkey(), spl_mint_pda)
            .with_compressible(compressible_params)
            .instruction()
            .unwrap();

    rpc.create_and_send_transaction(&[create_compressible_ata_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Build all actions for a single instruction
    let actions = vec![
        // 1. MintToCompressed - mint to compressed account
        MintActionType::MintTo {
            recipients: vec![MintToRecipient {
                recipient: Keypair::new().pubkey(),
                amount: 1000u64,
            }],
            token_account_version: 2,
        },
        // 2. MintToCToken - mint to decompressed account
        MintActionType::MintToCToken {
            account: light_token::instruction::get_associated_token_address(&recipient.pubkey(), &spl_mint_pda),
            amount: 2000u64,
        },
        // 3. UpdateMintAuthority
        MintActionType::UpdateMintAuthority {
            new_authority: Some(new_mint_authority.pubkey()),
        },
        // 4. UpdateFreezeAuthority
        MintActionType::UpdateFreezeAuthority {
            new_authority: Some(new_freeze_authority.pubkey()),
        },
        // 5. UpdateMetadataField - update the name
        MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 0, // Name field
            key: vec![],
            value: "Updated Token Name".as_bytes().to_vec(),
        },
        // 6. UpdateMetadataField - update the symbol
        MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 1, // Symbol field
            key: vec![],
            value: "UPDATED".as_bytes().to_vec(),
        },
        // 7. UpdateMetadataField - update the URI
        MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 2, // URI field
            key: vec![],
            value: "https://updated.example.com/token.json".as_bytes().to_vec(),
        },
        // 8. UpdateMetadataField - update the first additional metadata field
        MintActionType::UpdateMetadataField {
            extension_index: 0,
            field_type: 3, // Custom key field
            key: vec![1, 2, 3, 4],
            value: "updated_value".as_bytes().to_vec(),
        },
        // 9. RemoveMetadataKey - remove the second additional metadata key
        MintActionType::RemoveMetadataKey {
            extension_index: 0,
            key: vec![4, 5, 6, 7],
            idempotent: 0,
        },
        // 10. UpdateMetadataAuthority
        MintActionType::UpdateMetadataAuthority {
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
        .value
        .unwrap();

    let pre_compressed_mint: Mint = BorshDeserialize::deserialize(
        &mut pre_compressed_mint_account.data.unwrap().data.as_slice(),
    )
    .unwrap();

    // Execute all actions in a single instruction
    let result = light_test_utils::actions::mint_action(
        &mut rpc,
        light_test_utils::actions::legacy::instructions::mint_action::MintActionParams {
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

    // Use the new assert_mint_action function (now also validates Light Token account state)
    assert_mint_action(
        &mut rpc,
        compressed_mint_address,
        pre_compressed_mint,
        actions,
    )
    .await;
}

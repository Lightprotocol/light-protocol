use anchor_lang::prelude::borsh::BorshDeserialize;
use light_batched_merkle_tree::initialize_state_tree::InitStateTreeAccountsInstructionData;
use light_client::indexer::Indexer;
use light_compressed_token_sdk::instructions::{
    derive_compressed_mint_address, find_spl_mint_address,
};
use light_ctoken_types::state::{extensions::AdditionalMetadata, CompressedMint};
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    assert_mint_action::assert_mint_action, mint_assert::assert_compressed_mint_account, Rpc,
};
use light_token_client::actions::create_mint;
use serial_test::serial;
use solana_sdk::{signature::Keypair, signer::Signer};

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
        .value
        .unwrap();
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

        // Skip if no actions were generated
        if actions.is_empty() {
            continue;
        }

        // Shuffle the actions to randomize order
        use rand::seq::SliceRandom;
        actions.shuffle(&mut rng);

        // Fix action ordering: remove any UpdateMetadataField actions that come after RemoveMetadataKey for the same key
        use std::collections::HashSet;

        use light_compressed_token_sdk::instructions::mint_action::MintActionType;

        let mut removed_keys: HashSet<Vec<u8>> = HashSet::new();
        let mut i = 0;

        while i < actions.len() {
            match &actions[i] {
                MintActionType::RemoveMetadataKey { key, .. } => {
                    // Track that this key has been removed
                    removed_keys.insert(key.clone());
                    i += 1;
                }
                MintActionType::UpdateMetadataField {
                    key, field_type: 3, ..
                } => {
                    // If trying to update a key that was already removed, remove this action
                    if removed_keys.contains(key) {
                        actions.remove(i);
                        // Don't increment i, check the same position again
                    } else {
                        i += 1;
                    }
                }
                _ => {
                    i += 1;
                }
            }
        }

        // Get pre-state compressed mint
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

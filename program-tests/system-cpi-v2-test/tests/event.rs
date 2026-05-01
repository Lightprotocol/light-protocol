#![cfg(feature = "test-sbf")]

use std::{collections::HashMap, fs};

use anchor_lang::{
    prelude::borsh::{BorshDeserialize, BorshSerialize},
    Discriminator,
};
use create_address_test_program::create_invoke_cpi_instruction;
use light_client::{
    indexer::{AddressWithTree, Indexer},
    local_test_validator::{spawn_validator, LightValidatorConfig},
    rpc::LightClientConfig,
};
use light_compressed_account::{
    address::{derive_address, derive_address_legacy},
    compressed_account::{
        CompressedAccount, CompressedAccountData, CompressedAccountWithMerkleContext,
        MerkleContext, PackedCompressedAccountWithMerkleContext,
    },
    instruction_data::{
        compressed_proof::CompressedProof,
        data::{
            NewAddressParams, OutputCompressedAccountWithContext,
            OutputCompressedAccountWithPackedContext,
        },
        with_readonly::{InAccount, InstructionDataInvokeCpiWithReadOnly},
    },
    nullifier::create_nullifier,
    tx_hash::create_tx_hash,
    TreeType,
};
use light_compressed_token::process_transfer::transfer_sdk::to_account_metas;
use light_event::{
    event::{
        BatchNullifyContext, BatchPublicTransactionEvent, MerkleTreeSequenceNumber,
        MerkleTreeSequenceNumberV1, NewAddress, PublicTransactionEvent,
    },
    parse::event_from_light_transaction,
};
use light_merkle_tree_metadata::events::{
    batch::BatchEvent as MetadataBatchEvent, MerkleTreeEvent as MetadataMerkleTreeEvent,
};
use light_program_test::{
    accounts::test_accounts::TestAccounts, LightProgramTest, ProgramTestConfig,
};
use light_sdk::address::NewAddressParamsAssigned;
use light_test_utils::{
    create_address_test_program_sdk::{
        local_dev_proofless_shielded_append_plaintext, local_dev_proofless_shielded_spend_args,
        proofless_shielded_append_args_from_plaintext, proofless_shielded_append_instruction,
        proofless_shielded_spend_instruction, shielded_spend_nullifier_address,
        ProoflessShieldedAppendInstructionInputs, ProoflessShieldedSpendInstructionInputs,
        LOCAL_DEV_DATA_HASH, LOCAL_DEV_ENCRYPTED_UTXO_HASH, LOCAL_DEV_OPERATION_COMMITMENT,
        LOCAL_DEV_UTXO_HASH, LOCAL_DEV_UTXO_LEAF_INDEX, LOCAL_DEV_ZONE_CONFIG_HASH,
    },
    pack::{
        pack_compressed_accounts, pack_new_address_params_assigned, pack_output_compressed_accounts,
    },
    LightClient, Rpc, RpcError,
};
use serde::Serialize;
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProoflessAppendCaptureSnapshot {
    schema_version: u32,
    source: String,
    transaction: ProoflessAppendCapturedTransaction,
    expected: ProoflessAppendCaptureExpected,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProoflessAppendCapturedTransaction {
    shielded_program_instruction: ProoflessAppendCapturedInstruction,
    light_system_instruction: ProoflessAppendCapturedInstruction,
    system_instruction: ProoflessAppendCapturedInstruction,
    account_compression_instruction: ProoflessAppendCapturedInstruction,
    shielded_event_noop_instruction: ProoflessAppendCapturedInstruction,
    batch_append_instruction: ProoflessAppendCapturedInstruction,
    batch_append_noop_instruction: ProoflessAppendCapturedInstruction,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProoflessAppendCapturedInstruction {
    name: String,
    program_id: String,
    data: String,
    accounts: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProoflessAppendCaptureExpected {
    zone_config_hash: String,
    operation_commitment: String,
    data_hash: String,
    utxo_hash: String,
    encrypted_utxo_hash: String,
    compressed_account_hash: String,
    utxo_tree: String,
    output_queue: String,
    output_leaf_index: u32,
    tree_sequence: u64,
    batch_append_sequence: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProoflessSpendCaptureSnapshot {
    schema_version: u32,
    source: String,
    transaction: ProoflessSpendCapturedTransaction,
    expected: ProoflessSpendCaptureExpected,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProoflessSpendCapturedTransaction {
    shielded_program_instruction: ProoflessAppendCapturedInstruction,
    light_system_instruction: ProoflessAppendCapturedInstruction,
    system_instruction: ProoflessAppendCapturedInstruction,
    account_compression_instruction: ProoflessAppendCapturedInstruction,
    shielded_event_noop_instruction: ProoflessAppendCapturedInstruction,
    nullifier_event_noop_instructions: Vec<ProoflessAppendCapturedInstruction>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProoflessSpendCaptureExpected {
    zone_config_hash: String,
    operation_commitment: String,
    utxo_hash: String,
    utxo_leaf_index: u64,
    spend_nullifier: String,
    nullifier_chain: String,
    nullifier_tree: String,
}

// TODO: add test with multiple batched address trees before we activate batched addresses
#[tokio::test]
#[serial]
async fn parse_batched_event_functional() {
    let mut rpc = LightProgramTest::new({
        let mut config = ProgramTestConfig::default_with_batched_trees(false);
        config.additional_programs = Some(vec![(
            "create_address_test_program",
            create_address_test_program::ID,
        )]);
        config
    })
    .await
    .expect("Failed to setup test programs with accounts");
    let env = rpc.test_accounts.clone();
    let payer = rpc.get_payer().insecure_clone();
    // Insert 8 output accounts that we can use as inputs.
    {
        let num_expected_events = 1;
        let output_accounts =
            vec![get_compressed_output_account(true, env.v2_state_trees[0].output_queue,); 8];
        let (events, output_accounts, _) = perform_test_transaction(
            &mut rpc,
            &payer,
            vec![],
            output_accounts,
            vec![],
            None,
            None,
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(events.len(), num_expected_events as usize);
        let expected_batched_event = BatchPublicTransactionEvent {
            event: PublicTransactionEvent {
                input_compressed_account_hashes: Vec::new(),
                output_leaf_indices: (0..8).collect(),
                output_compressed_account_hashes: output_accounts
                    .iter()
                    .enumerate()
                    .map(|(i, x)| {
                        x.compressed_account
                            .hash(&env.v2_state_trees[0].merkle_tree.into(), &(i as u32), true)
                            .unwrap()
                    })
                    .collect::<Vec<_>>(),
                output_compressed_accounts: output_accounts.to_vec(),
                sequence_numbers: vec![MerkleTreeSequenceNumberV1 {
                    tree_pubkey: env.v2_state_trees[0].merkle_tree.into(),
                    // queue_pubkey: env.v2_state_trees[0].output_queue,
                    // tree_type: TreeType::StateV2 as u64,
                    seq: 0,
                }],
                relay_fee: None,
                message: None,
                is_compress: false,
                compress_or_decompress_lamports: None,
                pubkey_array: vec![env.v2_state_trees[0].output_queue.into()],
                ata_owners: vec![],
            },
            address_sequence_numbers: Vec::new(),
            input_sequence_numbers: Vec::new(),
            batch_input_accounts: Vec::new(),
            new_addresses: Vec::new(),
            tx_hash: [0u8; 32],
        };
        assert_eq!(events[0], expected_batched_event);
    }

    // Full functional 8 input, 8 outputs, 2 legacy addresses
    {
        let num_expected_events = 1;
        let output_accounts =
            vec![get_compressed_output_account(true, env.v2_state_trees[0].output_queue,); 8];
        let input_accounts = (0..8)
            .map(|i| {
                get_compressed_input_account(MerkleContext {
                    leaf_index: i,
                    merkle_tree_pubkey: env.v2_state_trees[0].merkle_tree.into(),
                    prove_by_index: true,
                    queue_pubkey: env.v2_state_trees[0].output_queue.into(),
                    tree_type: light_compressed_account::TreeType::StateV2,
                })
            })
            .collect::<Vec<_>>();

        let new_addresses = [
            derive_address_legacy(&env.v1_address_trees[0].merkle_tree.into(), &[1u8; 32]).unwrap(),
            derive_address_legacy(&env.v1_address_trees[0].merkle_tree.into(), &[2u8; 32]).unwrap(),
        ];
        let payer = rpc.get_payer().insecure_clone();

        let addresses_with_tree = new_addresses
            .iter()
            .map(|new_address| AddressWithTree {
                address: *new_address,
                tree: env.v1_address_trees[0].merkle_tree,
            })
            .collect::<Vec<_>>();

        let proof_res = rpc
            .get_validity_proof(Vec::new(), addresses_with_tree, None)
            .await;

        let proof_result = proof_res.unwrap().value;

        let new_address_params = vec![
            NewAddressParamsAssigned {
                seed: [1u8; 32],
                address_queue_pubkey: env.v1_address_trees[0].queue.into(),
                address_merkle_tree_pubkey: env.v1_address_trees[0].merkle_tree.into(),
                address_merkle_tree_root_index: proof_result.get_address_root_indices()[0],
                assigned_account_index: None,
            },
            NewAddressParamsAssigned {
                seed: [2u8; 32],
                address_queue_pubkey: env.v1_address_trees[0].queue.into(),
                address_merkle_tree_pubkey: env.v1_address_trees[0].merkle_tree.into(),
                address_merkle_tree_root_index: proof_result.get_address_root_indices()[1],
                assigned_account_index: None,
            },
        ];
        let (events, output_accounts, _) = perform_test_transaction(
            &mut rpc,
            &payer,
            input_accounts.to_vec(),
            output_accounts,
            new_address_params,
            None,
            proof_result.proof.0,
        )
        .await
        .unwrap()
        .unwrap();
        let slot = rpc.get_slot().await.unwrap();
        assert_eq!(events.len(), num_expected_events as usize);
        let input_hashes = input_accounts
            .iter()
            .map(|x| x.hash().unwrap())
            .collect::<Vec<_>>();
        let output_hashes = output_accounts
            .iter()
            .enumerate()
            .map(|(i, x)| {
                x.compressed_account
                    .hash(
                        &env.v2_state_trees[0].merkle_tree.into(),
                        &((i + 8) as u32),
                        true,
                    )
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let tx_hash = create_tx_hash(&input_hashes, &output_hashes, slot).unwrap();
        let batch_input_accounts = input_hashes
            .iter()
            .zip(input_accounts.iter())
            .enumerate()
            .map(|(i, (hash, x))| BatchNullifyContext {
                account_hash: *hash,
                tx_hash,
                nullifier: create_nullifier(hash, x.merkle_context.leaf_index as u64, &tx_hash)
                    .unwrap(),
                nullifier_queue_index: i as u64,
            })
            .collect::<Vec<_>>();

        let expected_batched_event = BatchPublicTransactionEvent {
            event: PublicTransactionEvent {
                input_compressed_account_hashes: input_hashes,
                output_leaf_indices: (8..16).collect(),
                output_compressed_account_hashes: output_accounts
                    .iter()
                    .enumerate()
                    .map(|(i, x)| {
                        x.compressed_account
                            .hash(
                                &env.v2_state_trees[0].merkle_tree.into(),
                                &((i + 8) as u32),
                                true,
                            )
                            .unwrap()
                    })
                    .collect::<Vec<_>>(),
                output_compressed_accounts: output_accounts.to_vec(),
                ata_owners: vec![],
                sequence_numbers: vec![MerkleTreeSequenceNumberV1 {
                    tree_pubkey: env.v2_state_trees[0].merkle_tree.into(),
                    // queue_pubkey: env.v2_state_trees[0].output_queue,
                    // tree_type: TreeType::StateV2 as u64,
                    seq: 8,
                }],
                relay_fee: None,
                message: None,
                is_compress: false,
                compress_or_decompress_lamports: None,
                pubkey_array: vec![
                    env.v1_address_trees[0].merkle_tree.into(),
                    env.v1_address_trees[0].queue.into(),
                    env.v2_state_trees[0].merkle_tree.into(),
                    env.v2_state_trees[0].output_queue.into(),
                ],
            },
            address_sequence_numbers: Vec::new(),
            input_sequence_numbers: vec![MerkleTreeSequenceNumber {
                tree_pubkey: env.v2_state_trees[0].merkle_tree.into(),
                queue_pubkey: env.v2_state_trees[0].output_queue.into(),
                tree_type: TreeType::StateV2 as u64,
                seq: 0,
            }],
            batch_input_accounts,
            new_addresses: new_addresses
                .iter()
                .map(|x| NewAddress {
                    address: *x,
                    mt_pubkey: env.v1_address_trees[0].merkle_tree.into(),
                    queue_index: u64::MAX,
                })
                .collect(),
            tx_hash,
        };
        assert_eq!(events[0], expected_batched_event);
    }
    // Full functional 8 input, 8 outputs, 2 batched addresses
    {
        let num_expected_events = 1;
        let output_accounts =
            vec![get_compressed_output_account(true, env.v2_state_trees[0].output_queue,); 8];
        let input_accounts = (8..16)
            .map(|i| {
                get_compressed_input_account(MerkleContext {
                    leaf_index: i,
                    merkle_tree_pubkey: env.v2_state_trees[0].merkle_tree.into(),
                    prove_by_index: true,
                    queue_pubkey: env.v2_state_trees[0].output_queue.into(),
                    tree_type: light_compressed_account::TreeType::StateV2,
                })
            })
            .collect::<Vec<_>>();

        let new_addresses = [
            derive_address(
                &[1u8; 32],
                &env.v2_address_trees[0].to_bytes(),
                &create_address_test_program::ID.to_bytes(),
            ),
            derive_address(
                &[2u8; 32],
                &env.v2_address_trees[0].to_bytes(),
                &create_address_test_program::ID.to_bytes(),
            ),
        ];
        let payer = rpc.get_payer().insecure_clone();

        let addresses_with_tree = new_addresses
            .iter()
            .map(|address| AddressWithTree {
                address: *address,
                tree: env.v2_address_trees[0],
            })
            .collect::<Vec<_>>();

        let proof_res = rpc
            .get_validity_proof(Vec::new(), addresses_with_tree, None)
            .await;

        let proof_result = proof_res.unwrap().value;

        let new_address_params = vec![
            NewAddressParamsAssigned {
                seed: [1u8; 32],
                address_queue_pubkey: env.v2_address_trees[0].into(),
                address_merkle_tree_pubkey: env.v2_address_trees[0].into(),
                address_merkle_tree_root_index: proof_result.get_address_root_indices()[0],
                assigned_account_index: None,
            },
            NewAddressParamsAssigned {
                seed: [2u8; 32],
                address_queue_pubkey: env.v2_address_trees[0].into(),
                address_merkle_tree_pubkey: env.v2_address_trees[0].into(),
                address_merkle_tree_root_index: proof_result.get_address_root_indices()[1],
                assigned_account_index: None,
            },
        ];
        let (events, output_accounts, _) = perform_test_transaction(
            &mut rpc,
            &payer,
            input_accounts.to_vec(),
            output_accounts,
            new_address_params,
            None,
            proof_result.proof.0,
        )
        .await
        .unwrap()
        .unwrap();
        let slot = rpc.get_slot().await.unwrap();
        assert_eq!(events.len(), num_expected_events as usize);
        let input_hashes = input_accounts
            .iter()
            .map(|x| {
                x.compressed_account
                    .hash(
                        &env.v2_state_trees[0].merkle_tree.into(),
                        &x.merkle_context.leaf_index,
                        true,
                    )
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let output_hashes = output_accounts
            .iter()
            .enumerate()
            .map(|(i, x)| {
                x.compressed_account
                    .hash(
                        &env.v2_state_trees[0].merkle_tree.into(),
                        &((i + 16) as u32),
                        true,
                    )
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let tx_hash = create_tx_hash(&input_hashes, &output_hashes, slot).unwrap();
        let batch_input_accounts = input_hashes
            .iter()
            .zip(input_accounts.iter())
            .enumerate()
            .map(|(i, (hash, x))| BatchNullifyContext {
                account_hash: *hash,
                tx_hash,
                nullifier: create_nullifier(hash, x.merkle_context.leaf_index as u64, &tx_hash)
                    .unwrap(),
                nullifier_queue_index: 8 + i as u64,
            })
            .collect::<Vec<_>>();

        let expected_batched_event = BatchPublicTransactionEvent {
            event: PublicTransactionEvent {
                input_compressed_account_hashes: input_hashes,
                output_leaf_indices: (16..24).collect(),
                output_compressed_account_hashes: output_accounts
                    .iter()
                    .enumerate()
                    .map(|(i, x)| {
                        x.compressed_account
                            .hash(
                                &env.v2_state_trees[0].merkle_tree.into(),
                                &((i + 16) as u32),
                                true,
                            )
                            .unwrap()
                    })
                    .collect::<Vec<_>>(),
                output_compressed_accounts: output_accounts.to_vec(),
                sequence_numbers: vec![MerkleTreeSequenceNumberV1 {
                    tree_pubkey: env.v2_state_trees[0].merkle_tree.into(),
                    // queue_pubkey: env.v2_state_trees[0].output_queue,
                    // tree_type: TreeType::StateV2 as u64,
                    seq: 16,
                }],
                relay_fee: None,
                message: None,
                is_compress: false,
                compress_or_decompress_lamports: None,
                pubkey_array: vec![
                    env.v2_address_trees[0].into(),
                    env.v2_state_trees[0].merkle_tree.into(),
                    env.v2_state_trees[0].output_queue.into(),
                ],
                ata_owners: vec![],
            },
            address_sequence_numbers: vec![MerkleTreeSequenceNumber {
                tree_pubkey: env.v2_address_trees[0].into(),
                queue_pubkey: Pubkey::default().into(),
                tree_type: TreeType::AddressV2 as u64,
                seq: 0,
            }],
            input_sequence_numbers: vec![MerkleTreeSequenceNumber {
                tree_pubkey: env.v2_state_trees[0].merkle_tree.into(),
                queue_pubkey: env.v2_state_trees[0].output_queue.into(),
                tree_type: TreeType::StateV2 as u64,
                seq: 8,
            }],
            batch_input_accounts,
            new_addresses: new_addresses
                .iter()
                .enumerate()
                .map(|(i, x)| NewAddress {
                    address: *x,
                    mt_pubkey: env.v2_address_trees[0].into(),
                    queue_index: i as u64,
                })
                .collect(),
            tx_hash,
        };
        assert_eq!(events[0], expected_batched_event);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
#[serial]
async fn parse_multiple_batched_events_functional() {
    for num_expected_events in 1..5 {
        let mut config = ProgramTestConfig::default_with_batched_trees(false);
        config.with_prover = false;
        config.additional_programs = Some(vec![(
            "create_address_test_program",
            create_address_test_program::ID,
        )]);

        let mut rpc = LightProgramTest::new(config)
            .await
            .expect("Failed to setup test programs with accounts");
        let env = rpc.test_accounts.clone();
        let payer = rpc.get_payer().insecure_clone();
        rpc.airdrop_lamports(&payer.pubkey(), 10_000_000_000)
            .await
            .unwrap();
        let output_accounts = vec![get_compressed_output_account(
            true,
            env.v2_state_trees[0].output_queue,
        )];
        let (events, output_accounts, _) = perform_test_transaction(
            &mut rpc,
            &payer,
            vec![],
            output_accounts,
            vec![],
            Some(num_expected_events),
            None,
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(events.len(), num_expected_events as usize);
        let expected_batched_event = BatchPublicTransactionEvent {
            event: PublicTransactionEvent {
                input_compressed_account_hashes: Vec::new(),
                output_leaf_indices: vec![0],
                output_compressed_account_hashes: vec![output_accounts[0]
                    .compressed_account
                    .hash(&env.v2_state_trees[0].merkle_tree.into(), &0u32, true)
                    .unwrap()],
                output_compressed_accounts: output_accounts.to_vec(),
                sequence_numbers: vec![MerkleTreeSequenceNumberV1 {
                    tree_pubkey: env.v2_state_trees[0].merkle_tree.into(),
                    // queue_pubkey: env.v2_state_trees[0].output_queue,
                    // tree_type: TreeType::StateV2 as u64,
                    seq: 0,
                }],
                relay_fee: None,
                message: None,
                is_compress: false,
                compress_or_decompress_lamports: None,
                pubkey_array: vec![env.v2_state_trees[0].output_queue.into()],
                ata_owners: vec![],
            },
            address_sequence_numbers: Vec::new(),
            input_sequence_numbers: Vec::new(),
            batch_input_accounts: Vec::new(),
            new_addresses: Vec::new(),
            tx_hash: [0u8; 32],
        };
        assert_eq!(events[0], expected_batched_event);
        for i in 1..num_expected_events {
            let mut expected_event = expected_batched_event.clone();
            expected_event.event.sequence_numbers = vec![MerkleTreeSequenceNumberV1 {
                tree_pubkey: env.v2_state_trees[0].merkle_tree.into(),
                // queue_pubkey: env.v2_state_trees[0].output_queue,
                // tree_type: TreeType::StateV2 as u64,
                seq: i as u64,
            }];
            expected_event.event.output_compressed_account_hashes = vec![output_accounts[0]
                .clone()
                .compressed_account
                .hash(&env.v2_state_trees[0].merkle_tree.into(), &(i as u32), true)
                .unwrap()];
            expected_event.event.output_leaf_indices = vec![i as u32];
            assert_eq!(events[i as usize], expected_event);
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
#[serial]
async fn proofless_shielded_append_emits_light_and_shielded_events() {
    let mut config = ProgramTestConfig::default_with_batched_trees(false);
    config.with_prover = true;
    config.additional_programs = Some(vec![(
        "create_address_test_program",
        create_address_test_program::ID,
    )]);

    let mut rpc = LightProgramTest::new(config)
        .await
        .expect("Failed to setup test programs with accounts");
    let env = rpc.test_accounts.clone();
    let payer = rpc.get_payer().insecure_clone();
    rpc.airdrop_lamports(&payer.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let encrypted_utxo = vec![0xC1, 0xC2, 0xC3, 0xC4];
    let (append_args, commitments) = proofless_shielded_append_args_from_plaintext(
        local_dev_proofless_shielded_append_plaintext(encrypted_utxo.clone()),
    );
    assert_eq!(commitments.data_hash, LOCAL_DEV_DATA_HASH);
    assert_eq!(commitments.utxo_hash, LOCAL_DEV_UTXO_HASH);
    assert_eq!(
        commitments.encrypted_utxo_hash,
        LOCAL_DEV_ENCRYPTED_UTXO_HASH
    );
    let instruction =
        proofless_shielded_append_instruction(ProoflessShieldedAppendInstructionInputs {
            signer: &payer.pubkey(),
            output_compressed_account_merkle_tree_pubkey: &env.v2_state_trees[0].output_queue,
            registered_program_pda: &env.protocol.registered_program_pda,
            args: append_args,
        });
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        rpc.context.latest_blockhash(),
    );

    let simulation = rpc
        .context
        .simulate_transaction(transaction.clone())
        .expect("proofless shielded append simulation should succeed");

    let shielded_event = simulation
        .meta
        .inner_instructions
        .iter()
        .flatten()
        .find_map(|inner_instruction| {
            let data = &inner_instruction.instruction.data;
            create_address_test_program::ShieldedPoolTxEvent::matches_discriminator(data).then(
                || {
                    create_address_test_program::ShieldedPoolTxEvent::try_from_slice(data)
                        .expect("shielded event should decode")
                },
            )
        })
        .expect("shielded event should be emitted through Noop CPI");
    assert_eq!(
        shielded_event.tx_kind,
        create_address_test_program::ShieldedPoolTxKind::ProoflessShield
    );
    assert_eq!(
        shielded_event.zone_config_hash,
        Some(LOCAL_DEV_ZONE_CONFIG_HASH)
    );
    assert_eq!(
        shielded_event.operation_commitment,
        LOCAL_DEV_OPERATION_COMMITMENT
    );
    assert_eq!(shielded_event.outputs.len(), 1);
    assert_eq!(shielded_event.outputs[0].compressed_output_index, 0);
    assert_eq!(shielded_event.outputs[0].utxo_hash, commitments.utxo_hash);
    assert_eq!(shielded_event.outputs[0].encrypted_utxo, encrypted_utxo);
    assert_eq!(
        shielded_event.outputs[0].encrypted_utxo_hash,
        commitments.encrypted_utxo_hash
    );

    let mut instruction_data = Vec::new();
    let mut program_ids = Vec::new();
    let mut instruction_accounts = Vec::new();
    for instruction in transaction.message.instructions.iter() {
        program_ids.push(transaction.message.account_keys[instruction.program_id_index as usize]);
        instruction_data.push(instruction.data.clone());
        instruction_accounts.push(
            instruction
                .accounts
                .iter()
                .map(|index| transaction.message.account_keys[*index as usize])
                .collect::<Vec<_>>(),
        );
    }
    for inner_instruction in simulation.meta.inner_instructions.iter().flatten() {
        program_ids.push(
            transaction.message.account_keys
                [inner_instruction.instruction.program_id_index as usize],
        );
        instruction_data.push(inner_instruction.instruction.data.clone());
        instruction_accounts.push(
            inner_instruction
                .instruction
                .accounts
                .iter()
                .map(|index| transaction.message.account_keys[*index as usize])
                .collect::<Vec<_>>(),
        );
    }
    let public_events = event_from_light_transaction(
        &program_ids.iter().map(|x| (*x).into()).collect::<Vec<_>>(),
        instruction_data.as_slice(),
        instruction_accounts
            .iter()
            .map(|accounts| accounts.iter().map(|x| (*x).into()).collect())
            .collect(),
    )
    .expect("Light public event parser should accept the simulated transaction")
    .expect("Light public event should be present");
    assert_eq!(public_events.len(), 1);

    let public_event = &public_events[0].event;
    assert_eq!(public_event.input_compressed_account_hashes.len(), 0);
    assert_eq!(public_event.output_compressed_accounts.len(), 1);
    assert_eq!(public_event.output_leaf_indices, vec![0]);
    assert_eq!(
        public_event.sequence_numbers,
        vec![MerkleTreeSequenceNumberV1 {
            tree_pubkey: env.v2_state_trees[0].merkle_tree.into(),
            seq: 0,
        }]
    );
    let compressed_account = &public_event.output_compressed_accounts[0].compressed_account;
    let expected_owner: light_compressed_account::Pubkey = create_address_test_program::ID.into();
    assert_eq!(compressed_account.owner, expected_owner);
    let compressed_data = compressed_account
        .data
        .as_ref()
        .expect("shielded UTXO compressed account must carry data hash");
    assert_eq!(
        compressed_data.discriminator,
        create_address_test_program::SHIELDED_UTXO_ACCOUNT_DISCRIMINATOR
    );
    assert_eq!(compressed_data.data_hash, commitments.utxo_hash);
    let compressed_account_hash = compressed_account
        .hash(&env.v2_state_trees[0].merkle_tree.into(), &0u32, true)
        .expect("shielded compressed account should hash");
    assert_eq!(
        public_event.output_compressed_account_hashes[0],
        compressed_account_hash
    );

    let capture = {
        let outer_instruction = transaction
            .message
            .instructions
            .first()
            .expect("proofless append tx has one outer instruction");
        let signer_placeholder = Pubkey::new_from_array([0x11; 32]);
        let shielded_program_instruction = capture_instruction(
            "shielded_program_instruction",
            transaction.message.account_keys[outer_instruction.program_id_index as usize],
            &outer_instruction.data,
            resolve_accounts(
                &transaction.message.account_keys,
                &outer_instruction.accounts,
                &payer.pubkey(),
                &signer_placeholder,
            ),
        );

        let mut light_system_instruction = None;
        let mut system_instruction = None;
        let mut account_compression_instruction = None;
        let mut shielded_event_noop_instruction = None;
        let noop_program =
            Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY);
        for inner_instruction in simulation.meta.inner_instructions.iter().flatten() {
            let instruction = &inner_instruction.instruction;
            let program_id =
                transaction.message.account_keys[instruction.program_id_index as usize];
            let accounts = resolve_accounts(
                &transaction.message.account_keys,
                &instruction.accounts,
                &payer.pubkey(),
                &signer_placeholder,
            );
            if program_id == light_system_program::ID {
                set_capture_once(
                    &mut light_system_instruction,
                    capture_instruction(
                        "light_system_instruction",
                        program_id,
                        &instruction.data,
                        accounts,
                    ),
                );
            } else if program_id == solana_sdk::system_program::id() {
                set_capture_once(
                    &mut system_instruction,
                    capture_instruction(
                        "system_instruction",
                        program_id,
                        &instruction.data,
                        accounts,
                    ),
                );
            } else if program_id == account_compression::ID {
                set_capture_once(
                    &mut account_compression_instruction,
                    capture_instruction(
                        "account_compression_instruction",
                        program_id,
                        &instruction.data,
                        accounts,
                    ),
                );
            } else if program_id == noop_program
                && create_address_test_program::ShieldedPoolTxEvent::matches_discriminator(
                    &instruction.data,
                )
            {
                set_capture_once(
                    &mut shielded_event_noop_instruction,
                    capture_instruction(
                        "shielded_event_noop_instruction",
                        program_id,
                        &instruction.data,
                        accounts,
                    ),
                );
            }
        }

        let utxo_tree =
            Pubkey::new_from_array(*public_event.sequence_numbers[0].tree_pubkey.array_ref());
        let output_queue = env.v2_state_trees[0].output_queue;
        let output_leaf_index = public_event.output_leaf_indices[0];
        let tree_sequence = public_event.sequence_numbers[0].seq;
        let batch_append_sequence = tree_sequence + 1;
        let batch_append_event = MetadataMerkleTreeEvent::BatchAppend(MetadataBatchEvent {
            merkle_tree_pubkey: utxo_tree.to_bytes(),
            batch_index: 0,
            zkp_batch_index: 0,
            zkp_batch_size: public_event.output_compressed_accounts.len() as u64,
            old_next_index: output_leaf_index as u64,
            new_next_index: output_leaf_index as u64
                + public_event.output_compressed_accounts.len() as u64,
            new_root: [0x91; 32],
            root_index: (batch_append_sequence % 64) as u32,
            sequence_number: batch_append_sequence,
            output_queue_pubkey: Some(output_queue.to_bytes()),
        });
        let batch_append_data = batch_append_event
            .try_to_vec()
            .expect("batch append capture should serialize");

        ProoflessAppendCaptureSnapshot {
            schema_version: 1,
            source: "program-tests/system-cpi-v2-test/proofless_shielded_append_emits_light_and_shielded_events".to_string(),
            transaction: ProoflessAppendCapturedTransaction {
                shielded_program_instruction,
                light_system_instruction: light_system_instruction
                    .expect("capture should include Light system CPI"),
                system_instruction: system_instruction.expect("capture should include system CPI"),
                account_compression_instruction: account_compression_instruction
                    .expect("capture should include account-compression CPI"),
                shielded_event_noop_instruction: shielded_event_noop_instruction
                    .expect("capture should include shielded Noop CPI"),
                batch_append_instruction: capture_instruction(
                    "batch_append_instruction",
                    account_compression::ID,
                    &[],
                    vec![utxo_tree],
                ),
                batch_append_noop_instruction: capture_instruction(
                    "batch_append_noop_instruction",
                    noop_program,
                    &batch_append_data,
                    vec![],
                ),
            },
            expected: ProoflessAppendCaptureExpected {
                zone_config_hash: hex_0x(&LOCAL_DEV_ZONE_CONFIG_HASH),
                operation_commitment: hex_0x(&LOCAL_DEV_OPERATION_COMMITMENT),
                data_hash: hex_0x(&LOCAL_DEV_DATA_HASH),
                utxo_hash: hex_0x(&LOCAL_DEV_UTXO_HASH),
                encrypted_utxo_hash: hex_0x(&LOCAL_DEV_ENCRYPTED_UTXO_HASH),
                compressed_account_hash: hex_0x(&compressed_account_hash),
                utxo_tree: utxo_tree.to_string(),
                output_queue: output_queue.to_string(),
                output_leaf_index,
                tree_sequence,
                batch_append_sequence,
            },
        }
    };
    maybe_write_or_assert_proofless_append_capture(&capture);

    rpc.context
        .send_transaction(transaction)
        .expect("proofless shielded append transaction should land");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
#[serial]
async fn proofless_shielded_spend_emits_shielded_nullifier_events() {
    let mut config = ProgramTestConfig::default_with_batched_trees(false);
    config.with_prover = true;
    config.additional_programs = Some(vec![(
        "create_address_test_program",
        create_address_test_program::ID,
    )]);

    let mut rpc = LightProgramTest::new(config)
        .await
        .expect("Failed to setup test programs with accounts");
    let env = rpc.test_accounts.clone();
    let payer = rpc.get_payer().insecure_clone();
    rpc.airdrop_lamports(&payer.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let mut spend_args =
        local_dev_proofless_shielded_spend_args(LOCAL_DEV_UTXO_HASH, LOCAL_DEV_UTXO_LEAF_INDEX);
    let expected_nullifier = spend_args.nullifiers[0];
    let expected_nullifier_chain = spend_args.nullifier_chain;
    let nullifier_tree = env.v2_address_trees[0];
    spend_args.nullifier_tree = nullifier_tree.to_bytes();
    let indexed_nullifier_address = shielded_spend_nullifier_address(
        expected_nullifier,
        nullifier_tree.to_bytes(),
        create_address_test_program::ID.to_bytes(),
    );
    let proof_result = rpc
        .get_validity_proof(
            Vec::new(),
            vec![AddressWithTree {
                address: indexed_nullifier_address,
                tree: nullifier_tree,
            }],
            None,
        )
        .await
        .expect("proofless shielded spend nullifier proof should be available")
        .value;
    let nullifier_address_params = vec![NewAddressParams {
        seed: expected_nullifier,
        address_queue_pubkey: nullifier_tree.into(),
        address_merkle_tree_pubkey: nullifier_tree.into(),
        address_merkle_tree_root_index: proof_result.get_address_root_indices()[0],
    }];
    let instruction =
        proofless_shielded_spend_instruction(ProoflessShieldedSpendInstructionInputs {
            signer: &payer.pubkey(),
            registered_program_pda: &env.protocol.registered_program_pda,
            args: spend_args,
            proof: proof_result.proof.0,
            nullifier_address_params,
        });
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        rpc.context.latest_blockhash(),
    );

    let simulation = rpc
        .context
        .simulate_transaction(transaction.clone())
        .expect("proofless shielded spend simulation should succeed");

    let mut shielded_tx_events = Vec::new();
    let mut nullifier_events = Vec::new();
    for inner_instruction in simulation.meta.inner_instructions.iter().flatten() {
        let data = &inner_instruction.instruction.data;
        if create_address_test_program::ShieldedPoolTxEvent::matches_discriminator(data) {
            shielded_tx_events.push(
                create_address_test_program::ShieldedPoolTxEvent::try_from_slice(data)
                    .expect("shielded spend event should decode"),
            );
        } else if create_address_test_program::ShieldedNullifierEvent::matches_discriminator(data) {
            nullifier_events.push(
                create_address_test_program::ShieldedNullifierEvent::try_from_slice(data)
                    .expect("shielded nullifier event should decode"),
            );
        }
    }

    assert_eq!(shielded_tx_events.len(), 1);
    let shielded_event = &shielded_tx_events[0];
    assert_eq!(
        shielded_event.tx_kind,
        create_address_test_program::ShieldedPoolTxKind::Transact
    );
    assert_eq!(
        shielded_event.zone_config_hash,
        Some(LOCAL_DEV_ZONE_CONFIG_HASH)
    );
    assert_eq!(
        shielded_event.operation_commitment,
        LOCAL_DEV_OPERATION_COMMITMENT
    );
    assert_eq!(shielded_event.outputs.len(), 0);
    assert_eq!(shielded_event.input_nullifiers, vec![expected_nullifier]);
    assert_eq!(shielded_event.nullifier_chain, expected_nullifier_chain);

    assert_eq!(nullifier_events.len(), 1);
    assert_eq!(nullifier_events[0].nullifier, expected_nullifier);
    assert_eq!(
        nullifier_events[0].nullifier_tree,
        nullifier_tree.to_bytes()
    );
    assert_eq!(nullifier_events[0].tx_event_index, 0);

    let mut instruction_data = Vec::new();
    let mut program_ids = Vec::new();
    let mut instruction_accounts = Vec::new();
    for instruction in transaction.message.instructions.iter() {
        program_ids.push(transaction.message.account_keys[instruction.program_id_index as usize]);
        instruction_data.push(instruction.data.clone());
        instruction_accounts.push(
            instruction
                .accounts
                .iter()
                .map(|index| transaction.message.account_keys[*index as usize])
                .collect::<Vec<_>>(),
        );
    }
    for inner_instruction in simulation.meta.inner_instructions.iter().flatten() {
        program_ids.push(
            transaction.message.account_keys
                [inner_instruction.instruction.program_id_index as usize],
        );
        instruction_data.push(inner_instruction.instruction.data.clone());
        instruction_accounts.push(
            inner_instruction
                .instruction
                .accounts
                .iter()
                .map(|index| transaction.message.account_keys[*index as usize])
                .collect::<Vec<_>>(),
        );
    }
    let public_events = event_from_light_transaction(
        &program_ids.iter().map(|x| (*x).into()).collect::<Vec<_>>(),
        instruction_data.as_slice(),
        instruction_accounts
            .iter()
            .map(|accounts| accounts.iter().map(|x| (*x).into()).collect())
            .collect(),
    )
    .expect("Light public event parser should accept the simulated spend transaction")
    .expect("Light public event should be present for spend nullifier insertion");
    assert_eq!(public_events.len(), 1);
    assert_eq!(public_events[0].event.output_compressed_accounts.len(), 0);
    assert_eq!(public_events[0].new_addresses.len(), 1);
    assert_eq!(
        public_events[0].new_addresses[0].address,
        indexed_nullifier_address
    );
    assert_eq!(
        Pubkey::new_from_array(*public_events[0].new_addresses[0].mt_pubkey.array_ref()),
        nullifier_tree
    );
    assert_eq!(
        public_events[0].address_sequence_numbers[0].tree_pubkey,
        light_compressed_account::Pubkey::from(nullifier_tree.to_bytes())
    );

    let capture = {
        let outer_instruction = transaction
            .message
            .instructions
            .first()
            .expect("proofless spend tx has one outer instruction");
        let signer_placeholder = Pubkey::new_from_array([0x11; 32]);
        let shielded_program_instruction = capture_instruction(
            "shielded_program_instruction",
            transaction.message.account_keys[outer_instruction.program_id_index as usize],
            &outer_instruction.data,
            resolve_accounts(
                &transaction.message.account_keys,
                &outer_instruction.accounts,
                &payer.pubkey(),
                &signer_placeholder,
            ),
        );

        let mut light_system_instruction = None;
        let mut system_instruction = None;
        let mut account_compression_instruction = None;
        let mut shielded_event_noop_instruction = None;
        let mut nullifier_event_noop_instructions = Vec::new();
        let noop_program =
            Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY);
        for inner_instruction in simulation.meta.inner_instructions.iter().flatten() {
            let instruction = &inner_instruction.instruction;
            let program_id =
                transaction.message.account_keys[instruction.program_id_index as usize];
            let accounts = resolve_accounts(
                &transaction.message.account_keys,
                &instruction.accounts,
                &payer.pubkey(),
                &signer_placeholder,
            );
            if program_id == light_system_program::ID {
                set_capture_once(
                    &mut light_system_instruction,
                    capture_instruction(
                        "light_system_instruction",
                        program_id,
                        &instruction.data,
                        accounts,
                    ),
                );
            } else if program_id == solana_sdk::system_program::id() {
                set_capture_once(
                    &mut system_instruction,
                    capture_instruction(
                        "system_instruction",
                        program_id,
                        &instruction.data,
                        accounts,
                    ),
                );
            } else if program_id == account_compression::ID {
                set_capture_once(
                    &mut account_compression_instruction,
                    capture_instruction(
                        "account_compression_instruction",
                        program_id,
                        &instruction.data,
                        accounts,
                    ),
                );
            } else if program_id == noop_program
                && create_address_test_program::ShieldedPoolTxEvent::matches_discriminator(
                    &instruction.data,
                )
            {
                set_capture_once(
                    &mut shielded_event_noop_instruction,
                    capture_instruction(
                        "shielded_event_noop_instruction",
                        program_id,
                        &instruction.data,
                        accounts,
                    ),
                );
            } else if program_id == noop_program
                && create_address_test_program::ShieldedNullifierEvent::matches_discriminator(
                    &instruction.data,
                )
            {
                let name = format!(
                    "nullifier_event_noop_instruction_{}",
                    nullifier_event_noop_instructions.len()
                );
                nullifier_event_noop_instructions.push(capture_instruction(
                    &name,
                    program_id,
                    &instruction.data,
                    accounts,
                ));
            }
        }
        assert_eq!(
            nullifier_event_noop_instructions.len(),
            1,
            "proofless spend capture should include one nullifier event"
        );

        ProoflessSpendCaptureSnapshot {
            schema_version: 1,
            source: "program-tests/system-cpi-v2-test/proofless_shielded_spend_emits_shielded_nullifier_events".to_string(),
            transaction: ProoflessSpendCapturedTransaction {
                shielded_program_instruction,
                light_system_instruction: light_system_instruction
                    .expect("capture should include Light system CPI"),
                system_instruction: system_instruction.expect("capture should include system CPI"),
                account_compression_instruction: account_compression_instruction
                    .expect("capture should include account-compression CPI"),
                shielded_event_noop_instruction: shielded_event_noop_instruction
                    .expect("capture should include shielded spend Noop CPI"),
                nullifier_event_noop_instructions,
            },
            expected: ProoflessSpendCaptureExpected {
                zone_config_hash: hex_0x(&LOCAL_DEV_ZONE_CONFIG_HASH),
                operation_commitment: hex_0x(&LOCAL_DEV_OPERATION_COMMITMENT),
                utxo_hash: hex_0x(&LOCAL_DEV_UTXO_HASH),
                utxo_leaf_index: LOCAL_DEV_UTXO_LEAF_INDEX,
                spend_nullifier: hex_0x(&expected_nullifier),
                nullifier_chain: hex_0x(
                    &expected_nullifier_chain.expect("single-input spend has nullifier chain"),
                ),
                nullifier_tree: hex_0x(&nullifier_tree.to_bytes()),
            },
        }
    };
    maybe_write_or_assert_proofless_spend_capture(&capture);

    rpc.context
        .send_transaction(transaction)
        .expect("proofless shielded spend transaction should land");
}

fn capture_instruction(
    name: &str,
    program_id: Pubkey,
    data: &[u8],
    accounts: Vec<Pubkey>,
) -> ProoflessAppendCapturedInstruction {
    ProoflessAppendCapturedInstruction {
        name: name.to_string(),
        program_id: program_id.to_string(),
        data: hex_0x(data),
        accounts: accounts
            .into_iter()
            .map(|account| account.to_string())
            .collect(),
    }
}

fn resolve_accounts(
    account_keys: &[Pubkey],
    account_indices: &[u8],
    signer: &Pubkey,
    signer_placeholder: &Pubkey,
) -> Vec<Pubkey> {
    account_indices
        .iter()
        .map(|index| {
            let account = account_keys[*index as usize];
            if account == *signer {
                *signer_placeholder
            } else {
                account
            }
        })
        .collect()
}

fn set_capture_once(
    slot: &mut Option<ProoflessAppendCapturedInstruction>,
    instruction: ProoflessAppendCapturedInstruction,
) {
    assert!(
        slot.replace(instruction).is_none(),
        "proofless append capture saw duplicate instruction"
    );
}

fn maybe_write_or_assert_proofless_append_capture(capture: &ProoflessAppendCaptureSnapshot) {
    let actual = serde_json::to_value(capture).expect("proofless append capture should encode");
    if let Ok(path) = std::env::var("PHOTON_PROOFLESS_APPEND_CAPTURE_EXPECTED") {
        let expected = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read expected capture {path}: {err}"));
        let expected: serde_json::Value =
            serde_json::from_str(&expected).expect("expected capture should be valid JSON");
        assert_eq!(actual, expected, "proofless append capture drifted");
    }
    if let Ok(path) = std::env::var("PHOTON_PROOFLESS_APPEND_CAPTURE_OUT") {
        let encoded = serde_json::to_string_pretty(&actual)
            .expect("proofless append capture should encode")
            + "\n";
        fs::write(&path, encoded)
            .unwrap_or_else(|err| panic!("failed to write capture {path}: {err}"));
    }
}

fn maybe_write_or_assert_proofless_spend_capture(capture: &ProoflessSpendCaptureSnapshot) {
    let actual = serde_json::to_value(capture).expect("proofless spend capture should encode");
    if let Ok(path) = std::env::var("PHOTON_PROOFLESS_SPEND_CAPTURE_EXPECTED") {
        let expected = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read expected spend capture {path}: {err}"));
        let expected: serde_json::Value =
            serde_json::from_str(&expected).expect("expected spend capture should be valid JSON");
        assert_eq!(actual, expected, "proofless spend capture drifted");
    }
    if let Ok(path) = std::env::var("PHOTON_PROOFLESS_SPEND_CAPTURE_OUT") {
        let encoded = serde_json::to_string_pretty(&actual)
            .expect("proofless spend capture should encode")
            + "\n";
        fs::write(&path, encoded)
            .unwrap_or_else(|err| panic!("failed to write spend capture {path}: {err}"));
    }
}

fn hex_0x(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

/// 1 output compressed account
#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
#[serial]
#[ignore]
async fn generate_photon_test_data_multiple_events() {
    for num_expected_events in 4..5 {
        spawn_validator(LightValidatorConfig {
            enable_indexer: false,
            enable_prover: true,
            wait_time: 10,
            sbf_programs: vec![(
                create_address_test_program::ID.to_string(),
                "../../target/deploy/create_address_test_program.so".to_string(),
            )],
            upgradeable_programs: vec![],
            limit_ledger_size: None,
            use_surfpool: true,
            validator_args: vec![],
        })
        .await;

        let mut rpc = LightClient::new(LightClientConfig::local_no_indexer())
            .await
            .unwrap();
        let env = TestAccounts::get_local_test_validator_accounts();

        let payer = rpc.get_payer().insecure_clone();
        rpc.airdrop_lamports(&payer.pubkey(), 10_000_000_000)
            .await
            .unwrap();
        let output_accounts = vec![get_compressed_output_account(
            true,
            env.v2_state_trees[0].output_queue,
        )];
        let (events, output_accounts, _) = perform_test_transaction(
            &mut rpc,
            &payer,
            vec![],
            output_accounts,
            vec![],
            Some(num_expected_events),
            None,
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(events.len(), num_expected_events as usize);
        let expected_batched_event = BatchPublicTransactionEvent {
            event: PublicTransactionEvent {
                input_compressed_account_hashes: Vec::new(),
                output_leaf_indices: vec![0],
                output_compressed_account_hashes: vec![output_accounts[0]
                    .compressed_account
                    .hash(&env.v2_state_trees[0].merkle_tree.into(), &0u32, true)
                    .unwrap()],
                output_compressed_accounts: output_accounts.to_vec(),
                sequence_numbers: vec![MerkleTreeSequenceNumberV1 {
                    tree_pubkey: env.v2_state_trees[0].merkle_tree.into(),
                    // queue_pubkey: env.v2_state_trees[0].output_queue,
                    // tree_type: TreeType::StateV2 as u64,
                    seq: 0,
                }],
                relay_fee: None,
                message: None,
                is_compress: false,
                compress_or_decompress_lamports: None,
                pubkey_array: vec![env.v2_state_trees[0].output_queue.into()],
                ata_owners: vec![],
            },
            address_sequence_numbers: Vec::new(),
            input_sequence_numbers: Vec::new(),
            batch_input_accounts: Vec::new(),
            new_addresses: Vec::new(),
            tx_hash: [0u8; 32],
        };
        assert_eq!(events[0], expected_batched_event);
        for i in 1..num_expected_events {
            let mut expected_event = expected_batched_event.clone();
            expected_event.event.sequence_numbers = vec![MerkleTreeSequenceNumberV1 {
                tree_pubkey: env.v2_state_trees[0].merkle_tree.into(),
                // queue_pubkey: env.v2_state_trees[0].output_queue,
                // tree_type: TreeType::StateV2 as u64,
                seq: i as u64,
            }];
            expected_event.event.output_compressed_account_hashes = vec![output_accounts[0]
                .clone()
                .compressed_account
                .hash(&env.v2_state_trees[0].merkle_tree.into(), &(i as u32), true)
                .unwrap()];
            expected_event.event.output_leaf_indices = vec![i as u32];
            assert_eq!(events[i as usize], expected_event);
        }
    }
}

pub fn get_compressed_input_account(
    merkle_context: MerkleContext,
) -> CompressedAccountWithMerkleContext {
    CompressedAccountWithMerkleContext {
        compressed_account: CompressedAccount {
            owner: create_address_test_program::ID.into(),
            lamports: 0,
            address: None,
            data: Some(CompressedAccountData {
                data: vec![2u8; 31],
                discriminator: u64::MAX.to_be_bytes(),
                data_hash: [3u8; 32],
            }),
        },
        merkle_context,
    }
}

pub fn get_compressed_output_account(
    data: bool,
    merkle_tree: Pubkey,
) -> OutputCompressedAccountWithContext {
    OutputCompressedAccountWithContext {
        compressed_account: CompressedAccount {
            owner: create_address_test_program::ID.into(),
            lamports: 0,
            address: None,
            data: if data {
                Some(CompressedAccountData {
                    data: vec![2u8; 31],
                    discriminator: u64::MAX.to_be_bytes(),
                    data_hash: [3u8; 32],
                })
            } else {
                None
            },
        },
        merkle_tree: merkle_tree.into(),
    }
}

pub async fn perform_test_transaction<R: Rpc>(
    rpc: &mut R,
    payer: &Keypair,
    input_accounts: Vec<CompressedAccountWithMerkleContext>,
    output_accounts: Vec<OutputCompressedAccountWithContext>,
    new_addresses: Vec<NewAddressParamsAssigned>,
    num_cpis: Option<u8>,
    proof: Option<CompressedProof>,
) -> Result<
    Option<(
        Vec<BatchPublicTransactionEvent>,
        Vec<OutputCompressedAccountWithPackedContext>,
        Vec<PackedCompressedAccountWithMerkleContext>,
    )>,
    RpcError,
> {
    let mut remaining_accounts = HashMap::<Pubkey, usize>::new();

    let packed_new_address_params =
        pack_new_address_params_assigned(new_addresses.as_slice(), &mut remaining_accounts);

    let packed_inputs = pack_compressed_accounts(
        input_accounts.as_slice(),
        &vec![None; input_accounts.len()],
        &mut remaining_accounts,
    );
    let output_compressed_accounts = pack_output_compressed_accounts(
        output_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<_>>()
            .as_slice(),
        output_accounts
            .iter()
            .map(|x| x.merkle_tree.into())
            .collect::<Vec<_>>()
            .as_slice(),
        &mut remaining_accounts,
    );

    let ix_data = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: 255,
        with_cpi_context: false,
        invoking_program_id: create_address_test_program::ID.into(),
        proof,
        new_address_params: packed_new_address_params,
        is_compress: false,
        compress_or_decompress_lamports: 0,
        output_compressed_accounts: output_compressed_accounts.clone(),
        input_compressed_accounts: packed_inputs
            .iter()
            .map(|x| InAccount {
                address: x.compressed_account.address,
                merkle_context: x.merkle_context,
                lamports: x.compressed_account.lamports,
                discriminator: x.compressed_account.data.as_ref().unwrap().discriminator,
                data_hash: x.compressed_account.data.as_ref().unwrap().data_hash,
                root_index: x.root_index,
            })
            .collect::<Vec<_>>(),
        with_transaction_hash: true,
        ..Default::default()
    };
    let remaining_accounts = to_account_metas(remaining_accounts);
    let instruction = create_invoke_cpi_instruction(
        payer.pubkey(),
        [
            light_system_program::instruction::InvokeCpiWithReadOnly::DISCRIMINATOR.to_vec(),
            ix_data.try_to_vec().unwrap(),
        ]
        .concat(),
        remaining_accounts,
        num_cpis,
    );
    let res = rpc
        .create_and_send_transaction_with_batched_event(&[instruction], &payer.pubkey(), &[payer])
        .await?;
    if let Some(res) = res {
        Ok(Some((res.0, output_compressed_accounts, packed_inputs)))
    } else {
        Ok(None)
    }
}

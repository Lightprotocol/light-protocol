#![cfg(feature = "test-sbf")]

use std::collections::HashMap;

use anchor_lang::{prelude::borsh::BorshSerialize, Discriminator};
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
        data::{OutputCompressedAccountWithContext, OutputCompressedAccountWithPackedContext},
        with_readonly::{InAccount, InstructionDataInvokeCpiWithReadOnly},
    },
    nullifier::create_nullifier,
    tx_hash::create_tx_hash,
    TreeType,
};
use light_compressed_token::process_transfer::transfer_sdk::to_account_metas;
use light_event::event::{
    BatchNullifyContext, BatchPublicTransactionEvent, MerkleTreeSequenceNumber,
    MerkleTreeSequenceNumberV1, NewAddress, PublicTransactionEvent,
};
use light_program_test::{
    accounts::test_accounts::TestAccounts, LightProgramTest, ProgramTestConfig,
};
use light_sdk::address::NewAddressParamsAssigned;
use light_test_utils::{
    pack::{
        pack_compressed_accounts, pack_new_address_params_assigned, pack_output_compressed_accounts,
    },
    LightClient, Rpc, RpcError,
};
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

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
            limit_ledger_size: None,
            grpc_port: None,
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

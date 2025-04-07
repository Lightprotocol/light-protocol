// #![cfg(feature = "test-sbf")]

use std::collections::HashMap;

use anchor_lang::prelude::borsh::BorshSerialize;
use create_address_test_program::create_invoke_cpi_instruction;
use light_compressed_account::{
    address::{derive_address, derive_address_legacy, pack_new_address_params},
    compressed_account::{
        pack_compressed_accounts, pack_output_compressed_accounts, CompressedAccount,
        CompressedAccountData, CompressedAccountWithMerkleContext, MerkleContext,
        PackedCompressedAccountWithMerkleContext,
    },
    indexer_event::event::{
        BatchNullifyContext, BatchPublicTransactionEvent, MerkleTreeSequenceNumber, NewAddress,
        PublicTransactionEvent,
    },
    instruction_data::{
        compressed_proof::CompressedProof,
        data::{
            NewAddressParams, OutputCompressedAccountWithContext,
            OutputCompressedAccountWithPackedContext,
        },
        invoke_cpi::InstructionDataInvokeCpi,
        with_readonly::{InAccount, InstructionDataInvokeCpiWithReadOnly},
    },
    nullifier::create_nullifier,
    tx_hash::create_tx_hash,
    TreeType,
};
use light_compressed_token::process_transfer::transfer_sdk::to_account_metas;
use light_program_test::{
    indexer::{TestIndexer, TestIndexerExtensions},
    test_env::{setup_test_programs_with_accounts, EnvAccounts},
    test_rpc::ProgramTestRpcConnection,
};
use light_prover_client::gnark::helpers::{
    spawn_prover, spawn_validator, LightValidatorConfig, ProverConfig, ProverMode,
};
use light_test_utils::{RpcConnection, RpcError, SolanaRpcConnection, SolanaRpcUrl};
use serial_test::serial;
use solana_sdk::{
    commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair, signer::Signer,
};

// TODO: add test with multiple batched address trees before we activate batched addresses
#[tokio::test]
#[serial]
async fn parse_batched_event_functional() {
    let (mut rpc, env) = setup_test_programs_with_accounts(Some(vec![(
        String::from("create_address_test_program"),
        create_address_test_program::ID,
    )]))
    .await;
    spawn_prover(
        false,
        ProverConfig {
            run_mode: Some(ProverMode::Rpc),
            circuits: vec![],
        },
    )
    .await;

    let payer = rpc.get_payer().insecure_clone();
    // Insert 8 output accounts that we can use as inputs.
    {
        let num_expected_events = 1;
        let output_accounts =
            vec![get_compressed_output_account(true, env.batched_output_queue,); 8];
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
                            .hash(&env.batched_state_merkle_tree, &(i as u32), true)
                            .unwrap()
                    })
                    .collect::<Vec<_>>(),
                output_compressed_accounts: output_accounts.to_vec(),
                sequence_numbers: vec![MerkleTreeSequenceNumber {
                    tree_pubkey: env.batched_state_merkle_tree,
                    queue_pubkey: env.batched_output_queue,
                    tree_type: TreeType::BatchedState as u64,
                    seq: 0,
                }],
                relay_fee: None,
                message: None,
                is_compress: false,
                compress_or_decompress_lamports: None,
                pubkey_array: vec![env.batched_output_queue],
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
            vec![get_compressed_output_account(true, env.batched_output_queue,); 8];
        let input_accounts = (0..8)
            .map(|i| {
                get_compressed_input_account(MerkleContext {
                    leaf_index: i,
                    merkle_tree_pubkey: env.batched_state_merkle_tree,
                    prove_by_index: true,
                    nullifier_queue_pubkey: env.batched_output_queue,
                    tree_type: light_compressed_account::TreeType::BatchedState,
                })
            })
            .collect::<Vec<_>>();

        let new_addresses = vec![
            derive_address_legacy(&env.address_merkle_tree_pubkey, &[1u8; 32]).unwrap(),
            derive_address_legacy(&env.address_merkle_tree_pubkey, &[2u8; 32]).unwrap(),
        ];
        let payer = rpc.get_payer().insecure_clone();
        let mut test_indexer =
            TestIndexer::<ProgramTestRpcConnection>::init_from_env(&payer, &env, None).await;
        let proof_res = test_indexer
            .create_proof_for_compressed_accounts2(
                None,
                None,
                Some(&new_addresses),
                Some(vec![env.address_merkle_tree_pubkey; 2]),
                &mut rpc,
            )
            .await;

        let new_address_params = vec![
            NewAddressParams {
                seed: [1u8; 32],
                address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
                address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
                address_merkle_tree_root_index: proof_res.address_root_indices[0],
            },
            NewAddressParams {
                seed: [2u8; 32],
                address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
                address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
                address_merkle_tree_root_index: proof_res.address_root_indices[1],
            },
        ];
        let (events, output_accounts, _) = perform_test_transaction(
            &mut rpc,
            &payer,
            input_accounts.to_vec(),
            output_accounts,
            new_address_params,
            None,
            proof_res.proof,
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
                        &env.batched_state_merkle_tree,
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
                    .hash(&env.batched_state_merkle_tree, &((i + 8) as u32), true)
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
                input_compressed_account_hashes: input_accounts
                    .iter()
                    .map(|x| x.hash().unwrap())
                    .collect::<Vec<_>>(),
                output_leaf_indices: (8..16).collect(),
                output_compressed_account_hashes: output_accounts
                    .iter()
                    .enumerate()
                    .map(|(i, x)| {
                        x.compressed_account
                            .hash(&env.batched_state_merkle_tree, &((i + 8) as u32), true)
                            .unwrap()
                    })
                    .collect::<Vec<_>>(),
                output_compressed_accounts: output_accounts.to_vec(),
                sequence_numbers: vec![MerkleTreeSequenceNumber {
                    tree_pubkey: env.batched_state_merkle_tree,
                    queue_pubkey: env.batched_output_queue,
                    tree_type: TreeType::BatchedState as u64,
                    seq: 8,
                }],
                relay_fee: None,
                message: None,
                is_compress: false,
                compress_or_decompress_lamports: None,
                pubkey_array: vec![
                    env.address_merkle_tree_pubkey,
                    env.address_merkle_tree_queue_pubkey,
                    env.batched_state_merkle_tree,
                    env.batched_output_queue,
                ],
            },
            address_sequence_numbers: Vec::new(),
            input_sequence_numbers: vec![MerkleTreeSequenceNumber {
                tree_pubkey: env.batched_state_merkle_tree,
                queue_pubkey: env.batched_output_queue,
                tree_type: TreeType::BatchedState as u64,
                seq: 0,
            }],
            batch_input_accounts,
            new_addresses: new_addresses
                .iter()
                .map(|x| NewAddress {
                    address: *x,
                    mt_pubkey: env.address_merkle_tree_pubkey,
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
            vec![get_compressed_output_account(true, env.batched_output_queue,); 8];
        let input_accounts = (8..16)
            .map(|i| {
                get_compressed_input_account(MerkleContext {
                    leaf_index: i,
                    merkle_tree_pubkey: env.batched_state_merkle_tree,
                    prove_by_index: true,
                    nullifier_queue_pubkey: env.batched_output_queue,
                    tree_type: light_compressed_account::TreeType::BatchedState,
                })
            })
            .collect::<Vec<_>>();

        let new_addresses = vec![
            derive_address(
                &[1u8; 32],
                &env.batch_address_merkle_tree.to_bytes(),
                &create_address_test_program::ID.to_bytes(),
            ),
            derive_address(
                &[2u8; 32],
                &env.batch_address_merkle_tree.to_bytes(),
                &create_address_test_program::ID.to_bytes(),
            ),
        ];
        let payer = rpc.get_payer().insecure_clone();
        let mut test_indexer =
            TestIndexer::<ProgramTestRpcConnection>::init_from_env(&payer, &env, None).await;
        let proof_res = test_indexer
            .create_proof_for_compressed_accounts2(
                None,
                None,
                Some(&new_addresses),
                Some(vec![env.batch_address_merkle_tree; 2]),
                &mut rpc,
            )
            .await;

        let new_address_params = vec![
            NewAddressParams {
                seed: [1u8; 32],
                address_queue_pubkey: env.batch_address_merkle_tree,
                address_merkle_tree_pubkey: env.batch_address_merkle_tree,
                address_merkle_tree_root_index: proof_res.address_root_indices[0],
            },
            NewAddressParams {
                seed: [2u8; 32],
                address_queue_pubkey: env.batch_address_merkle_tree,
                address_merkle_tree_pubkey: env.batch_address_merkle_tree,
                address_merkle_tree_root_index: proof_res.address_root_indices[1],
            },
        ];
        let (events, output_accounts, _) = perform_test_transaction(
            &mut rpc,
            &payer,
            input_accounts.to_vec(),
            output_accounts,
            new_address_params,
            None,
            proof_res.proof,
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
                        &env.batched_state_merkle_tree,
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
                    .hash(&env.batched_state_merkle_tree, &((i + 16) as u32), true)
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
                input_compressed_account_hashes: input_accounts
                    .iter()
                    .map(|x| x.hash().unwrap())
                    .collect::<Vec<_>>(),
                output_leaf_indices: (16..24).collect(),
                output_compressed_account_hashes: output_accounts
                    .iter()
                    .enumerate()
                    .map(|(i, x)| {
                        x.compressed_account
                            .hash(&env.batched_state_merkle_tree, &((i + 16) as u32), true)
                            .unwrap()
                    })
                    .collect::<Vec<_>>(),
                output_compressed_accounts: output_accounts.to_vec(),
                sequence_numbers: vec![MerkleTreeSequenceNumber {
                    tree_pubkey: env.batched_state_merkle_tree,
                    queue_pubkey: env.batched_output_queue,
                    tree_type: TreeType::BatchedState as u64,
                    seq: 16,
                }],
                relay_fee: None,
                message: None,
                is_compress: false,
                compress_or_decompress_lamports: None,
                pubkey_array: vec![
                    env.batch_address_merkle_tree,
                    env.batched_state_merkle_tree,
                    env.batched_output_queue,
                ],
            },
            address_sequence_numbers: vec![MerkleTreeSequenceNumber {
                tree_pubkey: env.batch_address_merkle_tree,
                queue_pubkey: Pubkey::default(),
                tree_type: TreeType::BatchedAddress as u64,
                seq: 0,
            }],
            input_sequence_numbers: vec![MerkleTreeSequenceNumber {
                tree_pubkey: env.batched_state_merkle_tree,
                queue_pubkey: env.batched_output_queue,
                tree_type: TreeType::BatchedState as u64,
                seq: 8,
            }],
            batch_input_accounts,
            new_addresses: new_addresses
                .iter()
                .map(|x| NewAddress {
                    address: *x,
                    mt_pubkey: env.batch_address_merkle_tree,
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
        let (mut rpc, env) = setup_test_programs_with_accounts(Some(vec![(
            String::from("create_address_test_program"),
            create_address_test_program::ID,
        )]))
        .await;

        let payer = rpc.get_payer().insecure_clone();
        rpc.airdrop_lamports(&payer.pubkey(), 10_000_000_000)
            .await
            .unwrap();
        let output_accounts = vec![get_compressed_output_account(
            true,
            env.batched_output_queue,
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
                    .hash(&env.batched_state_merkle_tree, &0u32, true)
                    .unwrap()],
                output_compressed_accounts: output_accounts.to_vec(),
                sequence_numbers: vec![MerkleTreeSequenceNumber {
                    tree_pubkey: env.batched_state_merkle_tree,
                    queue_pubkey: env.batched_output_queue,
                    tree_type: TreeType::BatchedState as u64,
                    seq: 0,
                }],
                relay_fee: None,
                message: None,
                is_compress: false,
                compress_or_decompress_lamports: None,
                pubkey_array: vec![env.batched_output_queue],
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
            expected_event.event.sequence_numbers = vec![MerkleTreeSequenceNumber {
                tree_pubkey: env.batched_state_merkle_tree,
                queue_pubkey: env.batched_output_queue,
                tree_type: TreeType::BatchedState as u64,
                seq: i as u64,
            }];
            expected_event.event.output_compressed_account_hashes = vec![output_accounts[0]
                .clone()
                .compressed_account
                .hash(&env.batched_state_merkle_tree, &(i as u32), true)
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
            wait_time: 10,
            prover_config: None,
            sbf_programs: vec![(
                create_address_test_program::ID.to_string(),
                "../../target/deploy/create_address_test_program.so".to_string(),
            )],
            limit_ledger_size: None,
        })
        .await;
        let mut rpc =
            SolanaRpcConnection::new(SolanaRpcUrl::Localnet, Some(CommitmentConfig::confirmed()));
        let env = EnvAccounts::get_local_test_validator_accounts();

        let payer = rpc.get_payer().insecure_clone();
        rpc.airdrop_lamports(&payer.pubkey(), 10_000_000_000)
            .await
            .unwrap();
        let output_accounts = vec![get_compressed_output_account(
            true,
            env.batched_output_queue,
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
                    .hash(&env.batched_state_merkle_tree, &0u32, true)
                    .unwrap()],
                output_compressed_accounts: output_accounts.to_vec(),
                sequence_numbers: vec![MerkleTreeSequenceNumber {
                    tree_pubkey: env.batched_state_merkle_tree,
                    queue_pubkey: env.batched_output_queue,
                    tree_type: TreeType::BatchedState as u64,
                    seq: 0,
                }],
                relay_fee: None,
                message: None,
                is_compress: false,
                compress_or_decompress_lamports: None,
                pubkey_array: vec![env.batched_output_queue],
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
            expected_event.event.sequence_numbers = vec![MerkleTreeSequenceNumber {
                tree_pubkey: env.batched_state_merkle_tree,
                queue_pubkey: env.batched_output_queue,
                tree_type: TreeType::BatchedState as u64,
                seq: i as u64,
            }];
            expected_event.event.output_compressed_account_hashes = vec![output_accounts[0]
                .clone()
                .compressed_account
                .hash(&env.batched_state_merkle_tree, &(i as u32), true)
                .unwrap()];
            expected_event.event.output_leaf_indices = vec![i as u32];
            assert_eq!(events[i as usize], expected_event);
        }
    }
}

fn get_compressed_input_account(
    merkle_context: MerkleContext,
) -> CompressedAccountWithMerkleContext {
    CompressedAccountWithMerkleContext {
        compressed_account: CompressedAccount {
            owner: create_address_test_program::ID,
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

fn get_compressed_output_account(
    data: bool,
    merkle_tree: Pubkey,
) -> OutputCompressedAccountWithContext {
    OutputCompressedAccountWithContext {
        compressed_account: CompressedAccount {
            owner: create_address_test_program::ID,
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
        merkle_tree,
    }
}

async fn perform_test_transaction<R: RpcConnection>(
    rpc: &mut R,
    payer: &Keypair,
    input_accounts: Vec<CompressedAccountWithMerkleContext>,
    output_accounts: Vec<OutputCompressedAccountWithContext>,
    new_addresses: Vec<NewAddressParams>,
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
        pack_new_address_params(new_addresses.as_slice(), &mut remaining_accounts);

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
            .map(|x| x.merkle_tree)
            .collect::<Vec<_>>()
            .as_slice(),
        &mut remaining_accounts,
    );
    let inputs_struct = InstructionDataInvokeCpi {
        proof,
        new_address_params: packed_new_address_params,
        input_compressed_accounts_with_merkle_context: packed_inputs.clone(),
        output_compressed_accounts: output_compressed_accounts.clone(),
        relay_fee: None,
        compress_or_decompress_lamports: None,
        is_compress: false,
        cpi_context: None,
    };

    let ix_data = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: 255, // TODO: correct
        with_cpi_context: inputs_struct.cpi_context.is_some(),
        invoking_program_id: create_address_test_program::ID.into(),
        proof: inputs_struct.proof,
        new_address_params: inputs_struct.new_address_params,
        is_decompress: false,
        compress_or_decompress_lamports: inputs_struct
            .compress_or_decompress_lamports
            .unwrap_or_default(),
        output_compressed_accounts: inputs_struct.output_compressed_accounts,
        input_compressed_accounts: inputs_struct
            .input_compressed_accounts_with_merkle_context
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
        ..Default::default()
    };
    let remaining_accounts = to_account_metas(remaining_accounts);
    let instruction = create_invoke_cpi_instruction(
        payer.pubkey(),
        ix_data.try_to_vec().unwrap(),
        remaining_accounts,
        num_cpis,
    );
    let res = rpc
        .create_and_send_transaction_with_batched_event(
            &[instruction],
            &payer.pubkey(),
            &[payer],
            None,
        )
        .await?;
    if let Some(res) = res {
        Ok(Some((res.0, output_compressed_accounts, packed_inputs)))
    } else {
        Ok(None)
    }
}

#![cfg(feature = "test-sbf")]

mod event;

use event::{get_compressed_input_account, get_compressed_output_account};
use light_account_checks::error::AccountError;
use light_batched_merkle_tree::{
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    initialize_state_tree::InitStateTreeAccountsInstructionData,
};
use light_client::{
    indexer::{AddressWithTree, Context, Indexer, Response, ValidityProofWithContext},
    rpc::Rpc,
};
use light_compressed_account::{
    address::{derive_address, derive_address_legacy},
    compressed_account::{MerkleContext, PackedMerkleContext, ReadOnlyCompressedAccount},
    constants::ACCOUNT_COMPRESSION_PROGRAM_ID,
    instruction_data::{
        cpi_context::CompressedCpiContext,
        with_account_info::{CompressedAccountInfo, InAccountInfo, OutAccountInfo},
    },
    TreeType,
};
use light_program_test::{
    indexer::{TestIndexer, TestIndexerExtensions},
    utils::assert::assert_rpc_error,
    LightProgramTest, ProgramTestConfig,
};
use light_prover_client::prover::spawn_prover;
use light_sdk::{
    address::{NewAddressParamsAssigned, ReadOnlyAddress},
    instruction::ValidityProof,
};
use light_system_program::errors::SystemProgramError;
use rand::{thread_rng, Rng};
use serial_test::serial;
use solana_sdk::pubkey::Pubkey;
/// Test with read only instruction with different input combinations:
/// - anchor compat accounts
/// - small accounts
/// - with V1 and V2 trees
/// readonly_accounts  0..4; skipped for V1 trees
/// readonly_addresses = 0..2; skipped for V1 trees
/// input_accounts = 0..4;
/// output_accounts = 0..4;
#[serial]
#[tokio::test]
async fn functional_read_only() {
    spawn_prover().await;
    for (batched, is_v2_ix) in [(true, false), (true, true), (false, false), (false, true)] {
        let config = if batched {
            let mut config = ProgramTestConfig::default_with_batched_trees(false);
            config.with_prover = false;
            config.additional_programs = Some(vec![(
                "create_address_test_program",
                create_address_test_program::ID,
            )]);
            config.v2_state_tree_config = Some(InitStateTreeAccountsInstructionData::default());
            config.v2_address_tree_config =
                Some(InitAddressTreeAccountsInstructionData::test_default());
            config
        } else {
            ProgramTestConfig::new(
                false,
                Some(vec![(
                    "create_address_test_program",
                    create_address_test_program::ID,
                )]),
            )
        };
        let mut rpc = LightProgramTest::new(config)
            .await
            .expect("Failed to setup test programs with accounts");
        let env = rpc.test_accounts.clone();
        let queue = if batched {
            env.v2_state_trees[0].output_queue
        } else {
            env.v1_state_trees[0].nullifier_queue
        };
        let tree = if batched {
            env.v2_state_trees[0].merkle_tree
        } else {
            env.v1_state_trees[0].merkle_tree
        };
        let address_tree = if batched {
            env.v2_address_trees[0]
        } else {
            env.v1_address_trees[0].merkle_tree
        };

        let payer = rpc.get_payer().insecure_clone();
        let mut test_indexer = TestIndexer::init_from_acounts(&payer, &env, 0).await;
        // Create a bunch of outputs that we can use as inputs.
        for _ in 0..5 {
            let output_accounts = vec![
                get_compressed_output_account(
                    true,
                    if batched {
                        env.v2_state_trees[0].output_queue
                    } else {
                        env.v1_state_trees[0].merkle_tree
                    }
                );
                30
            ];
            local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                output_accounts,
                vec![],
                None,
                None,
                None,
                is_v2_ix,
                true,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await
            .unwrap();
        }

        let mut rng = thread_rng();

        let mut input_merkle_tree_index = 4;
        let max_readonly_accounts = 4;
        let max_readonly_addresses = 2;
        let max_input_accounts = 4;
        let max_output_accounts = 4;
        // With v1 trees -> proof by zkp
        for num_outputs in 1..=max_output_accounts {
            for num_inputs in 0..=max_input_accounts {
                for mut num_read_only_accounts in 0..max_readonly_accounts {
                    for mut num_read_only_addresses in 0..max_readonly_addresses {
                        if !batched {
                            num_read_only_addresses = 0;
                            num_read_only_accounts = 0;
                        }
                        println!("num_outputs: {}, num_inputs: {}, num_read_only_accounts: {}, num_read_only_addresses: {}", num_outputs, num_inputs, num_read_only_accounts, num_read_only_addresses);
                        let input_accounts = (input_merkle_tree_index
                            ..num_inputs + input_merkle_tree_index)
                            .map(|i| {
                                println!("input leaf index: {}", i);
                                get_input_account_info(PackedMerkleContext {
                                    leaf_index: i,
                                    merkle_tree_pubkey_index: 1,
                                    queue_pubkey_index: 0,
                                    prove_by_index: batched,
                                })
                            })
                            .collect::<Vec<_>>();
                        input_merkle_tree_index += num_inputs;
                        let output_accounts = (0..num_outputs)
                            .map(|_| get_output_account_info(if batched { 0 } else { 1 }))
                            .collect::<Vec<_>>();
                        let mut account_infos = Vec::new();
                        for i in 0..num_inputs {
                            let output = if num_outputs > i {
                                Some(output_accounts[i as usize].clone())
                            } else {
                                None
                            };

                            account_infos.push(CompressedAccountInfo {
                                address: None,
                                input: Some(input_accounts[i as usize].clone()),
                                output,
                            });
                        }
                        for i in num_inputs..num_outputs {
                            account_infos.push(CompressedAccountInfo {
                                address: None,
                                input: None,
                                output: Some(output_accounts[i as usize].clone()),
                            });
                        }
                        let read_only_accounts = (1..=num_read_only_accounts)
                            .map(|i| {
                                println!("read only leaf index: {}", i);
                                let accounts = test_indexer
                                    .get_compressed_accounts_with_merkle_context_by_owner(
                                        &create_address_test_program::ID,
                                    );
                                let account = accounts
                                    .get(accounts.len().saturating_sub(i as usize))
                                    .unwrap();
                                println!("leaf index {}", account.merkle_context.leaf_index);
                                let mut merkle_context = account.merkle_context;
                                if batched {
                                    merkle_context.prove_by_index = true;
                                }
                                ReadOnlyCompressedAccount {
                                    merkle_context,
                                    account_hash: account.hash().unwrap(),
                                    root_index: 0,
                                }
                            })
                            .collect::<Vec<_>>();
                        let read_only_addresses = (0..num_read_only_addresses)
                            .map(|_| {
                                let mut address = rng.gen::<[u8; 32]>();
                                address[0] = 0;
                                address
                            })
                            .collect::<Vec<_>>();
                        let proof_res = if read_only_addresses.is_empty() && num_inputs == 0 {
                            ValidityProofWithContext {
                                proof: ValidityProof::default(),
                                accounts: vec![],
                                addresses: vec![],
                            }
                        } else {
                            let input_hashes = if num_inputs == 0 {
                                None
                            } else {
                                let hashes: Vec<[u8; 32]> = if batched {
                                    input_accounts
                                        .iter()
                                        .map(|account| {
                                            test_indexer
                                                .state_merkle_trees
                                                .iter()
                                                .find(|x| x.accounts.merkle_tree == tree)
                                                .unwrap()
                                                .output_queue_elements
                                                [account.merkle_context.leaf_index as usize]
                                                .0
                                        })
                                        .collect::<Vec<_>>()
                                } else {
                                    input_accounts
                                        .iter()
                                        .map(|account| {
                                            test_indexer
                                                .state_merkle_trees
                                                .iter()
                                                .find(|x| x.accounts.merkle_tree == tree)
                                                .unwrap()
                                                .merkle_tree
                                                .get_leaf(
                                                    account.merkle_context.leaf_index as usize,
                                                )
                                                .unwrap()
                                        })
                                        .collect::<Vec<_>>()
                                };
                                Some(hashes)
                            };
                            let (new_addresses, address_tree_pubkey) =
                                if read_only_addresses.is_empty() {
                                    (None, None)
                                } else {
                                    (
                                        Some(read_only_addresses.as_slice()),
                                        Some(vec![address_tree; read_only_addresses.len()]),
                                    )
                                };
                            let addresses_with_tree = match (new_addresses, address_tree_pubkey) {
                                (Some(addresses), Some(trees)) => addresses
                                    .iter()
                                    .zip(trees.iter())
                                    .map(|(address, tree)| AddressWithTree {
                                        address: *address,
                                        tree: *tree,
                                    })
                                    .collect::<Vec<_>>(),
                                _ => vec![],
                            };

                            rpc.get_validity_proof(
                                input_hashes.unwrap_or_default(),
                                addresses_with_tree,
                                None,
                            )
                            .await
                            .unwrap()
                            .value
                        };
                        let readonly_addresses = proof_res
                            .get_address_root_indices()
                            .iter()
                            .zip(read_only_addresses)
                            .map(|(root_index, address)| ReadOnlyAddress {
                                address_merkle_tree_pubkey: address_tree.into(),
                                address,
                                address_merkle_tree_root_index: *root_index,
                            })
                            .collect::<Vec<_>>();
                        if !batched {
                            proof_res.get_root_indices().iter().enumerate().for_each(
                                |(i, root_index)| {
                                    account_infos[i].input.as_mut().unwrap().root_index =
                                        root_index.unwrap_or_default();
                                },
                            );
                        }
                        local_sdk::perform_test_transaction(
                            &mut rpc,
                            &mut test_indexer,
                            &payer,
                            Vec::new(),
                            Vec::new(),
                            vec![],
                            proof_res.proof.0,
                            None,
                            Some(account_infos),
                            is_v2_ix,
                            true,
                            read_only_accounts,
                            readonly_addresses,
                            queue,
                            tree,
                            false,
                            None,
                            false,
                            None,
                            None,
                        )
                        .await
                        .unwrap();
                    }
                }
            }
        }
    }
}

/// Test with account infos instruction with different input combinations:
/// - anchor compat accounts
/// - small accounts
/// - with V1 and V2 trees
/// readonly_accounts  0..4; skipped for V1 trees
/// readonly_addresses = 0..2; skipped for V1 trees
/// input_accounts = 0..4;
/// output_accounts = 0..4;
#[serial]
#[tokio::test]
async fn functional_account_infos() {
    spawn_prover().await;
    for (batched, is_v2_ix) in
        [(true, false), (true, true), (false, false), (false, true)].into_iter()
    {
        let config = if batched {
            let mut config = ProgramTestConfig::default_with_batched_trees(false);
            config.with_prover = false;
            config.v2_state_tree_config = Some(InitStateTreeAccountsInstructionData::default());
            config.additional_programs = Some(vec![(
                "create_address_test_program",
                create_address_test_program::ID,
            )]);
            config
        } else {
            ProgramTestConfig::new(
                false,
                Some(vec![(
                    "create_address_test_program",
                    create_address_test_program::ID,
                )]),
            )
        };
        let mut rpc = LightProgramTest::new(config).await.unwrap();
        let env = rpc.test_accounts.clone();
        let queue = if batched {
            env.v2_state_trees[0].output_queue
        } else {
            env.v1_state_trees[0].nullifier_queue
        };
        let tree = if batched {
            env.v2_state_trees[0].merkle_tree
        } else {
            env.v1_state_trees[0].merkle_tree
        };
        let address_tree = if batched {
            env.v2_address_trees[0]
        } else {
            env.v1_address_trees[0].merkle_tree
        };

        let payer = rpc.get_payer().insecure_clone();
        let mut test_indexer = TestIndexer::init_from_acounts(&payer, &env, 0).await;
        // Create a bunch of outputs that we can use as inputs.
        for _ in 0..5 {
            let output_accounts = vec![
                get_compressed_output_account(
                    true,
                    if batched {
                        env.v2_state_trees[0].output_queue
                    } else {
                        env.v1_state_trees[0].merkle_tree
                    }
                );
                30
            ];
            local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                output_accounts,
                vec![],
                None,
                None,
                None,
                is_v2_ix,
                true,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await
            .unwrap()
            .unwrap();
        }
        let mut rng = thread_rng();

        let mut input_merkle_tree_index = 4;
        let max_readonly_accounts = 4;
        let max_readonly_addresses = 2;
        let max_input_accounts = 4;
        let max_output_accounts = 4;
        // With v1 trees -> proof by zkp
        for num_outputs in 1..=max_output_accounts {
            for num_inputs in 0..=max_input_accounts {
                for mut num_read_only_accounts in 0..max_readonly_accounts {
                    for mut num_read_only_addresses in 0..max_readonly_addresses {
                        if !batched {
                            num_read_only_addresses = 0;
                            num_read_only_accounts = 0;
                        }
                        println!("num_outputs: {}, num_inputs: {}, num_read_only_accounts: {}, num_read_only_addresses: {}", num_outputs, num_inputs, num_read_only_accounts, num_read_only_addresses);
                        let input_accounts = (input_merkle_tree_index
                            ..num_inputs + input_merkle_tree_index)
                            .map(|i| {
                                println!("input leaf index: {}", i);
                                get_input_account_info(PackedMerkleContext {
                                    leaf_index: i,
                                    merkle_tree_pubkey_index: 1,
                                    queue_pubkey_index: 0,
                                    prove_by_index: batched,
                                })
                            })
                            .collect::<Vec<_>>();
                        input_merkle_tree_index += num_inputs;
                        let output_accounts = (0..num_outputs)
                            .map(|_| get_output_account_info(if batched { 0 } else { 1 }))
                            .collect::<Vec<_>>();
                        let mut account_infos = Vec::new();
                        for i in 0..num_inputs {
                            let output = if num_outputs > i {
                                Some(output_accounts[i as usize].clone())
                            } else {
                                None
                            };

                            account_infos.push(CompressedAccountInfo {
                                address: None,
                                input: Some(input_accounts[i as usize].clone()),
                                output,
                            });
                        }
                        for i in num_inputs..num_outputs {
                            account_infos.push(CompressedAccountInfo {
                                address: None,
                                input: None,
                                output: Some(output_accounts[i as usize].clone()),
                            });
                        }
                        let read_only_accounts = (1..=num_read_only_accounts)
                            .map(|i| {
                                println!("read only leaf index: {}", i);
                                let accounts = test_indexer
                                    .get_compressed_accounts_with_merkle_context_by_owner(
                                        &create_address_test_program::ID,
                                    );
                                let account = accounts
                                    .get(accounts.len().saturating_sub(i as usize))
                                    .unwrap();
                                println!("leaf index {}", account.merkle_context.leaf_index);
                                let mut merkle_context = account.merkle_context;
                                if batched {
                                    merkle_context.prove_by_index = true;
                                }
                                ReadOnlyCompressedAccount {
                                    merkle_context,
                                    account_hash: account.hash().unwrap(),
                                    root_index: 0,
                                }
                            })
                            .collect::<Vec<_>>();
                        let read_only_addresses = (0..num_read_only_addresses)
                            .map(|_| {
                                let mut address = rng.gen::<[u8; 32]>();
                                address[0] = 0;
                                address
                            })
                            .collect::<Vec<_>>();
                        let proof_res: Response<ValidityProofWithContext> =
                            if read_only_addresses.is_empty() && num_inputs == 0 {
                                Response::<ValidityProofWithContext> {
                                    context: Context { slot: 0 },
                                    value: ValidityProofWithContext {
                                        proof: ValidityProof::default(),
                                        accounts: vec![],
                                        addresses: vec![],
                                    },
                                }
                            } else {
                                let input_hashes = if num_inputs == 0 {
                                    None
                                } else {
                                    let hashes: Vec<[u8; 32]> = if batched {
                                        input_accounts
                                            .iter()
                                            .map(|account| {
                                                test_indexer
                                                    .state_merkle_trees
                                                    .iter()
                                                    .find(|x| x.accounts.merkle_tree == tree)
                                                    .unwrap()
                                                    .output_queue_elements
                                                    [account.merkle_context.leaf_index as usize]
                                                    .0
                                            })
                                            .collect::<Vec<_>>()
                                    } else {
                                        input_accounts
                                            .iter()
                                            .map(|account| {
                                                test_indexer
                                                    .state_merkle_trees
                                                    .iter()
                                                    .find(|x| x.accounts.merkle_tree == tree)
                                                    .unwrap()
                                                    .merkle_tree
                                                    .get_leaf(
                                                        account.merkle_context.leaf_index as usize,
                                                    )
                                                    .unwrap()
                                            })
                                            .collect::<Vec<_>>()
                                    };
                                    Some(hashes)
                                };
                                let (new_addresses, address_tree_pubkey) =
                                    if read_only_addresses.is_empty() {
                                        (None, None)
                                    } else {
                                        (
                                            Some(read_only_addresses.as_slice()),
                                            Some(vec![address_tree; read_only_addresses.len()]),
                                        )
                                    };
                                let addresses_with_tree = match (new_addresses, address_tree_pubkey)
                                {
                                    (Some(addresses), Some(trees)) => addresses
                                        .iter()
                                        .zip(trees.iter())
                                        .map(|(address, tree)| AddressWithTree {
                                            address: *address,
                                            tree: *tree,
                                        })
                                        .collect::<Vec<_>>(),
                                    _ => vec![],
                                };

                                rpc.get_validity_proof(
                                    input_hashes.unwrap_or_default(),
                                    addresses_with_tree,
                                    None,
                                )
                                .await
                                .unwrap()
                            };
                        let readonly_addresses = proof_res
                            .value
                            .get_address_root_indices()
                            .iter()
                            .zip(read_only_addresses)
                            .map(|(root_index, address)| ReadOnlyAddress {
                                address_merkle_tree_pubkey: address_tree.into(),
                                address,
                                address_merkle_tree_root_index: *root_index,
                            })
                            .collect::<Vec<_>>();
                        if !batched {
                            proof_res
                                .value
                                .get_root_indices()
                                .iter()
                                .enumerate()
                                .for_each(|(i, root_index)| {
                                    account_infos[i].input.as_mut().unwrap().root_index =
                                        root_index.unwrap_or_default();
                                });
                        }
                        local_sdk::perform_test_transaction(
                            &mut rpc,
                            &mut test_indexer,
                            &payer,
                            Vec::new(),
                            Vec::new(),
                            vec![],
                            proof_res.value.proof.0,
                            None,
                            Some(account_infos),
                            is_v2_ix,
                            true,
                            read_only_accounts,
                            readonly_addresses,
                            queue,
                            tree,
                            false,
                            None,
                            false,
                            None,
                            None,
                        )
                        .await
                        .unwrap();
                    }
                }
            }
        }
    }
}

/// Test with account info instruction with creating addresses.
/// Addresses are either assigned or unassigned.
/// To use an address in an account the account has to specify the address in the account info.
///
/// Failing Tests:
/// 1. Use unassigned new address in account info
/// 2. Create new address assigned to a non-existent account
/// 3. Address assigned to account. The accounts address is None.
/// 4. Create address assigned to account with different address.
/// 5. Create two addresses assigned to same account.
///
/// Functional tests:
/// 6. creates two assigned addresses.
/// 7. create two unassigned addresses.
/// 8. create one unassigned address.
/// 9. create two addresses 1 assigned one not assigned.
/// 10. create one assigned address.
#[serial]
#[tokio::test]
async fn create_addresses_with_account_info() {
    spawn_prover().await;
    let with_transaction_hash = true;
    for (batched, is_v2_ix) in
        [(true, false), (true, true), (false, false), (false, true)].into_iter()
    {
        let config = if batched {
            let mut config = ProgramTestConfig::default_with_batched_trees(false);
            config.with_prover = false;
            config.additional_programs = Some(vec![(
                "create_address_test_program",
                create_address_test_program::ID,
            )]);
            config.v2_state_tree_config = Some(InitStateTreeAccountsInstructionData::default());
            config
        } else {
            ProgramTestConfig::new(
                false,
                Some(vec![(
                    "create_address_test_program",
                    create_address_test_program::ID,
                )]),
            )
        };
        let mut rpc = LightProgramTest::new(config).await.unwrap();
        let env = rpc.test_accounts.clone();

        let queue = if batched {
            env.v2_state_trees[0].output_queue
        } else {
            env.v1_state_trees[0].nullifier_queue
        };
        let tree = if batched {
            env.v2_state_trees[0].merkle_tree
        } else {
            env.v1_state_trees[0].merkle_tree
        };
        let address_tree = if batched {
            env.v2_address_trees[0]
        } else {
            env.v1_address_trees[0].merkle_tree
        };
        let address_queue = if batched {
            env.v2_address_trees[0]
        } else {
            env.v1_address_trees[0].queue
        };
        let payer = rpc.get_payer().insecure_clone();
        let mut test_indexer = TestIndexer::init_from_acounts(&payer, &env, 0).await;

        let output_accounts = (0..2)
            .map(|_| get_output_account_info(if batched { 0 } else { 1 }))
            .collect::<Vec<_>>();
        let seed = [1u8; 32];
        let address = if batched {
            derive_address(
                &seed,
                &address_tree.to_bytes(),
                &create_address_test_program::ID.to_bytes(),
            )
        } else {
            derive_address_legacy(&address_tree.into(), &seed).unwrap()
        };

        let seed1 = [2u8; 32];
        let address1 = if batched {
            derive_address(
                &seed1,
                &address_tree.to_bytes(),
                &create_address_test_program::ID.to_bytes(),
            )
        } else {
            derive_address_legacy(&address_tree.into(), &seed1).unwrap()
        };
        let account_info = CompressedAccountInfo {
            address: Some(address),
            input: None,
            output: Some(output_accounts[0].clone()),
        };

        let account_info1 = CompressedAccountInfo {
            address: Some(address1),
            input: None,
            output: Some(output_accounts[1].clone()),
        };

        let addresses_with_tree = vec![
            AddressWithTree {
                address,
                tree: address_tree,
            },
            AddressWithTree {
                address: address1,
                tree: address_tree,
            },
        ];

        let rpc_result = rpc
            .get_validity_proof(Vec::new(), addresses_with_tree, None)
            .await
            .unwrap();
        let new_address_params = NewAddressParamsAssigned {
            seed,
            address_queue_pubkey: address_queue.into(),
            address_merkle_tree_pubkey: address_tree.into(),
            address_merkle_tree_root_index: rpc_result.value.get_address_root_indices()[0],
            assigned_account_index: Some(0),
        };
        let new_address_params1 = NewAddressParamsAssigned {
            seed: seed1,
            address_queue_pubkey: address_queue.into(),
            address_merkle_tree_pubkey: address_tree.into(),
            address_merkle_tree_root_index: rpc_result.value.get_address_root_indices()[1],
            assigned_account_index: Some(1),
        };
        // 1. Create unassigned address and use it in account_info.
        {
            let mut new_address_params = new_address_params.clone();
            new_address_params.assigned_account_index = None;
            let result = local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                vec![],
                vec![new_address_params],
                rpc_result.value.proof.0,
                None,
                Some(vec![account_info.clone()]),
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await;
            assert_rpc_error(result, 0, SystemProgramError::InvalidAddress.into()).unwrap();
        }
        // 2. Create address assigned to a non-existent account.
        {
            let result = local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                vec![],
                vec![new_address_params.clone()],
                rpc_result.value.proof.0,
                None,
                Some(vec![]),
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await;
            assert_rpc_error(
                result,
                0,
                SystemProgramError::NewAddressAssignedIndexOutOfBounds.into(),
            )
            .unwrap();
        }
        // 3. Address assigned to account. The accounts address is None.
        {
            let new_address_params = new_address_params.clone();
            let mut account_info = account_info.clone();
            account_info.address = None;
            let result = local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                vec![],
                vec![new_address_params],
                rpc_result.value.proof.0,
                None,
                Some(vec![account_info]),
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await;
            assert_rpc_error(result, 0, SystemProgramError::AddressIsNone.into()).unwrap();
        }
        // 4. Create address assigned to account with different address.
        {
            let result = local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                vec![],
                vec![new_address_params.clone()],
                rpc_result.value.proof.0,
                None,
                Some(vec![account_info1.clone()]),
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await;
            assert_rpc_error(result, 0, SystemProgramError::AddressDoesNotMatch.into()).unwrap();
        }
        // 5. Create two addresses assigned to same account.
        {
            let mut new_address_params1 = new_address_params1.clone();
            new_address_params1.assigned_account_index = Some(0);
            let result = local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                vec![],
                vec![new_address_params.clone(), new_address_params1],
                rpc_result.value.proof.0,
                None,
                Some(vec![account_info.clone()]),
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await;
            assert_rpc_error(result, 0, SystemProgramError::AddressDoesNotMatch.into()).unwrap();
        }
        // 6. Functional create two addresses.
        {
            local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                vec![],
                vec![new_address_params.clone(), new_address_params1.clone()],
                rpc_result.value.proof.0,
                None,
                Some(vec![account_info.clone(), account_info1.clone()]),
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await
            .unwrap();
        }
        // 7. Create two unassigned addresses.
        {
            let seed = [3u8; 32];
            let address = if batched {
                derive_address(
                    &seed,
                    &address_tree.to_bytes(),
                    &create_address_test_program::ID.to_bytes(),
                )
            } else {
                derive_address_legacy(&address_tree.into(), &seed).unwrap()
            };

            let seed1 = [4u8; 32];
            let address1 = if batched {
                derive_address(
                    &seed1,
                    &address_tree.to_bytes(),
                    &create_address_test_program::ID.to_bytes(),
                )
            } else {
                derive_address_legacy(&address_tree.into(), &seed1).unwrap()
            };
            let rpc_result = rpc
                .get_validity_proof(
                    Vec::new(),
                    vec![
                        AddressWithTree {
                            address,
                            tree: address_tree,
                        },
                        AddressWithTree {
                            address: address1,
                            tree: address_tree,
                        },
                    ],
                    None,
                )
                .await
                .unwrap();
            let new_address_params = NewAddressParamsAssigned {
                seed,
                address_queue_pubkey: address_queue.into(),
                address_merkle_tree_pubkey: address_tree.into(),
                address_merkle_tree_root_index: rpc_result.value.get_address_root_indices()[0],
                assigned_account_index: None,
            };
            let new_address_params1 = NewAddressParamsAssigned {
                seed: seed1,
                address_queue_pubkey: address_queue.into(),
                address_merkle_tree_pubkey: address_tree.into(),
                address_merkle_tree_root_index: rpc_result.value.get_address_root_indices()[1],
                assigned_account_index: None,
            };
            local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                vec![],
                vec![new_address_params.clone(), new_address_params1.clone()],
                rpc_result.value.proof.0,
                None,
                Some(vec![]),
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await
            .unwrap();
        }
        // 8. Create one unassigned address.
        {
            let seed = [5u8; 32];
            let address = if batched {
                derive_address(
                    &seed,
                    &address_tree.to_bytes(),
                    &create_address_test_program::ID.to_bytes(),
                )
            } else {
                derive_address_legacy(&address_tree.into(), &seed).unwrap()
            };

            let rpc_result = rpc
                .get_validity_proof(
                    Vec::new(),
                    vec![AddressWithTree {
                        address,
                        tree: address_tree,
                    }],
                    None,
                )
                .await
                .unwrap();
            let new_address_params = NewAddressParamsAssigned {
                seed,
                address_queue_pubkey: address_queue.into(),
                address_merkle_tree_pubkey: address_tree.into(),
                address_merkle_tree_root_index: rpc_result.value.get_address_root_indices()[0],
                assigned_account_index: None,
            };
            local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                vec![],
                vec![new_address_params.clone()],
                rpc_result.value.proof.0,
                None,
                Some(vec![]),
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await
            .unwrap();
        }
        // 9. Create two addresses one unassigned address one assigned address.
        {
            let output_accounts = (0..1)
                .map(|_| get_output_account_info(if batched { 0 } else { 1 }))
                .collect::<Vec<_>>();
            let seed = [6u8; 32];
            let address = if batched {
                derive_address(
                    &seed,
                    &address_tree.to_bytes(),
                    &create_address_test_program::ID.to_bytes(),
                )
            } else {
                derive_address_legacy(&address_tree.into(), &seed).unwrap()
            };

            let seed1 = [7u8; 32];
            let address1 = if batched {
                derive_address(
                    &seed1,
                    &address_tree.to_bytes(),
                    &create_address_test_program::ID.to_bytes(),
                )
            } else {
                derive_address_legacy(&address_tree.into(), &seed1).unwrap()
            };
            let account_info = CompressedAccountInfo {
                address: Some(address1),
                input: None,
                output: Some(output_accounts[0].clone()),
            };

            let rpc_result = rpc
                .get_validity_proof(
                    Vec::new(),
                    vec![
                        AddressWithTree {
                            address,
                            tree: address_tree,
                        },
                        AddressWithTree {
                            address: address1,
                            tree: address_tree,
                        },
                    ],
                    None,
                )
                .await
                .unwrap();
            let new_address_params = NewAddressParamsAssigned {
                seed,
                address_queue_pubkey: address_queue.into(),
                address_merkle_tree_pubkey: address_tree.into(),
                address_merkle_tree_root_index: rpc_result.value.get_address_root_indices()[0],
                assigned_account_index: None,
            };
            let new_address_params1 = NewAddressParamsAssigned {
                seed: seed1,
                address_queue_pubkey: address_queue.into(),
                address_merkle_tree_pubkey: address_tree.into(),
                address_merkle_tree_root_index: rpc_result.value.get_address_root_indices()[1],
                assigned_account_index: Some(0),
            };
            local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                vec![],
                vec![new_address_params, new_address_params1],
                rpc_result.value.proof.0,
                None,
                Some(vec![account_info]),
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await
            .unwrap();
        }
        // 10. Create one assigned address.
        {
            let output_accounts = (0..1)
                .map(|_| get_output_account_info(if batched { 0 } else { 1 }))
                .collect::<Vec<_>>();
            let seed = [8u8; 32];
            let address = if batched {
                derive_address(
                    &seed,
                    &address_tree.to_bytes(),
                    &create_address_test_program::ID.to_bytes(),
                )
            } else {
                derive_address_legacy(&address_tree.into(), &seed).unwrap()
            };

            let account_info = CompressedAccountInfo {
                address: Some(address),
                input: None,
                output: Some(output_accounts[0].clone()),
            };

            let rpc_result = rpc
                .get_validity_proof(
                    Vec::new(),
                    vec![AddressWithTree {
                        address,
                        tree: address_tree,
                    }],
                    None,
                )
                .await
                .unwrap();
            let new_address_params = NewAddressParamsAssigned {
                seed,
                address_queue_pubkey: address_queue.into(),
                address_merkle_tree_pubkey: address_tree.into(),
                address_merkle_tree_root_index: rpc_result.value.get_address_root_indices()[0],
                assigned_account_index: Some(0),
            };

            local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                vec![],
                vec![new_address_params],
                rpc_result.value.proof.0,
                None,
                Some(vec![account_info]),
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await
            .unwrap();
        }
    }
}

/// Test with account info instruction with creating addresses.
/// Addresses are either assigned or unassigned.
/// To use an address in an account the account has to specify the address in the account info.
///
/// Failing Tests:
/// 1. Use unassigned new address in account info
/// 2. Create new address assigned to a non-existent account
/// 3. Address assigned to account. The accounts address is None.
/// 4. Create address assigned to account with different address.
/// 5. Create two addresses assigned to same account.
///
/// Functional tests:
/// 6. creates two assigned addresses.
/// 7. create two unassigned addresses.
/// 8. create one unassigned address.
/// 9. create two addresses 1 assigned one not assigned.
/// 10. create one assigned address.
#[serial]
#[tokio::test]
async fn create_addresses_with_read_only() {
    spawn_prover().await;
    let with_transaction_hash = true;
    for (batched, is_v2_ix) in
        [(true, false), (true, true), (false, false), (false, true)].into_iter()
    {
        println!("batched {}, v2 ix {}", batched, is_v2_ix);
        let config = if batched {
            let mut config = ProgramTestConfig::default_with_batched_trees(false);
            config.with_prover = false;
            config.additional_programs = Some(vec![(
                "create_address_test_program",
                create_address_test_program::ID,
            )]);
            config.v2_state_tree_config = Some(InitStateTreeAccountsInstructionData::default());
            config
        } else {
            ProgramTestConfig::new(
                false,
                Some(vec![(
                    "create_address_test_program",
                    create_address_test_program::ID,
                )]),
            )
        };
        let mut rpc = LightProgramTest::new(config).await.unwrap();
        let env = rpc.test_accounts.clone();
        let queue = if batched {
            env.v2_state_trees[0].output_queue
        } else {
            env.v1_state_trees[0].nullifier_queue
        };
        let tree = if batched {
            env.v2_state_trees[0].merkle_tree
        } else {
            env.v1_state_trees[0].merkle_tree
        };
        let address_tree = if batched {
            env.v2_address_trees[0]
        } else {
            env.v1_address_trees[0].merkle_tree
        };
        let address_queue = if batched {
            env.v2_address_trees[0]
        } else {
            env.v1_address_trees[0].queue
        };
        let payer = rpc.get_payer().insecure_clone();
        let mut test_indexer = TestIndexer::init_from_acounts(&payer, &env, 0).await;

        let seed = [1u8; 32];
        let address = if batched {
            derive_address(
                &seed,
                &address_tree.to_bytes(),
                &create_address_test_program::ID.to_bytes(),
            )
        } else {
            derive_address_legacy(&address_tree.into(), &seed).unwrap()
        };

        let seed1 = [2u8; 32];
        let address1 = if batched {
            derive_address(
                &seed1,
                &address_tree.to_bytes(),
                &create_address_test_program::ID.to_bytes(),
            )
        } else {
            derive_address_legacy(&address_tree.into(), &seed1).unwrap()
        };
        let mut output_1 = get_compressed_output_account(true, if batched { queue } else { tree });
        output_1.compressed_account.address = Some(address);

        let mut output_2 = get_compressed_output_account(true, if batched { queue } else { tree });
        output_2.compressed_account.address = Some(address1);

        let addresses_with_tree = vec![
            AddressWithTree {
                address,
                tree: address_tree,
            },
            AddressWithTree {
                address: address1,
                tree: address_tree,
            },
        ];

        let rpc_result = rpc
            .get_validity_proof(Vec::new(), addresses_with_tree, None)
            .await
            .unwrap();
        let new_address_params = NewAddressParamsAssigned {
            seed,
            address_queue_pubkey: address_queue.into(),
            address_merkle_tree_pubkey: address_tree.into(),
            address_merkle_tree_root_index: rpc_result.value.get_address_root_indices()[0],
            assigned_account_index: Some(0),
        };
        let new_address_params1 = NewAddressParamsAssigned {
            seed: seed1,
            address_queue_pubkey: address_queue.into(),
            address_merkle_tree_pubkey: address_tree.into(),
            address_merkle_tree_root_index: rpc_result.value.get_address_root_indices()[1],
            assigned_account_index: Some(1),
        };
        // 1. Create unassigned address and use it in account_info.
        {
            println!("1");
            let mut new_address_params = new_address_params.clone();
            new_address_params.assigned_account_index = None;
            let result = local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                vec![output_1.clone()],
                vec![new_address_params],
                rpc_result.value.proof.0,
                None,
                None,
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await;
            assert_rpc_error(result, 0, SystemProgramError::InvalidAddress.into()).unwrap();
        }
        // 2. Create address assigned to a non-existent account.
        {
            println!("2");
            let result = local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                vec![],
                vec![new_address_params.clone()],
                rpc_result.value.proof.0,
                None,
                None,
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await;
            assert_rpc_error(
                result,
                0,
                SystemProgramError::NewAddressAssignedIndexOutOfBounds.into(),
            )
            .unwrap();
        }
        // 3. Address assigned to account. The accounts address is None.
        {
            println!("3");
            let new_address_params = new_address_params.clone();
            let mut output_1 = output_1.clone();
            output_1.compressed_account.address = None;
            let result = local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                vec![output_1],
                vec![new_address_params],
                rpc_result.value.proof.0,
                None,
                None,
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await;
            assert_rpc_error(result, 0, SystemProgramError::AddressIsNone.into()).unwrap();
        }
        // 4. Create address assigned to account with different address.
        {
            println!("4");
            let result = local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                vec![output_2.clone()],
                vec![new_address_params.clone()],
                rpc_result.value.proof.0,
                None,
                None,
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await;
            assert_rpc_error(result, 0, SystemProgramError::AddressDoesNotMatch.into()).unwrap();
        }
        // 5. Create two addresses assigned to same account.
        {
            println!("5");
            let mut new_address_params1 = new_address_params1.clone();
            new_address_params1.assigned_account_index = Some(0);
            let result = local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                vec![output_1.clone()],
                vec![new_address_params.clone(), new_address_params1],
                rpc_result.value.proof.0,
                None,
                None,
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await;
            assert_rpc_error(result, 0, SystemProgramError::AddressDoesNotMatch.into()).unwrap();
        }
        // 6. Functional create two addresses.
        {
            println!("6");
            local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                vec![output_1, output_2],
                vec![new_address_params.clone(), new_address_params1.clone()],
                rpc_result.value.proof.0,
                None,
                None,
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await
            .unwrap();
        }
        // 7. Create two unassigned addresses.
        {
            println!("7");
            let seed = [3u8; 32];
            let address = if batched {
                derive_address(
                    &seed,
                    &address_tree.to_bytes(),
                    &create_address_test_program::ID.to_bytes(),
                )
            } else {
                derive_address_legacy(&address_tree.into(), &seed).unwrap()
            };

            let seed1 = [4u8; 32];
            let address1 = if batched {
                derive_address(
                    &seed1,
                    &address_tree.to_bytes(),
                    &create_address_test_program::ID.to_bytes(),
                )
            } else {
                derive_address_legacy(&address_tree.into(), &seed1).unwrap()
            };
            let rpc_result = rpc
                .get_validity_proof(
                    Vec::new(),
                    vec![
                        AddressWithTree {
                            address,
                            tree: address_tree,
                        },
                        AddressWithTree {
                            address: address1,
                            tree: address_tree,
                        },
                    ],
                    None,
                )
                .await
                .unwrap();
            let new_address_params = NewAddressParamsAssigned {
                seed,
                address_queue_pubkey: address_queue.into(),
                address_merkle_tree_pubkey: address_tree.into(),
                address_merkle_tree_root_index: rpc_result.value.get_address_root_indices()[0],
                assigned_account_index: None,
            };
            let new_address_params1 = NewAddressParamsAssigned {
                seed: seed1,
                address_queue_pubkey: address_queue.into(),
                address_merkle_tree_pubkey: address_tree.into(),
                address_merkle_tree_root_index: rpc_result.value.get_address_root_indices()[1],
                assigned_account_index: None,
            };
            local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                vec![],
                vec![new_address_params.clone(), new_address_params1.clone()],
                rpc_result.value.proof.0,
                None,
                None,
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await
            .unwrap();
        }
        // 8. Create one unassigned address.
        {
            println!("8");
            let seed = [5u8; 32];
            let address = if batched {
                derive_address(
                    &seed,
                    &address_tree.to_bytes(),
                    &create_address_test_program::ID.to_bytes(),
                )
            } else {
                derive_address_legacy(&address_tree.into(), &seed).unwrap()
            };

            let rpc_result = rpc
                .get_validity_proof(
                    Vec::new(),
                    vec![AddressWithTree {
                        address,
                        tree: address_tree,
                    }],
                    None,
                )
                .await
                .unwrap();
            let new_address_params = NewAddressParamsAssigned {
                seed,
                address_queue_pubkey: address_queue.into(),
                address_merkle_tree_pubkey: address_tree.into(),
                address_merkle_tree_root_index: rpc_result.value.get_address_root_indices()[0],
                assigned_account_index: None,
            };
            local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                vec![],
                vec![new_address_params.clone()],
                rpc_result.value.proof.0,
                None,
                None,
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await
            .unwrap();
        }
        // 9. Create two addresses one unassigned address one assigned address.
        {
            println!("8");
            let mut output_accounts = (0..1)
                .map(|_| get_compressed_output_account(true, if batched { queue } else { tree }))
                .collect::<Vec<_>>();
            let seed = [6u8; 32];
            let address = if batched {
                derive_address(
                    &seed,
                    &address_tree.to_bytes(),
                    &create_address_test_program::ID.to_bytes(),
                )
            } else {
                derive_address_legacy(&address_tree.into(), &seed).unwrap()
            };

            let seed1 = [7u8; 32];
            let address1 = if batched {
                derive_address(
                    &seed1,
                    &address_tree.to_bytes(),
                    &create_address_test_program::ID.to_bytes(),
                )
            } else {
                derive_address_legacy(&address_tree.into(), &seed1).unwrap()
            };
            output_accounts[0].compressed_account.address = Some(address1);

            let rpc_result = rpc
                .get_validity_proof(
                    Vec::new(),
                    vec![
                        AddressWithTree {
                            address,
                            tree: address_tree,
                        },
                        AddressWithTree {
                            address: address1,
                            tree: address_tree,
                        },
                    ],
                    None,
                )
                .await
                .unwrap();
            let new_address_params = NewAddressParamsAssigned {
                seed,
                address_queue_pubkey: address_queue.into(),
                address_merkle_tree_pubkey: address_tree.into(),
                address_merkle_tree_root_index: rpc_result.value.get_address_root_indices()[0],
                assigned_account_index: None,
            };
            let new_address_params1 = NewAddressParamsAssigned {
                seed: seed1,
                address_queue_pubkey: address_queue.into(),
                address_merkle_tree_pubkey: address_tree.into(),
                address_merkle_tree_root_index: rpc_result.value.get_address_root_indices()[1],
                assigned_account_index: Some(0),
            };
            local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                output_accounts,
                vec![new_address_params, new_address_params1],
                rpc_result.value.proof.0,
                None,
                None,
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await
            .unwrap();
        }
        // 10. Create one assigned address.
        {
            println!("10");
            let mut output_accounts = (0..1)
                .map(|_| get_compressed_output_account(true, if batched { queue } else { tree }))
                .collect::<Vec<_>>();
            let seed = [8u8; 32];
            let address = if batched {
                derive_address(
                    &seed,
                    &address_tree.to_bytes(),
                    &create_address_test_program::ID.to_bytes(),
                )
            } else {
                derive_address_legacy(&address_tree.into(), &seed).unwrap()
            };

            output_accounts[0].compressed_account.address = Some(address);

            let rpc_result = rpc
                .get_validity_proof(
                    Vec::new(),
                    vec![AddressWithTree {
                        address,
                        tree: address_tree,
                    }],
                    None,
                )
                .await
                .unwrap();
            let new_address_params = NewAddressParamsAssigned {
                seed,
                address_queue_pubkey: address_queue.into(),
                address_merkle_tree_pubkey: address_tree.into(),
                address_merkle_tree_root_index: rpc_result.value.get_address_root_indices()[0],
                assigned_account_index: Some(0),
            };

            local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                output_accounts,
                vec![new_address_params],
                rpc_result.value.proof.0,
                None,
                None,
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await
            .unwrap();
        }
    }
}

#[tokio::test]
async fn compress_sol_with_account_info() {
    let with_transaction_hash = false;
    let batched = true;
    for is_v2_ix in [true, false].into_iter() {
        let config = {
            let mut config = ProgramTestConfig::default_with_batched_trees(false);
            config.with_prover = false;
            config.additional_programs = Some(vec![(
                "create_address_test_program",
                create_address_test_program::ID,
            )]);

            config.v2_state_tree_config = Some(InitStateTreeAccountsInstructionData::default());
            config.v2_address_tree_config =
                Some(InitAddressTreeAccountsInstructionData::test_default());
            config
        };
        let mut rpc = LightProgramTest::new(config)
            .await
            .expect("Failed to setup test programs with accounts");
        let env = rpc.test_accounts.clone();
        let queue = if batched {
            env.v2_state_trees[0].output_queue
        } else {
            env.v1_state_trees[0].nullifier_queue
        };
        let tree = if batched {
            env.v2_state_trees[0].merkle_tree
        } else {
            env.v1_state_trees[0].merkle_tree
        };

        let payer = rpc.get_payer().insecure_clone();
        let mut test_indexer = TestIndexer::init_from_acounts(&payer, &env, 0).await;

        // 1.Compress sol
        {
            let mut output_account = get_output_account_info(0);
            let compression_lamports = 1_000_000;
            output_account.lamports = compression_lamports;
            let account_info = CompressedAccountInfo {
                address: None,
                input: None,
                output: Some(output_account),
            };
            // 1. Failing compress sol, invalid pool derivation.
            {
                let result = local_sdk::perform_test_transaction(
                    &mut rpc,
                    &mut test_indexer,
                    &payer,
                    vec![],
                    vec![],
                    vec![],
                    None,
                    None,
                    Some(vec![account_info.clone()]),
                    is_v2_ix,
                    with_transaction_hash,
                    Vec::new(),
                    Vec::new(),
                    queue,
                    tree,
                    true,
                    Some(compression_lamports),
                    true,
                    None,
                    None,
                )
                .await;
                assert_rpc_error(result, 0, AccountError::InvalidSeeds.into()).unwrap();
            }
            // 2. Functional compress sol
            {
                local_sdk::perform_test_transaction(
                    &mut rpc,
                    &mut test_indexer,
                    &payer,
                    vec![],
                    vec![],
                    vec![],
                    None,
                    None,
                    Some(vec![account_info.clone()]),
                    is_v2_ix,
                    with_transaction_hash,
                    Vec::new(),
                    Vec::new(),
                    queue,
                    tree,
                    true,
                    Some(compression_lamports),
                    false,
                    None,
                    None,
                )
                .await
                .unwrap();
                let output_account_balance = test_indexer
                    .get_compressed_accounts_with_merkle_context_by_owner(
                        &create_address_test_program::ID,
                    )[0]
                .compressed_account
                .lamports;
                assert_eq!(output_account_balance, compression_lamports);
            }
        }
        // 2.Decompress sol
        {
            let mut input_account = get_input_account_info(PackedMerkleContext {
                merkle_tree_pubkey_index: 1,
                queue_pubkey_index: 0,
                leaf_index: 0,
                prove_by_index: true,
            });
            let compression_lamports = 1_000_000;
            input_account.lamports = compression_lamports;
            let account_info = CompressedAccountInfo {
                address: None,
                input: Some(input_account),
                output: None,
            };
            let recipient = Pubkey::new_unique();
            // 3. Failing decompress sol, invalid pool derivation.
            {
                let result = local_sdk::perform_test_transaction(
                    &mut rpc,
                    &mut test_indexer,
                    &payer,
                    vec![],
                    vec![],
                    vec![],
                    None,
                    Some(recipient),
                    Some(vec![account_info.clone()]),
                    is_v2_ix,
                    with_transaction_hash,
                    Vec::new(),
                    Vec::new(),
                    queue,
                    tree,
                    false,
                    Some(compression_lamports),
                    true,
                    None,
                    None,
                )
                .await;
                assert_rpc_error(result, 0, AccountError::InvalidSeeds.into()).unwrap();
            }
            // 4. Functional decompress sol
            {
                local_sdk::perform_test_transaction(
                    &mut rpc,
                    &mut test_indexer,
                    &payer,
                    vec![],
                    vec![],
                    vec![],
                    None,
                    Some(recipient),
                    Some(vec![account_info.clone()]),
                    is_v2_ix,
                    with_transaction_hash,
                    Vec::new(),
                    Vec::new(),
                    queue,
                    tree,
                    false,
                    Some(compression_lamports),
                    false,
                    None,
                    None,
                )
                .await
                .unwrap();
                let recipient_balance = rpc.get_balance(&recipient).await.unwrap();
                assert_eq!(recipient_balance, compression_lamports);
            }
        }
    }
}

#[serial]
#[tokio::test]
async fn cpi_context_with_read_only() {
    spawn_prover().await;
    let with_transaction_hash = false;
    let batched = true;
    for is_v2_ix in [true, false].into_iter() {
        let config = {
            let mut config = ProgramTestConfig::default_with_batched_trees(false);
            config.with_prover = false;
            config.additional_programs = Some(vec![(
                "create_address_test_program",
                create_address_test_program::ID,
            )]);
            config.v2_state_tree_config = Some(InitStateTreeAccountsInstructionData::default());
            config.v2_address_tree_config =
                Some(InitAddressTreeAccountsInstructionData::test_default());
            config
        };
        let mut rpc = LightProgramTest::new(config)
            .await
            .expect("Failed to setup test programs with accounts");
        let env = rpc.test_accounts.clone();
        let queue = if batched {
            env.v2_state_trees[0].output_queue
        } else {
            env.v1_state_trees[0].nullifier_queue
        };
        let tree = if batched {
            env.v2_state_trees[0].merkle_tree
        } else {
            env.v1_state_trees[0].merkle_tree
        };
        let address_tree = if batched {
            env.v2_address_trees[0]
        } else {
            env.v1_address_trees[0].merkle_tree
        };
        let address_queue = if batched {
            env.v2_address_trees[0]
        } else {
            env.v1_address_trees[0].queue
        };
        let cpi_context_account = if batched {
            env.v2_state_trees[0].cpi_context
        } else {
            env.v1_state_trees[0].cpi_context
        };

        let payer = rpc.get_payer().insecure_clone();
        let mut test_indexer = TestIndexer::init_from_acounts(&payer, &env, 0).await;
        // Create 3 input accounts.
        {
            let output_accounts = vec![
                get_compressed_output_account(
                    true,
                    if batched {
                        env.v2_state_trees[0].output_queue
                    } else {
                        env.v1_state_trees[0].merkle_tree
                    }
                );
                3
            ];
            local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                output_accounts,
                vec![],
                None,
                None,
                None,
                is_v2_ix,
                true,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await
            .unwrap()
            .unwrap();
        }

        let seed = [1u8; 32];
        let address = if batched {
            derive_address(
                &seed,
                &address_tree.to_bytes(),
                &create_address_test_program::ID.to_bytes(),
            )
        } else {
            derive_address_legacy(&address_tree.into(), &seed).unwrap()
        };

        let seed1 = [2u8; 32];
        let address1 = if batched {
            derive_address(
                &seed1,
                &address_tree.to_bytes(),
                &create_address_test_program::ID.to_bytes(),
            )
        } else {
            derive_address_legacy(&address_tree.into(), &seed1).unwrap()
        };
        let addresses_with_tree = vec![
            AddressWithTree {
                address,
                tree: address_tree,
            },
            AddressWithTree {
                address: address1,
                tree: address_tree,
            },
        ];

        let rpc_result = rpc
            .get_validity_proof(Vec::new(), addresses_with_tree, None)
            .await
            .unwrap();
        let new_address_params = NewAddressParamsAssigned {
            seed,
            address_queue_pubkey: address_queue.into(),
            address_merkle_tree_pubkey: address_tree.into(),
            address_merkle_tree_root_index: rpc_result.value.get_address_root_indices()[0],
            assigned_account_index: Some(2),
        };
        let new_address_params1 = NewAddressParamsAssigned {
            seed: seed1,
            address_queue_pubkey: address_queue.into(),
            address_merkle_tree_pubkey: address_tree.into(),
            address_merkle_tree_root_index: rpc_result.value.get_address_root_indices()[1],
            assigned_account_index: Some(0),
        };
        let owner_account1 = Pubkey::new_unique();
        // Insert into cpi context.
        {
            let input_accounts = vec![get_compressed_input_account(MerkleContext {
                merkle_tree_pubkey: tree.into(),
                queue_pubkey: queue.into(),
                leaf_index: 2,
                prove_by_index: true,
                tree_type: TreeType::StateV2,
            })];
            let mut output_account = get_compressed_output_account(false, queue);
            output_account.compressed_account.address = Some(address1);
            output_account.compressed_account.owner = owner_account1.into();
            local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                input_accounts,
                vec![output_account],
                vec![],
                None,
                None,
                None,
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                Some(CompressedCpiContext {
                    first_set_context: true,
                    ..Default::default()
                }),
                Some(cpi_context_account),
            )
            .await
            .unwrap();
            let output_account_balance =
                test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&owner_account1);
            assert!(output_account_balance.is_empty());
        }
        let owner_account2 = Pubkey::new_unique();
        // Insert into cpi context 2.
        {
            let input_accounts = vec![get_compressed_input_account(MerkleContext {
                merkle_tree_pubkey: tree.into(),
                queue_pubkey: queue.into(),
                leaf_index: 0,
                prove_by_index: true,
                tree_type: TreeType::StateV2,
            })];
            let mut output_account = get_compressed_output_account(false, queue);
            output_account.compressed_account.owner = owner_account2.into();
            local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                input_accounts,
                vec![output_account],
                vec![],
                None,
                None,
                None,
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                Some(CompressedCpiContext {
                    first_set_context: false,
                    set_context: true,
                    ..Default::default()
                }),
                Some(cpi_context_account),
            )
            .await
            .unwrap();
            let output_account_balance =
                test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&owner_account2);
            assert!(output_account_balance.is_empty());
        }
        // Execute cpi context.
        {
            let input_accounts = vec![get_compressed_input_account(MerkleContext {
                merkle_tree_pubkey: tree.into(),
                queue_pubkey: queue.into(),
                leaf_index: 1,
                prove_by_index: true,
                tree_type: TreeType::StateV2,
            })];
            let mut output_account = get_compressed_output_account(true, queue);
            output_account.compressed_account.address = Some(address);
            local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                input_accounts,
                vec![output_account],
                vec![new_address_params, new_address_params1],
                rpc_result.value.proof.0,
                None,
                None,
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                Some(CompressedCpiContext {
                    first_set_context: false,
                    set_context: false,
                    ..Default::default()
                }),
                Some(cpi_context_account),
            )
            .await
            .unwrap();
            let output_account_balance = test_indexer
                .get_compressed_accounts_with_merkle_context_by_owner(
                    &create_address_test_program::ID,
                );
            assert_eq!(output_account_balance.len(), 1);
            assert_eq!(
                output_account_balance[0].compressed_account.address,
                Some(address)
            );

            let account = test_indexer
                .get_compressed_account(address1, None)
                .await
                .unwrap()
                .value
                .unwrap();
            assert_eq!(account.owner, owner_account1);
            let output_account_balance =
                test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&owner_account1);
            assert_eq!(
                output_account_balance[0].compressed_account.address,
                Some(address1)
            );
            let output_account_balance =
                test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&owner_account2);
            assert_eq!(output_account_balance[0].compressed_account.address, None);
        }
    }
}

#[serial]
#[tokio::test]
async fn cpi_context_with_account_info() {
    spawn_prover().await;
    let with_transaction_hash = false;
    let batched = true;
    for is_v2_ix in [true, false].into_iter() {
        let config = if batched {
            let mut config = ProgramTestConfig::default_with_batched_trees(false);
            config.with_prover = false;
            config.additional_programs = Some(vec![(
                "create_address_test_program",
                create_address_test_program::ID,
            )]);
            config.v2_state_tree_config = Some(InitStateTreeAccountsInstructionData::default());
            config
        } else {
            ProgramTestConfig::new(
                false,
                Some(vec![(
                    "create_address_test_program",
                    create_address_test_program::ID,
                )]),
            )
        };
        let mut rpc = LightProgramTest::new(config).await.unwrap();
        let env = rpc.test_accounts.clone();
        let queue = if batched {
            env.v2_state_trees[0].output_queue
        } else {
            env.v1_state_trees[0].nullifier_queue
        };
        let tree = if batched {
            env.v2_state_trees[0].merkle_tree
        } else {
            env.v1_state_trees[0].merkle_tree
        };
        let address_tree = if batched {
            env.v2_address_trees[0]
        } else {
            env.v1_address_trees[0].merkle_tree
        };
        let address_queue = if batched {
            env.v2_address_trees[0]
        } else {
            env.v1_address_trees[0].queue
        };
        let cpi_context_account = if batched {
            env.v2_state_trees[0].cpi_context
        } else {
            env.v1_state_trees[0].cpi_context
        };
        println!("cpi context account {:?}", cpi_context_account);
        println!("cpi context account {:?}", cpi_context_account.to_bytes());

        let payer = rpc.get_payer().insecure_clone();
        let mut test_indexer = TestIndexer::init_from_acounts(&payer, &env, 0).await;
        // Create 3 input accounts.
        {
            let output_accounts = vec![
                get_compressed_output_account(
                    true,
                    if batched {
                        env.v2_state_trees[0].output_queue
                    } else {
                        env.v1_state_trees[0].merkle_tree
                    }
                );
                3
            ];
            local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                output_accounts,
                vec![],
                None,
                None,
                None,
                is_v2_ix,
                true,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                None,
                None,
            )
            .await
            .unwrap()
            .unwrap();
        }

        let seed = [1u8; 32];
        let address = if batched {
            derive_address(
                &seed,
                &address_tree.to_bytes(),
                &create_address_test_program::ID.to_bytes(),
            )
        } else {
            derive_address_legacy(&address_tree.into(), &seed).unwrap()
        };

        let seed1 = [2u8; 32];
        let address1 = if batched {
            derive_address(
                &seed1,
                &address_tree.to_bytes(),
                &create_address_test_program::ID.to_bytes(),
            )
        } else {
            derive_address_legacy(&address_tree.into(), &seed1).unwrap()
        };
        let addresses_with_tree = vec![
            AddressWithTree {
                address,
                tree: address_tree,
            },
            AddressWithTree {
                address: address1,
                tree: address_tree,
            },
        ];

        let rpc_result = rpc
            .get_validity_proof(Vec::new(), addresses_with_tree, None)
            .await
            .unwrap();
        let new_address_params = NewAddressParamsAssigned {
            seed,
            address_queue_pubkey: address_queue.into(),
            address_merkle_tree_pubkey: address_tree.into(),
            address_merkle_tree_root_index: rpc_result.value.get_address_root_indices()[0],
            assigned_account_index: Some(2),
        };
        let new_address_params1 = NewAddressParamsAssigned {
            seed: seed1,
            address_queue_pubkey: address_queue.into(),
            address_merkle_tree_pubkey: address_tree.into(),
            address_merkle_tree_root_index: rpc_result.value.get_address_root_indices()[1],
            assigned_account_index: Some(0),
        };
        let owner_account1 = Pubkey::new_unique();
        // Insert into cpi context.
        {
            let output_account = get_output_account_info(0);
            let account_info = CompressedAccountInfo {
                address: Some(address1),
                input: None,
                output: Some(output_account),
            };
            println!(
                "ACCOUNT_COMPRESSION_PROGRAM_ID {:?}",
                ACCOUNT_COMPRESSION_PROGRAM_ID
            );
            local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                Vec::new(),
                vec![],
                vec![],
                None,
                None,
                Some(vec![account_info]),
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                Some(CompressedCpiContext {
                    first_set_context: true,
                    ..Default::default()
                }),
                Some(cpi_context_account),
            )
            .await
            .unwrap();
            let output_account_balance =
                test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&owner_account1);
            assert!(output_account_balance.is_empty());
        }
        let owner_account2 = Pubkey::new_unique();
        // Insert into cpi context 2.
        {
            let input_account = get_input_account_info(PackedMerkleContext {
                merkle_tree_pubkey_index: 1,
                queue_pubkey_index: 0,
                leaf_index: 2,
                prove_by_index: true,
            });
            let account_info1 = CompressedAccountInfo {
                address: None,
                input: Some(input_account),
                output: None,
            };
            let input_account = get_input_account_info(PackedMerkleContext {
                merkle_tree_pubkey_index: 1,
                queue_pubkey_index: 0,
                leaf_index: 0,
                prove_by_index: true,
            });
            let output_account = get_output_account_info(0);
            let account_info2 = CompressedAccountInfo {
                address: None,
                input: Some(input_account),
                output: Some(output_account),
            };
            local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                vec![],
                vec![],
                None,
                None,
                Some(vec![account_info1, account_info2]),
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                Some(CompressedCpiContext {
                    first_set_context: false,
                    set_context: true,
                    ..Default::default()
                }),
                Some(cpi_context_account),
            )
            .await
            .unwrap();
            let output_account_balance =
                test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&owner_account2);
            assert!(output_account_balance.is_empty());
        }
        // Execute cpi context.
        {
            let input_account = get_input_account_info(PackedMerkleContext {
                merkle_tree_pubkey_index: 1,
                queue_pubkey_index: 0,
                leaf_index: 1,
                prove_by_index: true,
            });
            let account_info2 = CompressedAccountInfo {
                address: None,
                input: Some(input_account),
                output: Some(get_output_account_info(0)),
            };
            let account_info1 = CompressedAccountInfo {
                address: Some(address),
                input: None,
                output: Some(get_output_account_info(0)),
            };

            local_sdk::perform_test_transaction(
                &mut rpc,
                &mut test_indexer,
                &payer,
                vec![],
                vec![],
                vec![new_address_params, new_address_params1],
                rpc_result.value.proof.0,
                None,
                Some(vec![account_info1, account_info2]),
                is_v2_ix,
                with_transaction_hash,
                Vec::new(),
                Vec::new(),
                queue,
                tree,
                false,
                None,
                false,
                Some(CompressedCpiContext {
                    first_set_context: false,
                    set_context: false,
                    ..Default::default()
                }),
                Some(cpi_context_account),
            )
            .await
            .unwrap();
            let mut output_account_balance = test_indexer
                .get_compressed_accounts_with_merkle_context_by_owner(
                    &create_address_test_program::ID,
                );
            output_account_balance.sort_by(|a, b| {
                a.merkle_context
                    .leaf_index
                    .cmp(&b.merkle_context.leaf_index)
            });
            assert_eq!(output_account_balance.len(), 4);
            assert_eq!(
                output_account_balance[2].compressed_account.address,
                Some(address)
            );
            assert_eq!(output_account_balance[1].compressed_account.address, None);
            assert_eq!(
                output_account_balance[0].compressed_account.address,
                Some(address1)
            );
            assert_eq!(output_account_balance[3].compressed_account.address, None);
        }
    }
}

#[tokio::test]
async fn compress_sol_with_read_only() {
    let with_transaction_hash = false;
    let batched = true;
    for is_v2_ix in [true, false].into_iter() {
        let config = if batched {
            let mut config = ProgramTestConfig::default_with_batched_trees(false);
            config.with_prover = false;
            config.additional_programs = Some(vec![(
                "create_address_test_program",
                create_address_test_program::ID,
            )]);
            config.v2_state_tree_config = Some(InitStateTreeAccountsInstructionData::default());
            config
        } else {
            ProgramTestConfig::new(
                false,
                Some(vec![(
                    "create_address_test_program",
                    create_address_test_program::ID,
                )]),
            )
        };
        let mut rpc = LightProgramTest::new(config).await.unwrap();
        let env = rpc.test_accounts.clone();
        let queue = if batched {
            env.v2_state_trees[0].output_queue
        } else {
            env.v1_state_trees[0].nullifier_queue
        };
        let tree = if batched {
            env.v2_state_trees[0].merkle_tree
        } else {
            env.v1_state_trees[0].merkle_tree
        };

        let payer = rpc.get_payer().insecure_clone();
        let mut test_indexer = TestIndexer::init_from_acounts(&payer, &env, 0).await;

        // 1.Compress sol
        {
            let mut output_account = get_compressed_output_account(true, queue);
            let compression_lamports = 1_000_000;
            output_account.compressed_account.lamports = compression_lamports;

            // 1. Failing compress sol, invalid pool derivation.
            {
                let result = local_sdk::perform_test_transaction(
                    &mut rpc,
                    &mut test_indexer,
                    &payer,
                    vec![],
                    vec![output_account.clone()],
                    vec![],
                    None,
                    None,
                    None,
                    is_v2_ix,
                    with_transaction_hash,
                    Vec::new(),
                    Vec::new(),
                    queue,
                    tree,
                    true,
                    Some(compression_lamports),
                    true,
                    None,
                    None,
                )
                .await;
                assert_rpc_error(result, 0, AccountError::InvalidSeeds.into()).unwrap();
            }
            // 2. Functional compress sol
            {
                local_sdk::perform_test_transaction(
                    &mut rpc,
                    &mut test_indexer,
                    &payer,
                    vec![],
                    vec![output_account],
                    vec![],
                    None,
                    None,
                    None,
                    is_v2_ix,
                    with_transaction_hash,
                    Vec::new(),
                    Vec::new(),
                    queue,
                    tree,
                    true,
                    Some(compression_lamports),
                    false,
                    None,
                    None,
                )
                .await
                .unwrap();
                let output_account_balance = test_indexer
                    .get_compressed_accounts_with_merkle_context_by_owner(
                        &create_address_test_program::ID,
                    )[0]
                .compressed_account
                .lamports;
                assert_eq!(output_account_balance, compression_lamports);
            }
        }
        // 2.Decompress sol
        {
            let mut input_account = get_compressed_input_account(MerkleContext {
                merkle_tree_pubkey: tree.into(),
                queue_pubkey: queue.into(),
                leaf_index: 0,
                prove_by_index: true,
                tree_type: TreeType::StateV2,
            });
            let compression_lamports = 1_000_000;
            input_account.compressed_account.lamports = compression_lamports;

            let recipient = Pubkey::new_unique();
            // 3. Failing decompress sol, invalid pool derivation.
            {
                let result = local_sdk::perform_test_transaction(
                    &mut rpc,
                    &mut test_indexer,
                    &payer,
                    vec![input_account.clone()],
                    vec![],
                    vec![],
                    None,
                    Some(recipient),
                    None,
                    is_v2_ix,
                    with_transaction_hash,
                    Vec::new(),
                    Vec::new(),
                    queue,
                    tree,
                    false,
                    Some(compression_lamports),
                    true,
                    None,
                    None,
                )
                .await;
                assert_rpc_error(result, 0, AccountError::InvalidSeeds.into()).unwrap();
            }
            // 4. Functional decompress sol
            {
                local_sdk::perform_test_transaction(
                    &mut rpc,
                    &mut test_indexer,
                    &payer,
                    vec![input_account],
                    vec![],
                    vec![],
                    None,
                    Some(recipient),
                    None,
                    is_v2_ix,
                    with_transaction_hash,
                    Vec::new(),
                    Vec::new(),
                    queue,
                    tree,
                    false,
                    Some(compression_lamports),
                    false,
                    None,
                    None,
                )
                .await
                .unwrap();
                let recipient_balance = rpc.get_balance(&recipient).await.unwrap();
                assert_eq!(recipient_balance, compression_lamports);
            }
        }
    }
}

fn get_input_account_info(merkle_context: PackedMerkleContext) -> InAccountInfo {
    InAccountInfo {
        discriminator: u64::MAX.to_be_bytes(),
        data_hash: [3u8; 32],
        merkle_context,
        lamports: 0,
        root_index: 0,
    }
}
fn get_output_account_info(output_merkle_tree_index: u8) -> OutAccountInfo {
    OutAccountInfo {
        discriminator: u64::MAX.to_be_bytes(),
        data_hash: [3u8; 32],
        output_merkle_tree_index,
        lamports: 0,
        data: thread_rng().gen::<[u8; 32]>().to_vec(),
    }
}

#[serial]
#[tokio::test]
async fn test_duplicate_account_in_inputs_and_read_only() {
    spawn_prover().await;

    let mut config = ProgramTestConfig::default_with_batched_trees(false);
    config.with_prover = false;
    config.additional_programs = Some(vec![(
        "create_address_test_program",
        create_address_test_program::ID,
    )]);
    config.v2_state_tree_config = Some(InitStateTreeAccountsInstructionData::default());

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let env = rpc.test_accounts.clone();
    let queue = env.v2_state_trees[0].output_queue;
    let tree = env.v2_state_trees[0].merkle_tree;

    let payer = rpc.get_payer().insecure_clone();
    let mut test_indexer = TestIndexer::init_from_acounts(&payer, &env, 0).await;

    // Create a compressed account first
    let output_account = get_compressed_output_account(true, queue);
    local_sdk::perform_test_transaction(
        &mut rpc,
        &mut test_indexer,
        &payer,
        vec![],
        vec![output_account.clone()],
        vec![],
        None, // proof
        None,
        None,
        false,
        false,
        Vec::new(),
        Vec::new(),
        queue,
        tree,
        false,
        None,
        false,
        None,
        None,
    )
    .await
    .unwrap()
    .unwrap();

    // Now try to use the same account as both input and read-only
    let compressed_account = test_indexer.compressed_accounts[0].clone();

    let read_only_account = ReadOnlyCompressedAccount {
        account_hash: compressed_account.hash().unwrap(),
        merkle_context: MerkleContext {
            merkle_tree_pubkey: tree.into(),
            queue_pubkey: queue.into(),
            leaf_index: 0,
            prove_by_index: false,
            tree_type: TreeType::StateV2,
        },
        root_index: 0,
    };

    // Get validity proof for the input account
    let rpc_result = rpc
        .get_validity_proof(vec![compressed_account.hash().unwrap()], Vec::new(), None)
        .await
        .unwrap();

    // Attempt transaction with duplicate account - should fail
    let result = local_sdk::perform_test_transaction(
        &mut rpc,
        &mut test_indexer,
        &payer,
        vec![compressed_account], // input_accounts
        vec![],                   // output_accounts
        vec![],                   // new_addresses
        rpc_result.value.proof.0, // proof
        None,                     // sol_compression_recipient
        None,                     // account_infos
        false,                    // v2_ix
        false,                    // with_transaction_hash
        vec![read_only_account],  // read_only_accounts
        Vec::new(),               // read_only_addresses
        queue,
        tree,
        false, // is_compress
        None,  // compress_or_decompress_lamports
        false, // invalid_sol_pool
        None,  // invalid_fee_recipient
        None,  // invalid_cpi_context
    )
    .await;

    assert_rpc_error(
        result,
        0,
        SystemProgramError::DuplicateAccountInInputsAndReadOnly.into(),
    )
    .unwrap();
}

pub mod local_sdk {
    use std::{collections::HashMap, println};

    use anchor_lang::{prelude::AccountMeta, AnchorSerialize};
    use solana_sdk::pubkey::Pubkey;

    const LIGHT_CPI_SIGNER: CpiSigner =
        derive_light_cpi_signer!("FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy");
    use create_address_test_program::create_invoke_read_only_account_info_instruction;
    use light_client::indexer::Indexer;
    use light_compressed_account::{
        compressed_account::{
            CompressedAccountWithMerkleContext, MerkleContext,
            PackedCompressedAccountWithMerkleContext, ReadOnlyCompressedAccount,
        },
        instruction_data::{
            compressed_proof::CompressedProof,
            cpi_context::CompressedCpiContext,
            data::{OutputCompressedAccountWithContext, OutputCompressedAccountWithPackedContext},
            with_account_info::{CompressedAccountInfo, InstructionDataInvokeCpiWithAccountInfo},
            with_readonly::{InAccount, InstructionDataInvokeCpiWithReadOnly},
        },
    };
    use light_compressed_token::process_transfer::transfer_sdk::to_account_metas;
    use light_event::event::BatchPublicTransactionEvent;
    use light_program_test::indexer::TestIndexerExtensions;
    use light_sdk::{
        address::{NewAddressParamsAssigned, ReadOnlyAddress},
        cpi::{CpiAccountsConfig, CpiSigner},
        derive_light_cpi_signer,
        instruction::SystemAccountPubkeys,
    };
    use light_system_program::constants::SOL_POOL_PDA_SEED;
    use light_test_utils::{
        pack::{
            pack_compressed_accounts, pack_new_address_params_assigned,
            pack_output_compressed_accounts, pack_pubkey_usize, pack_read_only_accounts,
            pack_read_only_address_params,
        },
        Rpc, RpcError,
    };
    use solana_sdk::signature::{Keypair, Signer};

    use crate::event::get_compressed_input_account;

    /// Only proof by index.
    pub fn get_read_only_account(merkle_context: MerkleContext) -> ReadOnlyCompressedAccount {
        let account_hash = get_compressed_input_account(merkle_context).hash().unwrap();
        ReadOnlyCompressedAccount {
            account_hash,
            merkle_context,
            root_index: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn perform_test_transaction<R: Rpc, I: Indexer + TestIndexerExtensions>(
        rpc: &mut R,
        test_indexer: &mut I,
        payer: &Keypair,
        input_accounts: Vec<CompressedAccountWithMerkleContext>,
        output_accounts: Vec<OutputCompressedAccountWithContext>,
        new_addresses: Vec<NewAddressParamsAssigned>,
        proof: Option<CompressedProof>,
        sol_compression_recipient: Option<Pubkey>,
        account_infos: Option<Vec<CompressedAccountInfo>>,
        v2_ix: bool,
        with_transaction_hash: bool,
        read_only_accounts: Vec<ReadOnlyCompressedAccount>,
        read_only_addresses: Vec<ReadOnlyAddress>,
        queue: Pubkey,
        tree: Pubkey,
        is_compress: bool,
        compress_or_decompress_lamports: Option<u64>,
        invalid_sol_pool: bool,
        cpi_context: Option<CompressedCpiContext>,
        cpi_context_address: Option<Pubkey>,
    ) -> Result<
        Option<(
            Vec<BatchPublicTransactionEvent>,
            Vec<OutputCompressedAccountWithPackedContext>,
            Vec<PackedCompressedAccountWithMerkleContext>,
        )>,
        RpcError,
    > {
        let mut remaining_accounts = HashMap::<Pubkey, usize>::new();

        if account_infos.is_some() {
            pack_pubkey_usize(&queue, &mut remaining_accounts);
            pack_pubkey_usize(&tree, &mut remaining_accounts);
        }
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
        let packed_inputs = pack_compressed_accounts(
            input_accounts.as_slice(),
            &vec![None; input_accounts.len()],
            &mut remaining_accounts,
        );
        let packed_new_address_params =
            pack_new_address_params_assigned(new_addresses.as_slice(), &mut remaining_accounts);

        let read_only_accounts =
            pack_read_only_accounts(read_only_accounts.as_slice(), &mut remaining_accounts);
        let read_only_addresses =
            pack_read_only_address_params(read_only_addresses.as_slice(), &mut remaining_accounts);

        let ix_data = if account_infos.is_none() {
            InstructionDataInvokeCpiWithReadOnly {
                mode: if v2_ix { 1 } else { 0 },
                bump: 255,
                invoking_program_id: create_address_test_program::ID.into(),
                proof,
                new_address_params: packed_new_address_params,
                is_compress,
                compress_or_decompress_lamports: compress_or_decompress_lamports
                    .unwrap_or_default(),
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
                with_transaction_hash,
                read_only_accounts,
                read_only_addresses,
                with_cpi_context: cpi_context.is_some(),
                cpi_context: cpi_context.unwrap_or_default(),
            }
            .try_to_vec()
            .unwrap()
        } else if let Some(account_infos) = account_infos.as_ref() {
            InstructionDataInvokeCpiWithAccountInfo {
                mode: if v2_ix { 1 } else { 0 },
                bump: 255,
                invoking_program_id: create_address_test_program::ID.into(),
                proof,
                new_address_params: packed_new_address_params,
                read_only_accounts,
                read_only_addresses,
                is_compress,
                compress_or_decompress_lamports: compress_or_decompress_lamports
                    .unwrap_or_default(),
                account_infos: account_infos.clone(),
                with_transaction_hash,
                with_cpi_context: cpi_context.is_some(),
                cpi_context: cpi_context.unwrap_or_default(),
            }
            .try_to_vec()
            .unwrap()
        } else {
            unimplemented!("Invalid mode.")
        };
        println!(
            "sol pool pda {:?}",
            Pubkey::find_program_address(&[SOL_POOL_PDA_SEED], &light_system_program::ID).0
        );
        let remaining_accounts = to_account_metas(remaining_accounts);
        let config = SystemAccountMetaConfig {
            self_program: create_address_test_program::ID,
            cpi_context: cpi_context_address,
            sol_pool_pda: if compress_or_decompress_lamports.is_some() {
                let pool_pubkey = if invalid_sol_pool {
                    Pubkey::find_program_address(&[&[1]], &light_system_program::ID).0
                } else {
                    Pubkey::find_program_address(&[SOL_POOL_PDA_SEED], &light_system_program::ID).0
                };
                Some(pool_pubkey)
            } else {
                None
            },
            sol_compression_recipient,
            v2_ix,
        };

        let instruction_discriminator = if account_infos.is_none() {
            // INVOKE_CPI_WITH_READ_ONLY_INSTRUCTIOM
            [86, 47, 163, 166, 21, 223, 92, 8]
        } else {
            [228, 34, 128, 84, 47, 139, 86, 240]
            // INVOKE_CPI_WITH_ACCOUNT_INFO_INSTRUCTION
        };
        let mut onchain_config = CpiAccountsConfig::new(LIGHT_CPI_SIGNER);
        onchain_config.cpi_context = config.cpi_context.is_some();
        onchain_config.sol_pool_pda = config.sol_pool_pda.is_some();
        onchain_config.sol_compression_recipient = config.sol_compression_recipient.is_some();
        let write_into_cpi_context = if let Some(cpi_context) = cpi_context.as_ref() {
            if v2_ix {
                cpi_context.first_set_context || cpi_context.set_context
            } else {
                false
            }
        } else {
            false
        };
        let remaining_accounts = if write_into_cpi_context {
            vec![
                AccountMeta::new_readonly(
                    SystemAccountPubkeys::default().light_sytem_program,
                    false,
                ),
                AccountMeta::new(Pubkey::new_from_array(LIGHT_CPI_SIGNER.cpi_signer), false),
                AccountMeta::new(config.cpi_context.unwrap(), false),
            ]
        } else {
            [get_light_system_account_metas(config), remaining_accounts].concat()
        };

        let instruction = create_invoke_read_only_account_info_instruction(
            payer.pubkey(),
            [instruction_discriminator.to_vec(), ix_data].concat(),
            onchain_config,
            v2_ix,
            remaining_accounts,
            write_into_cpi_context,
        );

        let res = rpc
            .create_and_send_transaction_with_batched_event(
                &[instruction],
                &payer.pubkey(),
                &[payer],
            )
            .await?;
        if let Some(res) = res {
            println!("signature {:?}", res.1);
            println!("event {:?}", res.0[0].event);
            let slot = rpc.get_slot().await?;
            test_indexer.add_event_and_compressed_accounts(slot, &res.0[0].event);
            Ok(Some((res.0, output_compressed_accounts, packed_inputs)))
        } else {
            Ok(None)
        }
    }

    // Offchain
    #[derive(Debug, Default, Copy, Clone)]
    pub struct SystemAccountMetaConfig {
        pub self_program: Pubkey,
        pub cpi_context: Option<Pubkey>,
        pub sol_compression_recipient: Option<Pubkey>,
        pub sol_pool_pda: Option<Pubkey>,
        pub v2_ix: bool,
    }

    impl SystemAccountMetaConfig {
        pub fn new(self_program: Pubkey) -> Self {
            Self {
                self_program,
                cpi_context: None,
                sol_compression_recipient: None,
                sol_pool_pda: None,
                v2_ix: false,
            }
        }
        pub fn new_with_account_options(self_program: Pubkey) -> Self {
            Self {
                self_program,
                cpi_context: None,
                sol_compression_recipient: None,
                sol_pool_pda: None,
                v2_ix: true,
            }
        }

        pub fn new_with_cpi_context(self_program: Pubkey, cpi_context: Pubkey) -> Self {
            Self {
                self_program,
                cpi_context: Some(cpi_context),
                sol_compression_recipient: None,
                sol_pool_pda: None,
                v2_ix: false,
            }
        }
    }

    #[derive(Default, Debug)]
    pub struct PackedAccounts {
        pre_accounts: Vec<AccountMeta>,
        system_accounts: Vec<AccountMeta>,
        next_index: u8,
        map: HashMap<Pubkey, (u8, AccountMeta)>,
    }

    impl PackedAccounts {
        pub fn new_with_system_accounts(config: SystemAccountMetaConfig) -> Self {
            let mut remaining_accounts = PackedAccounts::default();
            remaining_accounts.add_system_accounts(config);
            remaining_accounts
        }

        pub fn add_pre_accounts_signer(&mut self, pubkey: Pubkey) {
            self.pre_accounts.push(AccountMeta {
                pubkey,
                is_signer: true,
                is_writable: false,
            });
        }

        pub fn add_pre_accounts_signer_mut(&mut self, pubkey: Pubkey) {
            self.pre_accounts.push(AccountMeta {
                pubkey,
                is_signer: true,
                is_writable: true,
            });
        }

        pub fn add_pre_accounts_meta(&mut self, account_meta: AccountMeta) {
            self.pre_accounts.push(account_meta);
        }

        pub fn add_system_accounts(&mut self, config: SystemAccountMetaConfig) {
            self.system_accounts
                .extend(get_light_system_account_metas(config));
        }

        /// Returns the index of the provided `pubkey` in the collection.
        ///
        /// If the provided `pubkey` is not a part of the collection, it gets
        /// inserted with a `next_index`.
        ///
        /// If the privided `pubkey` already exists in the collection, its already
        /// existing index is returned.
        pub fn insert_or_get(&mut self, pubkey: Pubkey) -> u8 {
            self.insert_or_get_config(pubkey, false, true)
        }

        pub fn insert_or_get_read_only(&mut self, pubkey: Pubkey) -> u8 {
            self.insert_or_get_config(pubkey, false, false)
        }

        pub fn insert_or_get_config(
            &mut self,
            pubkey: Pubkey,
            is_signer: bool,
            is_writable: bool,
        ) -> u8 {
            self.map
                .entry(pubkey)
                .or_insert_with(|| {
                    let index = self.next_index;
                    self.next_index += 1;
                    (
                        index,
                        AccountMeta {
                            pubkey,
                            is_signer,
                            is_writable,
                        },
                    )
                })
                .0
        }

        fn hash_set_accounts_to_metas(&self) -> Vec<AccountMeta> {
            let mut packed_accounts = self.map.iter().collect::<Vec<_>>();
            // hash maps are not sorted so we need to sort manually and collect into a vector again
            packed_accounts.sort_by(|a, b| a.1 .0.cmp(&b.1 .0));
            let packed_accounts = packed_accounts
                .iter()
                .map(|(_, (_, k))| k.clone())
                .collect::<Vec<AccountMeta>>();
            packed_accounts
        }

        fn get_offsets(&self) -> (usize, usize) {
            let system_accounts_start_offset = self.pre_accounts.len();
            let packed_accounts_start_offset =
                system_accounts_start_offset + self.system_accounts.len();
            (system_accounts_start_offset, packed_accounts_start_offset)
        }

        /// Converts the collection of accounts to a vector of
        /// [`AccountMeta`](solana_sdk::instruction::AccountMeta), which can be used
        /// as remaining accounts in instructions or CPI calls.
        pub fn to_account_metas(&self) -> (Vec<AccountMeta>, usize, usize) {
            let packed_accounts = self.hash_set_accounts_to_metas();
            let (system_accounts_start_offset, packed_accounts_start_offset) = self.get_offsets();
            let default_pubkeys = SystemAccountPubkeys::default();

            (
                [
                    self.pre_accounts.clone(),
                    self.system_accounts.clone(),
                    packed_accounts,
                    vec![AccountMeta::new_readonly(
                        default_pubkeys.light_sytem_program,
                        false,
                    )],
                ]
                .concat(),
                system_accounts_start_offset,
                packed_accounts_start_offset,
            )
        }
    }

    pub fn get_light_system_account_metas(config: SystemAccountMetaConfig) -> Vec<AccountMeta> {
        let cpi_signer = Pubkey::new_from_array(LIGHT_CPI_SIGNER.cpi_signer);

        let default_pubkeys = SystemAccountPubkeys::default();
        let mut vec = if config.v2_ix {
            // Accounts without noop and self program.
            let vec = vec![
                AccountMeta::new_readonly(default_pubkeys.light_sytem_program, false),
                AccountMeta::new_readonly(cpi_signer, false),
                AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
                AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
                AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
                AccountMeta::new_readonly(default_pubkeys.system_program, false),
            ];
            vec
        } else {
            let vec = vec![
                AccountMeta::new_readonly(default_pubkeys.light_sytem_program, false),
                AccountMeta::new_readonly(cpi_signer, false),
                AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
                AccountMeta::new_readonly(default_pubkeys.noop_program, false),
                AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
                AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
                AccountMeta::new_readonly(config.self_program, false),
            ];

            vec
        };
        if let Some(pubkey) = config.sol_pool_pda {
            println!("sol pool pda pubkey: {:?}", pubkey);
            vec.push(AccountMeta {
                pubkey,
                is_signer: false,
                is_writable: true,
            });
        }
        if let Some(pubkey) = config.sol_compression_recipient {
            vec.push(AccountMeta {
                pubkey,
                is_signer: false,
                is_writable: true,
            });
        }
        if !config.v2_ix {
            vec.push(AccountMeta::new_readonly(
                default_pubkeys.system_program,
                false,
            ));
        }
        if let Some(pubkey) = config.cpi_context {
            vec.push(AccountMeta {
                pubkey,
                is_signer: false,
                is_writable: true,
            });
        }

        vec
    }
}

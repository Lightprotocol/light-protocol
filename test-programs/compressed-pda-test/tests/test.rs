#![cfg(feature = "test-sbf")]

use light_compressed_pda::{
    errors::CompressedPdaError,
    sdk::{
        address::derive_address,
        compressed_account::{
            CompressedAccount, CompressedAccountWithMerkleContext, MerkleContext,
        },
        event::PublicTransactionEvent,
        invoke::{create_invoke_instruction, get_compressed_sol_pda},
    },
    NewAddressParams,
};
use light_hasher::Poseidon;
use light_test_utils::{
    assert_custom_error_or_program_error, create_and_send_transaction,
    create_and_send_transaction_with_event, test_env::setup_test_programs_with_accounts,
    test_indexer::TestIndexer, FeeConfig, TransactionParams,
};
use solana_cli_output::CliAccount;
use solana_program_test::{BanksClientError, ProgramTestContext};
use solana_sdk::{
    instruction::InstructionError,
    pubkey::Pubkey,
    signer::Signer,
    transaction::{Transaction, TransactionError},
};
use tokio::fs::write as async_write;

// TODO: use lazy_static to spawn the server once

/// Tests Execute compressed transaction:
/// 1. should succeed: without compressed account(0 lamports), no in compressed account
/// 2. should fail: in compressed account and invalid zkp
/// 3. should fail: in compressed account and invalid signer
/// 4. should succeed: in compressed account inserted in (1.) and valid zkp
#[tokio::test]
async fn invoke_test() {
    let (mut context, env) = setup_test_programs_with_accounts(None).await;

    let payer = context.payer.insecure_clone();
    let mut test_indexer = TestIndexer::init_from_env(
        &payer,
        &env,
        true,
        true,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;

    let payer_pubkey = payer.pubkey();

    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let nullifier_queue_pubkey = env.nullifier_queue_pubkey;
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: payer_pubkey,
        data: None,
        address: None,
    }];

    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &Vec::new(),
        &output_compressed_accounts,
        &Vec::new(),
        &[merkle_tree_pubkey],
        &Vec::new(),
        &Vec::new(),
        None,
        None,
        false,
        None,
    );

    let event = create_and_send_transaction_with_event(
        &mut context,
        &[instruction],
        &payer_pubkey,
        &[&payer],
        Some(TransactionParams {
            num_input_compressed_accounts: 0,
            num_output_compressed_accounts: 1,
            num_new_addresses: 0,
            compress: 0,
            fee_config: FeeConfig::default(),
        }),
    )
    .await
    .unwrap()
    .unwrap();
    test_indexer.add_event_and_compressed_accounts(event);

    assert_eq!(test_indexer.compressed_accounts.len(), 1);
    let input_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: payer_pubkey,
        data: None,
        address: None,
    }];
    // TODO: assert all compressed account properties
    // check invalid proof
    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey,
            leaf_index: 0,
            nullifier_queue_pubkey,
        }],
        &[merkle_tree_pubkey],
        &[0u16],
        &Vec::new(),
        None,
        None,
        false,
        None,
    );

    let res =
        create_and_send_transaction(&mut context, &[instruction], &payer_pubkey, &[&payer]).await;
    assert!(res.is_err());

    // check invalid signer for in compressed_account
    let invalid_signer_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: Pubkey::new_unique(),
        data: None,
        address: None,
    }];

    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &invalid_signer_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey,
            leaf_index: 0,
            nullifier_queue_pubkey,
        }],
        &[merkle_tree_pubkey],
        &[0u16],
        &Vec::new(),
        None,
        None,
        false,
        None,
    );

    let res =
        create_and_send_transaction(&mut context, &[instruction], &payer.pubkey(), &[&payer]).await;
    assert!(res.is_err());

    // create Merkle proof
    // get zkp from server
    // create instruction as usual with correct zkp
    let compressed_account_with_context = test_indexer.compressed_accounts[0].clone();
    let proof_rpc_res = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&[compressed_account_with_context
                .compressed_account
                .hash::<Poseidon>(
                    &merkle_tree_pubkey,
                    &compressed_account_with_context.merkle_context.leaf_index,
                )
                .unwrap()]),
            None,
            &mut context,
        )
        .await;
    let input_compressed_accounts = vec![compressed_account_with_context.compressed_account];
    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey,
            leaf_index: 0,
            nullifier_queue_pubkey,
        }],
        &[merkle_tree_pubkey],
        &proof_rpc_res.root_indices,
        &Vec::new(),
        Some(proof_rpc_res.proof.clone()),
        None,
        false,
        None,
    );
    println!("Transaction with zkp -------------------------");

    let event = create_and_send_transaction_with_event(
        &mut context,
        &[instruction],
        &payer_pubkey,
        &[&payer],
        Some(TransactionParams {
            num_input_compressed_accounts: 1,
            num_output_compressed_accounts: 1,
            num_new_addresses: 0,
            compress: 0,
            fee_config: FeeConfig::default(),
        }),
    )
    .await
    .unwrap()
    .unwrap();
    test_indexer.add_event_and_compressed_accounts(event);

    println!("Double spend -------------------------");
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: Pubkey::new_unique(),
        data: None,
        address: None,
    }];
    // double spend
    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey,
            leaf_index: 0,
            nullifier_queue_pubkey,
        }],
        &[merkle_tree_pubkey],
        &proof_rpc_res.root_indices,
        &Vec::new(),
        Some(proof_rpc_res.proof.clone()),
        None,
        false,
        None,
    );
    let res =
        create_and_send_transaction(&mut context, &[instruction], &payer.pubkey(), &[&payer]).await;
    assert!(res.is_err());
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: Pubkey::new_unique(),
        data: None,
        address: None,
    }];
    // invalid compressed_account
    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey,
            leaf_index: 1,
            nullifier_queue_pubkey,
        }],
        &[merkle_tree_pubkey],
        &proof_rpc_res.root_indices,
        &Vec::new(),
        Some(proof_rpc_res.proof.clone()),
        None,
        false,
        None,
    );
    let res =
        create_and_send_transaction(&mut context, &[instruction], &payer.pubkey(), &[&payer]).await;
    assert!(res.is_err());
}

/// Tests Execute compressed transaction with address:
/// 1. should fail: create out compressed account with address without input compressed account with address or created address
/// 2. should succeed: create out compressed account with new created address
/// 3. should fail: create two addresses with the same seeds
/// 4. should succeed: create two addresses with different seeds
/// 5. should succeed: create multiple addresses with different seeds and spend input compressed accounts
///    testing: (input accounts, new addresses) (1, 1), (1, 2), (2, 1), (2, 2)
#[tokio::test]
async fn test_with_address() {
    let (mut context, env) = setup_test_programs_with_accounts(None).await;
    let payer = context.payer.insecure_clone();
    let mut test_indexer = TestIndexer::init_from_env(
        &payer,
        &env,
        true,
        true,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;

    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let nullifier_queue_pubkey = env.nullifier_queue_pubkey;

    let address_seed = [1u8; 32];
    let derived_address = derive_address(&env.address_merkle_tree_pubkey, &address_seed).unwrap();
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: payer_pubkey,
        data: None,
        address: Some(derived_address), // this should not be sent, only derived on-chain
    }];

    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &Vec::new(),
        &output_compressed_accounts,
        &Vec::new(),
        &[merkle_tree_pubkey],
        &Vec::new(),
        &Vec::new(),
        None,
        None,
        false,
        None,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.get_new_latest_blockhash().await.unwrap(),
    );
    let res = solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await
    .unwrap();
    assert_custom_error_or_program_error(res, CompressedPdaError::InvalidAddress.into()).unwrap();

    let event = create_addresses(
        &mut context,
        &mut test_indexer,
        &env.address_merkle_tree_pubkey,
        &env.address_merkle_tree_queue_pubkey,
        &env.merkle_tree_pubkey,
        &env.nullifier_queue_pubkey,
        &[address_seed],
        &Vec::new(),
        true,
    )
    .await
    .unwrap()
    .unwrap();
    test_indexer.add_event_and_compressed_accounts(event);
    assert_eq!(test_indexer.compressed_accounts.len(), 1);
    assert_eq!(
        test_indexer.compressed_accounts[0]
            .compressed_account
            .address
            .unwrap(),
        derived_address
    );

    let compressed_account_with_context = test_indexer.compressed_accounts[0].clone();
    let proof_rpc_res = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&[compressed_account_with_context
                .compressed_account
                .hash::<Poseidon>(
                    &merkle_tree_pubkey,
                    &compressed_account_with_context.merkle_context.leaf_index,
                )
                .unwrap()]),
            None,
            &mut context,
        )
        .await;
    let input_compressed_accounts = vec![compressed_account_with_context.compressed_account];
    let recipient_pubkey = Pubkey::new_unique();
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: recipient_pubkey,
        data: None,
        address: Some(derived_address),
    }];
    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey,
            leaf_index: 0,
            nullifier_queue_pubkey,
        }],
        &[merkle_tree_pubkey],
        &proof_rpc_res.root_indices,
        &Vec::new(),
        Some(proof_rpc_res.proof.clone()),
        None,
        false,
        None,
    );
    println!("Transaction with zkp -------------------------");
    let event = create_and_send_transaction_with_event(
        &mut context,
        &[instruction],
        &payer_pubkey,
        &[&payer],
        Some(TransactionParams {
            num_input_compressed_accounts: 1,
            num_output_compressed_accounts: 1,
            num_new_addresses: 0,
            compress: 0,
            fee_config: FeeConfig::default(),
        }),
    )
    .await
    .unwrap()
    .unwrap();

    test_indexer.add_event_and_compressed_accounts(event);
    assert_eq!(test_indexer.compressed_accounts.len(), 1);
    assert_eq!(
        test_indexer.compressed_accounts[0]
            .compressed_account
            .address
            .unwrap(),
        derived_address
    );
    assert_eq!(
        test_indexer.compressed_accounts[0].compressed_account.owner,
        recipient_pubkey
    );

    let address_seed_2 = [2u8; 32];

    let event = create_addresses(
        &mut context,
        &mut test_indexer,
        &env.address_merkle_tree_pubkey,
        &env.address_merkle_tree_queue_pubkey,
        &env.merkle_tree_pubkey,
        &env.nullifier_queue_pubkey,
        &[address_seed_2, address_seed_2],
        &Vec::new(),
        true,
    )
    .await;
    // Should fail to insert the same address twice in the same tx
    assert!(matches!(
        event,
        Err(BanksClientError::TransactionError(
            // ElementAlreadyExists
            TransactionError::InstructionError(0, InstructionError::Custom(9002))
        ))
    ));

    println!("test 2in -------------------------");

    let address_seed_3 = [3u8; 32];
    let event = create_addresses(
        &mut context,
        &mut test_indexer,
        &env.address_merkle_tree_pubkey,
        &env.address_merkle_tree_queue_pubkey,
        &env.merkle_tree_pubkey,
        &env.nullifier_queue_pubkey,
        &[address_seed_2, address_seed_3],
        &Vec::new(),
        true,
    )
    .await
    .unwrap()
    .unwrap();
    test_indexer.add_event_and_compressed_accounts(event);

    // spend one input compressed accounts and create one new address
    println!("test combined -------------------------");

    let test_inputs = vec![
        (1, 1),
        (1, 2),
        (2, 1),
        (2, 2),
        (3, 1),
        // (3, 2), TODO: enable once heap optimization is done
        // (4, 1),
        // (4, 2),
    ];
    for (n_input_compressed_accounts, n_new_addresses) in test_inputs {
        let compressed_input_accounts =
            test_indexer.compressed_accounts[1..n_input_compressed_accounts].to_vec();
        let mut address_vec = Vec::new();
        // creates multiple seeds by taking the number of input accounts and zeroing out the jth byte
        for j in 0..n_new_addresses {
            let mut address_seed = [n_input_compressed_accounts as u8; 32];
            address_seed[j + (n_new_addresses * 2)] = 0_u8;
            address_vec.push(address_seed);
        }

        let event = create_addresses(
            &mut context,
            &mut test_indexer,
            &env.address_merkle_tree_pubkey,
            &env.address_merkle_tree_queue_pubkey,
            &env.merkle_tree_pubkey,
            &env.nullifier_queue_pubkey,
            &address_vec,
            &compressed_input_accounts,
            false, // TODO: enable once heap optimization is done
        )
        .await
        .unwrap()
        .unwrap();
        test_indexer.add_event_and_compressed_accounts(event);
        // there exists a compressed account with the address x
        for address_seed in address_vec.iter() {
            assert!(test_indexer
                .compressed_accounts
                .iter()
                .any(|x| x.compressed_account.address
                    == Some(
                        derive_address(&env.address_merkle_tree_pubkey, address_seed).unwrap()
                    )));
        }
        // input compressed accounts are spent
        for compressed_account in compressed_input_accounts.iter() {
            assert!(test_indexer
                .nullified_compressed_accounts
                .iter()
                .any(|x| x.compressed_account == compressed_account.compressed_account));
        }
        // TODO: assert that output compressed accounts with addresses of input accounts are created once enabled
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn create_addresses(
    context: &mut ProgramTestContext,
    test_indexer: &mut TestIndexer,
    address_merkle_tree_pubkey: &Pubkey,
    address_merkle_tree_queue_pubkey: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
    nullifier_queue_pubkey: &Pubkey,
    address_seeds: &[[u8; 32]],
    input_compressed_accounts: &[CompressedAccountWithMerkleContext],
    create_out_compressed_accounts_for_input_compressed_accounts: bool,
) -> Result<Option<PublicTransactionEvent>, BanksClientError> {
    let mut derived_addresses = Vec::new();
    for address_seed in address_seeds.iter() {
        let derived_address = derive_address(address_merkle_tree_pubkey, address_seed).unwrap();
        derived_addresses.push(derived_address);
    }
    let mut compressed_account_hashes = Vec::new();

    let compressed_account_input_hashes = if input_compressed_accounts.is_empty() {
        None
    } else {
        for compressed_account in input_compressed_accounts.iter() {
            compressed_account_hashes.push(
                compressed_account
                    .compressed_account
                    .hash::<Poseidon>(
                        merkle_tree_pubkey,
                        &compressed_account.merkle_context.leaf_index,
                    )
                    .unwrap(),
            );
        }
        Some(compressed_account_hashes.as_slice())
    };
    let proof_rpc_res = test_indexer
        .create_proof_for_compressed_accounts(
            compressed_account_input_hashes,
            Some(derived_addresses.as_slice()),
            context,
        )
        .await;
    let mut address_params = Vec::new();

    for (i, seed) in address_seeds.iter().enumerate() {
        let new_address_params = NewAddressParams {
            address_queue_pubkey: *address_merkle_tree_queue_pubkey,
            address_merkle_tree_pubkey: *address_merkle_tree_pubkey,
            seed: *seed,
            address_merkle_tree_root_index: proof_rpc_res.address_root_indices[i],
        };
        address_params.push(new_address_params);
    }

    let mut output_compressed_accounts = Vec::new();
    for address_param in address_params.iter() {
        output_compressed_accounts.push(CompressedAccount {
            lamports: 0,
            owner: context.payer.pubkey(),
            data: None,
            address: Some(derive_address(address_merkle_tree_pubkey, &address_param.seed).unwrap()),
        });
    }

    if create_out_compressed_accounts_for_input_compressed_accounts {
        for compressed_account in input_compressed_accounts.iter() {
            output_compressed_accounts.push(CompressedAccount {
                lamports: 0,
                owner: context.payer.pubkey(),
                data: None,
                address: compressed_account.compressed_account.address,
            });
        }
    }

    // create two new addresses with the same see should fail
    let instruction = create_invoke_instruction(
        &context.payer.pubkey(),
        &context.payer.pubkey().clone(),
        input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<CompressedAccount>>()
            .as_slice(),
        &output_compressed_accounts,
        input_compressed_accounts
            .iter()
            .map(|x| MerkleContext {
                merkle_tree_pubkey: *merkle_tree_pubkey,
                leaf_index: x.merkle_context.leaf_index,
                nullifier_queue_pubkey: *nullifier_queue_pubkey,
            })
            .collect::<Vec<MerkleContext>>()
            .as_slice(),
        &vec![*merkle_tree_pubkey; output_compressed_accounts.len()],
        &proof_rpc_res.root_indices,
        &address_params,
        Some(proof_rpc_res.proof.clone()),
        None,
        false,
        None,
    );

    let event = create_and_send_transaction_with_event::<PublicTransactionEvent>(
        context,
        &[instruction],
        &context.payer.pubkey(),
        &[&context.payer.insecure_clone()],
        Some(TransactionParams {
            num_input_compressed_accounts: input_compressed_accounts.len() as u8,
            num_output_compressed_accounts: output_compressed_accounts.len() as u8,
            num_new_addresses: address_params.len() as u8,
            compress: 0,
            fee_config: FeeConfig::default(),
        }),
    )
    .await;

    event
}

#[tokio::test]
async fn test_with_compression() {
    let (mut context, env) = setup_test_programs_with_accounts(None).await;
    let payer = context.payer.insecure_clone();

    let payer_pubkey = payer.pubkey();

    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let nullifier_queue_pubkey = env.nullifier_queue_pubkey;
    let address_merkle_tree_pubkey = env.address_merkle_tree_pubkey;
    let test_indexer = TestIndexer::new(
        merkle_tree_pubkey,
        nullifier_queue_pubkey,
        address_merkle_tree_pubkey,
        payer.insecure_clone(),
        true,
        false,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    );
    let compress_amount = 1_000_000;
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: compress_amount,
        owner: payer_pubkey,
        data: None,
        address: None, // this should not be sent, only derived on-chain
    }];

    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &Vec::new(),
        &output_compressed_accounts,
        &Vec::new(),
        &[merkle_tree_pubkey],
        &Vec::new(),
        &Vec::new(),
        None,
        Some(compress_amount),
        false,
        None,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.get_new_latest_blockhash().await.unwrap(),
    );
    let res = solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await
    .unwrap();
    // should fail because of insufficient input funds
    assert_custom_error_or_program_error(res, CompressedPdaError::ComputeOutputSumFailed.into())
        .unwrap();
    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &Vec::new(),
        &output_compressed_accounts,
        &Vec::new(),
        &[merkle_tree_pubkey],
        &Vec::new(),
        &Vec::new(),
        None,
        None,
        true,
        None,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.get_new_latest_blockhash().await.unwrap(),
    );
    let res = solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await
    .unwrap();
    // should fail because of insufficient decompress amount funds
    assert_custom_error_or_program_error(res, CompressedPdaError::ComputeOutputSumFailed.into())
        .unwrap();

    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &Vec::new(),
        &output_compressed_accounts,
        &Vec::new(),
        &[merkle_tree_pubkey],
        &Vec::new(),
        &Vec::new(),
        None,
        Some(compress_amount),
        true,
        None,
    );
    let sender_pre_balance = context
        .banks_client
        .get_account(payer_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let event = create_and_send_transaction_with_event(
        &mut context,
        &[instruction],
        &payer_pubkey,
        &[&payer],
        Some(TransactionParams {
            num_input_compressed_accounts: 0,
            num_output_compressed_accounts: 1,
            num_new_addresses: 0,
            compress: compress_amount as i64,
            fee_config: FeeConfig::default(),
        }),
    )
    .await
    .unwrap()
    .unwrap();

    let compressed_sol_pda_balance = context
        .banks_client
        .get_account(get_compressed_sol_pda())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    assert_eq!(
        compressed_sol_pda_balance, compress_amount,
        "balance of compressed sol pda insufficient, compress sol failed"
    );

    // Wait until now to reduce startup lag by prover server
    let mut test_indexer = test_indexer.await;
    test_indexer.add_event_and_compressed_accounts(event);
    assert_eq!(test_indexer.compressed_accounts.len(), 1);
    assert_eq!(
        test_indexer.compressed_accounts[0]
            .compressed_account
            .address,
        None
    );
    let sender_post_balance = context
        .banks_client
        .get_account(payer_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let network_fee = 5000;
    let state_merkle_tree_rollover_fee = 150;
    assert_eq!(
        sender_pre_balance,
        sender_post_balance + compress_amount + network_fee + state_merkle_tree_rollover_fee,
        "sender balance incorrect, compress sol failed diff {}",
        sender_pre_balance
            - (sender_pre_balance - compress_amount - network_fee - state_merkle_tree_rollover_fee)
    );
    let compressed_account_with_context = test_indexer.compressed_accounts[0].clone();
    let proof_rpc_res = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&[compressed_account_with_context
                .compressed_account
                .hash::<Poseidon>(
                    &merkle_tree_pubkey,
                    &compressed_account_with_context.merkle_context.leaf_index,
                )
                .unwrap()]),
            None,
            &mut context,
        )
        .await;
    let input_compressed_accounts = vec![compressed_account_with_context.compressed_account];
    let recipient_pubkey = Pubkey::new_unique();
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: recipient_pubkey,
        data: None,
        address: None,
    }];
    let recipient = Pubkey::new_unique();
    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey,
            leaf_index: 0,
            nullifier_queue_pubkey,
        }],
        &[merkle_tree_pubkey],
        &proof_rpc_res.root_indices,
        &Vec::new(),
        Some(proof_rpc_res.proof.clone()),
        Some(compress_amount),
        true,
        Some(recipient),
    );
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.get_new_latest_blockhash().await.unwrap(),
    );
    println!("Transaction with zkp -------------------------");

    let res = solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await
    .unwrap();
    // should fail because of insufficient output funds
    assert_custom_error_or_program_error(res, CompressedPdaError::SumCheckFailed.into()).unwrap();

    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey,
            leaf_index: 0,
            nullifier_queue_pubkey,
        }],
        &[merkle_tree_pubkey],
        &proof_rpc_res.root_indices,
        &Vec::new(),
        Some(proof_rpc_res.proof.clone()),
        Some(compress_amount),
        false,
        Some(recipient),
    );
    println!("Transaction with zkp -------------------------");

    let event = create_and_send_transaction_with_event(
        &mut context,
        &[instruction],
        &payer_pubkey,
        &[&payer],
        Some(TransactionParams {
            num_input_compressed_accounts: 1,
            num_output_compressed_accounts: 1,
            num_new_addresses: 0,
            compress: 0, // we are decompressing to a new account not the payer
            fee_config: FeeConfig::default(),
        }),
    )
    .await
    .unwrap()
    .unwrap();
    let recipient_balance = context
        .banks_client
        .get_account(recipient)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    assert_eq!(
        recipient_balance, compress_amount,
        "recipient balance incorrect, decompress sol failed"
    );
    test_indexer.add_event_and_compressed_accounts(event);
    assert_eq!(test_indexer.compressed_accounts.len(), 1);
    assert_eq!(
        test_indexer.compressed_accounts[0]
            .compressed_account
            .address,
        None
    );
    assert_eq!(
        test_indexer.compressed_accounts[0].compressed_account.owner,
        recipient_pubkey
    );
}

#[ignore = "this is a helper function to regenerate accounts"]
#[tokio::test]
async fn regenerate_accounts() {
    let output_dir = "../../cli/accounts/";
    let (mut context, env) = setup_test_programs_with_accounts(None).await;
    // List of public keys to fetch and export
    let pubkeys = vec![
        ("merkle_tree_pubkey", env.merkle_tree_pubkey),
        ("nullifier_queue_pubkey", env.nullifier_queue_pubkey),
        ("governance_authority_pda", env.governance_authority_pda),
        ("group_pda", env.group_pda),
        ("registered_program_pda", env.registered_program_pda),
        ("address_merkle_tree", env.address_merkle_tree_pubkey),
        (
            "address_merkle_tree_queue",
            env.address_merkle_tree_queue_pubkey,
        ),
    ];

    for (name, pubkey) in pubkeys {
        // Fetch account data. Adjust this part to match how you retrieve and structure your account data.
        let account = context.banks_client.get_account(pubkey).await.unwrap();
        println!(
            "{} DISCRIMINATOR {:?}",
            name,
            account.as_ref().unwrap().data[0..8].to_vec()
        );
        let account = CliAccount::new(&pubkey, &account.unwrap(), true);

        // Serialize the account data to JSON. Adjust according to your data structure.
        let json_data = serde_json::to_vec(&account).unwrap();

        // Construct the output file path
        let file_name = format!("{}_{}.json", name, pubkey);
        let file_path = format!("{}{}", output_dir, file_name);
        println!("Writing account data to {}", file_path);

        // Write the JSON data to a file in the specified directory
        async_write(file_path.clone(), json_data).await.unwrap();
    }
}

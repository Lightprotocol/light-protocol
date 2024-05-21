#![cfg(feature = "test-sbf")]
use light_hasher::Poseidon;
use light_system_program::{
    errors::CompressedPdaError,
    sdk::{
        address::derive_address,
        compressed_account::{CompressedAccount, MerkleContext},
        invoke::create_invoke_instruction,
    },
};
use light_test_utils::rpc::errors::RpcError;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::test_rpc::ProgramTestRpcConnection;
use light_test_utils::transaction_params::{FeeConfig, TransactionParams};
use light_test_utils::{
    assert_compressed_tx::assert_created_compressed_accounts,
    assert_custom_error_or_program_error,
    system_program::{
        compress_sol_test, create_addresses_test, decompress_sol_test, transfer_compressed_sol_test,
    },
    test_env::setup_test_programs_with_accounts,
    test_indexer::TestIndexer,
};
use solana_cli_output::CliAccount;
use solana_sdk::transaction::TransactionError;
use solana_sdk::{
    instruction::InstructionError, pubkey::Pubkey, signer::Signer, transaction::Transaction,
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

    let payer = context.get_payer().insecure_clone();
    let mut test_indexer = TestIndexer::<200, ProgramTestRpcConnection>::init_from_env(
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
    let output_merkle_tree_pubkeys = vec![merkle_tree_pubkey];
    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &Vec::new(),
        &output_compressed_accounts,
        &Vec::new(),
        output_merkle_tree_pubkeys.as_slice(),
        &Vec::new(),
        &Vec::new(),
        None,
        None,
        false,
        None,
    );

    let event = context
        .create_and_send_transaction_with_event(
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
    let (created_compressed_accounts, _) = test_indexer.add_event_and_compressed_accounts(&event);
    assert_created_compressed_accounts(
        output_compressed_accounts.as_slice(),
        output_merkle_tree_pubkeys.as_slice(),
        created_compressed_accounts.as_slice(),
        false,
    );

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

    let res = context
        .create_and_send_transaction(&[instruction], &payer_pubkey, &[&payer])
        .await;
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

    let res = context
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;
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
            Some(&[compressed_account_with_context
                .merkle_context
                .merkle_tree_pubkey]),
            None,
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

    let event = context
        .create_and_send_transaction_with_event(
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
    test_indexer.add_event_and_compressed_accounts(&event);

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
    let res = context
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;
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
    let res = context
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;
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
    let payer = context.get_payer().insecure_clone();
    let mut test_indexer = TestIndexer::<200, ProgramTestRpcConnection>::init_from_env(
        &payer,
        &env,
        true,
        true,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;

    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;

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
        context.get_latest_blockhash().await.unwrap(),
    );

    let res = context.process_transaction_with_metadata(transaction).await;
    assert_custom_error_or_program_error(res, CompressedPdaError::InvalidAddress.into()).unwrap();
    println!("creating address -------------------------");
    create_addresses_test(
        &mut context,
        &mut test_indexer,
        &[env.address_merkle_tree_pubkey],
        &[env.address_merkle_tree_queue_pubkey],
        vec![env.merkle_tree_pubkey],
        &[address_seed],
        &Vec::new(),
        false,
        None,
    )
    .await
    .unwrap();
    // transfer with address
    println!("transfer with address-------------------------");

    let compressed_account_with_context = test_indexer.compressed_accounts[0].clone();
    let recipient_pubkey = Pubkey::new_unique();
    transfer_compressed_sol_test(
        &mut context,
        &mut test_indexer,
        &payer,
        &[compressed_account_with_context.clone()],
        &[recipient_pubkey],
        &[compressed_account_with_context
            .merkle_context
            .merkle_tree_pubkey],
        None,
    )
    .await
    .unwrap();
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

    let event = create_addresses_test(
        &mut context,
        &mut test_indexer,
        &[
            env.address_merkle_tree_pubkey,
            env.address_merkle_tree_pubkey,
        ],
        &[
            env.address_merkle_tree_queue_pubkey,
            env.address_merkle_tree_queue_pubkey,
        ],
        vec![env.merkle_tree_pubkey, env.merkle_tree_pubkey],
        &[address_seed_2, address_seed_2],
        &Vec::new(),
        false,
        None,
    )
    .await;
    // Should fail to insert the same address twice in the same tx
    assert!(matches!(
        event,
        Err(RpcError::TransactionError(
            // ElementAlreadyExists
            TransactionError::InstructionError(0, InstructionError::Custom(9002))
        ))
    ));

    println!("test 2in -------------------------");

    let address_seed_3 = [3u8; 32];
    create_addresses_test(
        &mut context,
        &mut test_indexer,
        &[
            env.address_merkle_tree_pubkey,
            env.address_merkle_tree_pubkey,
        ],
        &[
            env.address_merkle_tree_queue_pubkey,
            env.address_merkle_tree_queue_pubkey,
        ],
        vec![env.merkle_tree_pubkey, env.merkle_tree_pubkey],
        &[address_seed_2, address_seed_3],
        &Vec::new(),
        false,
        None,
    )
    .await
    .unwrap();

    // Test combination
    // (num_input_compressed_accounts, num_new_addresses)
    let test_inputs = vec![
        (1, 1),
        (1, 2),
        (2, 1),
        (2, 2),
        (3, 1),
        (3, 2),
        (4, 1),
        (4, 2),
    ];
    for (n_input_compressed_accounts, n_new_addresses) in test_inputs {
        let compressed_input_accounts = test_indexer
            .get_compressed_accounts_by_owner(&payer_pubkey)[0..n_input_compressed_accounts]
            .to_vec();
        let mut address_vec = Vec::new();
        // creates multiple seeds by taking the number of input accounts and zeroing out the jth byte
        for j in 0..n_new_addresses {
            let mut address_seed = [n_input_compressed_accounts as u8; 32];
            address_seed[j + (n_new_addresses * 2)] = 0_u8;
            address_vec.push(address_seed);
        }

        create_addresses_test(
            &mut context,
            &mut test_indexer,
            &vec![env.address_merkle_tree_pubkey; n_new_addresses],
            &vec![env.address_merkle_tree_queue_pubkey; n_new_addresses],
            vec![env.merkle_tree_pubkey; n_new_addresses],
            &address_vec,
            &compressed_input_accounts,
            true,
            None,
        )
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn test_with_compression() {
    let (mut context, env) = setup_test_programs_with_accounts(None).await;
    let payer = context.get_payer().insecure_clone();

    let payer_pubkey = payer.pubkey();

    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let nullifier_queue_pubkey = env.nullifier_queue_pubkey;
    let mut test_indexer = TestIndexer::<200, ProgramTestRpcConnection>::init_from_env(
        &payer,
        &env,
        true,
        false,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;
    let compress_amount = 1_000_000;
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: compress_amount + 1,
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
        Some(compress_amount),
        false,
        None,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.get_latest_blockhash().await.unwrap(),
    );

    let result = context.process_transaction_with_metadata(transaction).await;
    // should fail because of insufficient input funds
    assert_custom_error_or_program_error(result, CompressedPdaError::ComputeOutputSumFailed.into())
        .unwrap();
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: compress_amount,
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
        true,
        None,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.get_latest_blockhash().await.unwrap(),
    );

    let result = context.process_transaction_with_metadata(transaction).await;
    // should fail because of insufficient decompress amount funds
    assert_custom_error_or_program_error(result, CompressedPdaError::ComputeOutputSumFailed.into())
        .unwrap();

    compress_sol_test(
        &mut context,
        &mut test_indexer,
        &payer,
        &Vec::new(),
        false,
        compress_amount,
        &env.merkle_tree_pubkey,
        None,
    )
    .await
    .unwrap();

    let compressed_account_with_context = test_indexer.compressed_accounts.last().unwrap().clone();
    let proof_rpc_res = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&[compressed_account_with_context
                .compressed_account
                .hash::<Poseidon>(
                    &merkle_tree_pubkey,
                    &compressed_account_with_context.merkle_context.leaf_index,
                )
                .unwrap()]),
            Some(&[compressed_account_with_context
                .merkle_context
                .merkle_tree_pubkey]),
            None,
            None,
            &mut context,
        )
        .await;
    let input_compressed_accounts =
        vec![compressed_account_with_context.clone().compressed_account];
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
        context.get_latest_blockhash().await.unwrap(),
    );
    println!("Transaction with zkp -------------------------");

    let result = context.process_transaction_with_metadata(transaction).await;
    // should fail because of insufficient output funds
    assert_custom_error_or_program_error(result, CompressedPdaError::SumCheckFailed.into())
        .unwrap();

    let compressed_account_with_context =
        test_indexer.get_compressed_accounts_by_owner(&payer_pubkey)[0].clone();
    decompress_sol_test(
        &mut context,
        &mut test_indexer,
        &payer,
        &vec![compressed_account_with_context],
        &recipient_pubkey,
        compress_amount,
        &env.merkle_tree_pubkey,
        None,
    )
    .await
    .unwrap();
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
        let account = context.get_account(pubkey).await.unwrap();
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

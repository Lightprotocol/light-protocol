#![cfg(feature = "test-sbf")]

use std::{assert_eq, println, vec::Vec};

use account_compression::{
    utils::constants::{
        STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_HEIGHT, STATE_MERKLE_TREE_ROOTS,
    },
    StateMerkleTreeAccount,
};
use anchor_lang::AnchorDeserialize;
use circuitlib_rs::{
    gnark::{
        constants::{INCLUSION_PATH, SERVER_ADDRESS},
        helpers::spawn_gnark_server,
        inclusion_json_formatter::InclusionJsonStruct,
        proof_helpers::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
    },
    inclusion::merkle_inclusion_proof_inputs::{InclusionMerkleProofInputs, InclusionProofInputs},
};
use light_test_utils::{
    create_and_send_transaction, get_hash_set, test_env::setup_test_programs_with_accounts,
    AccountZeroCopy,
};
use num_bigint::BigInt;
use num_traits::ops::bytes::FromBytes;
use psp_compressed_pda::{
    compressed_account::{derive_address, CompressedAccount, CompressedAccountWithMerkleContext},
    event::PublicTransactionEvent,
    sdk::{create_execute_compressed_instruction, get_compressed_sol_pda},
    utils::CompressedProof,
    CompressedSolPda, ErrorCode, NewAddressParams,
};
use reqwest::Client;
use solana_cli_output::CliAccount;
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    instruction::{Instruction, InstructionError},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};
use tokio::fs::write as async_write;

// TODO: use lazy_static to spawn the server once

async fn init_mock_indexer() -> MockIndexer {
    let env: light_test_utils::test_env::EnvWithAccounts =
        setup_test_programs_with_accounts().await;
    let context = env.context;
    let payer = context.payer.insecure_clone();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let indexed_array_pubkey = env.indexed_array_pubkey;
    MockIndexer::new(
        merkle_tree_pubkey,
        indexed_array_pubkey,
        payer.insecure_clone(),
    )
    .await
}

/// Tests Execute compressed transaction:
/// 1. should succeed: without compressed account(0 lamports), no in compressed account
/// 2. should fail: in compressed account and invalid zkp
/// 3. should fail: in compressed account and invalid signer
/// 4. should succeed: in compressed account inserted in (1.) and valid zkp
#[tokio::test]
async fn test_execute_compressed_transaction() {
    let env: light_test_utils::test_env::EnvWithAccounts =
        setup_test_programs_with_accounts().await;
    let mut context = env.context;
    let payer = context.payer.insecure_clone();

    let payer_pubkey = payer.pubkey();

    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let indexed_array_pubkey = env.indexed_array_pubkey;

    let mut mock_indexer = init_mock_indexer().await;

    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: payer_pubkey,
        data: None,
        address: None,
    }];
    let proof_mock = CompressedProof {
        a: [0u8; 32],
        b: [0u8; 64],
        c: [0u8; 32],
    };

    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &Vec::new(),
        &output_compressed_accounts,
        &Vec::new(),
        &Vec::new(),
        &Vec::new(),
        &[merkle_tree_pubkey],
        &Vec::new(),
        &Vec::new(),
        &proof_mock,
        None,
        false,
        None,
    );

    // TODO: add function to create_send_transaction_update_indexer
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
    .await;

    mock_indexer.add_lamport_compressed_accounts(
        res.unwrap()
            .metadata
            .unwrap()
            .return_data
            .unwrap()
            .data
            .to_vec(),
    );
    assert_eq!(mock_indexer.compressed_accounts.len(), 1);
    let input_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: payer_pubkey,
        data: None,
        address: None,
    }];
    // TODO: assert all compressed account properties
    // check invalid proof
    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[merkle_tree_pubkey],
        &[0u32],
        &[indexed_array_pubkey],
        &[merkle_tree_pubkey],
        &[0u16],
        &Vec::new(),
        &proof_mock,
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

    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &invalid_signer_compressed_accounts,
        &output_compressed_accounts,
        &[merkle_tree_pubkey],
        &[0u32],
        &[indexed_array_pubkey],
        &[merkle_tree_pubkey],
        &[0u16],
        &Vec::new(),
        &proof_mock,
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
    let compressed_account_with_context = mock_indexer.compressed_accounts[0].clone();
    let (root_indices, proof) = mock_indexer
        .create_proof_for_compressed_accounts(
            &[compressed_account_with_context
                .compressed_account
                .hash(
                    &merkle_tree_pubkey,
                    &compressed_account_with_context.leaf_index,
                )
                .unwrap()],
            &mut context,
        )
        .await;
    let input_compressed_accounts = vec![compressed_account_with_context.compressed_account];
    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[merkle_tree_pubkey],
        &[0u32],
        &[indexed_array_pubkey],
        &[merkle_tree_pubkey],
        &root_indices,
        &Vec::new(),
        &proof,
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
    println!("Transaction with zkp -------------------------");

    let res = solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await;
    mock_indexer.add_lamport_compressed_accounts(
        res.unwrap()
            .metadata
            .unwrap()
            .return_data
            .unwrap()
            .data
            .to_vec(),
    );

    println!("Double spend -------------------------");
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: Pubkey::new_unique(),
        data: None,
        address: None,
    }];
    // double spend
    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[merkle_tree_pubkey],
        &[0u32],
        &[indexed_array_pubkey],
        &[merkle_tree_pubkey],
        &root_indices,
        &Vec::new(),
        &proof,
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
    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[merkle_tree_pubkey],
        &[1u32],
        &[indexed_array_pubkey],
        &[merkle_tree_pubkey],
        &root_indices,
        &Vec::new(),
        &proof,
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
#[tokio::test]
async fn test_with_address() {
    let env: light_test_utils::test_env::EnvWithAccounts =
        setup_test_programs_with_accounts().await;
    let mut context = env.context;
    let payer = context.payer.insecure_clone();

    let payer_pubkey = payer.pubkey();

    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let indexed_array_pubkey = env.indexed_array_pubkey;

    let mut mock_indexer = init_mock_indexer().await;

    let address_seed = [1u8; 32];
    let derived_address = derive_address(&env.address_merkle_tree_pubkey, &address_seed).unwrap();
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: payer_pubkey,
        data: None,
        address: Some(derived_address), // this should not be sent, only derived on-chain
    }];
    let proof_mock = CompressedProof {
        a: [0u8; 32],
        b: [0u8; 64],
        c: [0u8; 32],
    };

    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &Vec::new(),
        &output_compressed_accounts,
        &Vec::new(),
        &Vec::new(),
        &Vec::new(),
        &[merkle_tree_pubkey],
        &Vec::new(),
        &Vec::new(),
        &proof_mock,
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
    assert_eq!(
        res.result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(ErrorCode::InvalidAddress.into())
        ))
    );
    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &Vec::new(),
        &output_compressed_accounts,
        &Vec::new(),
        &Vec::new(),
        &Vec::new(),
        &[merkle_tree_pubkey],
        &Vec::new(),
        &vec![NewAddressParams {
            address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
            address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
            seed: address_seed,
            address_merkle_tree_root_index: 0,
        }],
        &proof_mock,
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
    .await;

    mock_indexer.add_lamport_compressed_accounts(
        res.unwrap()
            .metadata
            .unwrap()
            .return_data
            .unwrap()
            .data
            .to_vec(),
    );
    assert_eq!(mock_indexer.compressed_accounts.len(), 1);
    assert_eq!(
        mock_indexer.compressed_accounts[0]
            .compressed_account
            .address
            .unwrap(),
        derived_address
    );
    let compressed_account_with_context = mock_indexer.compressed_accounts[0].clone();
    let (root_indices, proof) = mock_indexer
        .create_proof_for_compressed_accounts(
            &[compressed_account_with_context
                .compressed_account
                .hash(
                    &merkle_tree_pubkey,
                    &compressed_account_with_context.leaf_index,
                )
                .unwrap()],
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
    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[merkle_tree_pubkey],
        &[0u32],
        &[indexed_array_pubkey],
        &[merkle_tree_pubkey],
        &root_indices,
        &Vec::new(),
        &proof,
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
    println!("Transaction with zkp -------------------------");

    let res = solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await;
    mock_indexer.add_lamport_compressed_accounts(
        res.unwrap()
            .metadata
            .unwrap()
            .return_data
            .unwrap()
            .data
            .to_vec(),
    );
    assert_eq!(mock_indexer.compressed_accounts.len(), 1);
    assert_eq!(
        mock_indexer.compressed_accounts[0]
            .compressed_account
            .address
            .unwrap(),
        derived_address
    );
    assert_eq!(
        mock_indexer.compressed_accounts[0].compressed_account.owner,
        recipient_pubkey
    );
}
use anchor_lang::{InstructionData, ToAccountMetas};
use circuitlib_rs::gnark::helpers::ProofType;

#[tokio::test]
async fn test_with_compression() {
    let env: light_test_utils::test_env::EnvWithAccounts =
        setup_test_programs_with_accounts().await;
    let mut context = env.context;
    let payer = context.payer.insecure_clone();

    let payer_pubkey = payer.pubkey();

    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let indexed_array_pubkey = env.indexed_array_pubkey;
    let mock_indexer = MockIndexer::new(
        merkle_tree_pubkey,
        indexed_array_pubkey,
        payer.insecure_clone(),
    );
    let instruction_data = psp_compressed_pda::instruction::InitCompressSolPda {};
    let accounts = psp_compressed_pda::accounts::InitializeCompressedSolPda {
        fee_payer: payer.pubkey(),
        compressed_sol_pda: get_compressed_sol_pda(),
        system_program: anchor_lang::solana_program::system_program::ID,
    };
    let instruction = Instruction {
        program_id: psp_compressed_pda::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    };
    create_and_send_transaction(&mut context, &[instruction], &payer_pubkey, &[&payer])
        .await
        .unwrap();

    let compress_amount = 1_000_000;
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: compress_amount,
        owner: payer_pubkey,
        data: None,
        address: None, // this should not be sent, only derived on-chain
    }];
    let proof_mock = CompressedProof {
        a: [0u8; 32],
        b: [0u8; 64],
        c: [0u8; 32],
    };

    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &Vec::new(),
        &output_compressed_accounts,
        &Vec::new(),
        &Vec::new(),
        &Vec::new(),
        &[merkle_tree_pubkey],
        &Vec::new(),
        &Vec::new(),
        &proof_mock,
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
    assert_eq!(
        res.result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(ErrorCode::ComputeOutputSumFailed.into())
        ))
    );
    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &Vec::new(),
        &output_compressed_accounts,
        &Vec::new(),
        &Vec::new(),
        &Vec::new(),
        &[merkle_tree_pubkey],
        &Vec::new(),
        &Vec::new(),
        &proof_mock,
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
    assert_eq!(
        res.result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(ErrorCode::ComputeOutputSumFailed.into())
        ))
    );

    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &Vec::new(),
        &output_compressed_accounts,
        &Vec::new(),
        &Vec::new(),
        &Vec::new(),
        &[merkle_tree_pubkey],
        &Vec::new(),
        &Vec::new(),
        &proof_mock,
        Some(compress_amount),
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

    let compressed_sol_pda_balance = context
        .banks_client
        .get_account(get_compressed_sol_pda())
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let rent = context.banks_client.get_rent().await.unwrap();
    let rent = rent.minimum_balance(CompressedSolPda::LEN);
    assert_eq!(
        compressed_sol_pda_balance,
        compress_amount + rent,
        "balance of compressed sol pda insufficient, compress sol failed"
    );

    // Wait until now to reduce startup lag by prover server
    let mut mock_indexer = mock_indexer.await;
    mock_indexer
        .add_lamport_compressed_accounts(res.metadata.unwrap().return_data.unwrap().data.to_vec());
    assert_eq!(mock_indexer.compressed_accounts.len(), 1);
    assert_eq!(
        mock_indexer.compressed_accounts[0]
            .compressed_account
            .address,
        None
    );
    let compressed_account_with_context = mock_indexer.compressed_accounts[0].clone();
    let (root_indices, proof) = mock_indexer
        .create_proof_for_compressed_accounts(
            &[compressed_account_with_context
                .compressed_account
                .hash(
                    &merkle_tree_pubkey,
                    &compressed_account_with_context.leaf_index,
                )
                .unwrap()],
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
    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[merkle_tree_pubkey],
        &[0u32],
        &[indexed_array_pubkey],
        &[merkle_tree_pubkey],
        &root_indices,
        &Vec::new(),
        &proof,
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

    assert_eq!(
        res.result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(ErrorCode::SumCheckFailed.into())
        ))
    );

    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[merkle_tree_pubkey],
        &[0u32],
        &[indexed_array_pubkey],
        &[merkle_tree_pubkey],
        &root_indices,
        &Vec::new(),
        &proof,
        Some(compress_amount),
        false,
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
    .await;
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
    mock_indexer.add_lamport_compressed_accounts(
        res.unwrap()
            .metadata
            .unwrap()
            .return_data
            .unwrap()
            .data
            .to_vec(),
    );
    assert_eq!(mock_indexer.compressed_accounts.len(), 1);
    assert_eq!(
        mock_indexer.compressed_accounts[0]
            .compressed_account
            .address,
        None
    );
    assert_eq!(
        mock_indexer.compressed_accounts[0].compressed_account.owner,
        recipient_pubkey
    );
}

#[ignore = "this is a helper function to regenerate accounts"]
#[tokio::test]
async fn regenerate_accounts() {
    let output_dir = "../../cli/accounts/";
    let env = setup_test_programs_with_accounts().await;
    let mut context = env.context;

    // List of public keys to fetch and export
    let pubkeys = vec![
        ("merkle_tree_pubkey", env.merkle_tree_pubkey),
        ("indexed_array_pubkey", env.indexed_array_pubkey),
        ("governance_authority_pda", env.governance_authority_pda),
        ("group_pda", env.group_pda),
        ("registered_program_pda", env.registered_program_pda),
        // ("address_merkle_tree", env.address_merkle_tree_pubkey),
        (
            "address_merkle_tree_queue",
            env.address_merkle_tree_queue_pubkey,
        ),
    ];

    for (name, pubkey) in pubkeys {
        // Fetch account data. Adjust this part to match how you retrieve and structure your account data.
        let account = context.banks_client.get_account(pubkey).await.unwrap();
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

#[derive(Debug)]
pub struct MockIndexer {
    pub merkle_tree_pubkey: Pubkey,
    pub indexed_array_pubkey: Pubkey,
    pub payer: Keypair,
    pub compressed_accounts: Vec<CompressedAccountWithMerkleContext>,
    pub nullified_compressed_accounts: Vec<CompressedAccountWithMerkleContext>,
    pub events: Vec<PublicTransactionEvent>,
    pub merkle_tree: light_merkle_tree_reference::MerkleTree<light_hasher::Poseidon>,
}

impl MockIndexer {
    async fn new(merkle_tree_pubkey: Pubkey, indexed_array_pubkey: Pubkey, payer: Keypair) -> Self {
        spawn_gnark_server(
            "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
            true,
            ProofType::Inclusion,
        )
        .await;

        let merkle_tree = light_merkle_tree_reference::MerkleTree::<light_hasher::Poseidon>::new(
            STATE_MERKLE_TREE_HEIGHT as usize,
            STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
        );

        Self {
            merkle_tree_pubkey,
            indexed_array_pubkey,
            payer,
            compressed_accounts: vec![],
            nullified_compressed_accounts: vec![],
            events: vec![],
            merkle_tree,
        }
    }

    pub async fn create_proof_for_compressed_accounts(
        &mut self,
        compressed_accounts: &[[u8; 32]],
        context: &mut ProgramTestContext,
    ) -> (Vec<u16>, CompressedProof) {
        let client = Client::new();

        let mut inclusion_proofs = Vec::<InclusionMerkleProofInputs>::new();
        for compressed_account in compressed_accounts.iter() {
            let leaf_index = self.merkle_tree.get_leaf_index(compressed_account).unwrap();
            let proof = self
                .merkle_tree
                .get_proof_of_leaf(leaf_index, true)
                .unwrap();
            inclusion_proofs.push(InclusionMerkleProofInputs {
                roots: BigInt::from_be_bytes(self.merkle_tree.root().as_slice()),
                leaves: BigInt::from_be_bytes(compressed_account),
                in_path_indices: BigInt::from_be_bytes(leaf_index.to_be_bytes().as_slice()), // leaf_index as u32,
                in_path_elements: proof.iter().map(|x| BigInt::from_be_bytes(x)).collect(),
            });
        }
        let inclusion_proof_inputs = InclusionProofInputs(inclusion_proofs.as_slice());
        let json_payload =
            InclusionJsonStruct::from_inclusion_proof_inputs(&inclusion_proof_inputs).to_string();

        let response_result = client
            .post(&format!("{}{}", SERVER_ADDRESS, INCLUSION_PATH))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(json_payload)
            .send()
            .await
            .expect("Failed to execute request.");
        assert!(response_result.status().is_success());
        let body = response_result.text().await.unwrap();
        let proof_json = deserialize_gnark_proof_json(&body).unwrap();
        let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
        let (proof_a, proof_b, proof_c) = compress_proof(&proof_a, &proof_b, &proof_c);

        let merkle_tree_account =
            AccountZeroCopy::<StateMerkleTreeAccount>::new(context, self.merkle_tree_pubkey).await;
        let merkle_tree = merkle_tree_account
            .deserialized()
            .copy_merkle_tree()
            .unwrap();
        assert_eq!(
            self.merkle_tree.root(),
            merkle_tree.root().unwrap(),
            "Local Merkle tree root is not equal to latest on-chain root"
        );

        let root_indices: Vec<u16> =
            vec![merkle_tree.current_root_index as u16; compressed_accounts.len()];
        (
            root_indices,
            CompressedProof {
                a: proof_a,
                b: proof_b,
                c: proof_c,
            },
        )
    }

    /// deserializes an event
    /// adds the output_compressed_accounts to the compressed_accounts
    /// removes the input_compressed_accounts from the compressed_accounts
    /// adds the input_compressed_accounts to the nullified_compressed_accounts
    pub fn add_lamport_compressed_accounts(&mut self, event_bytes: Vec<u8>) {
        let event_bytes = event_bytes.clone();
        let event = PublicTransactionEvent::deserialize(&mut event_bytes.as_slice()).unwrap();
        self.add_event_and_compressed_accounts(event);
    }

    pub fn add_event_and_compressed_accounts(
        &mut self,
        event: PublicTransactionEvent,
    ) -> Vec<usize> {
        for compressed_account in event.input_compressed_accounts.iter() {
            let index = self
                .compressed_accounts
                .iter()
                .position(|x| x.compressed_account == compressed_account.compressed_account)
                .expect("compressed_account not found");
            self.compressed_accounts.remove(index);
            // TODO: nullify compressed_account in Merkle tree, not implemented yet
            self.nullified_compressed_accounts
                .push(compressed_account.clone());
        }
        let mut indices = Vec::with_capacity(event.output_compressed_accounts.len());
        for (i, compressed_account) in event.output_compressed_accounts.iter().enumerate() {
            self.compressed_accounts
                .push(CompressedAccountWithMerkleContext {
                    compressed_account: compressed_account.clone(),
                    leaf_index: event.output_leaf_indices[i],
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 0,
                });
            indices.push(self.compressed_accounts.len() - 1);
            self.merkle_tree
                .append(
                    &compressed_account
                        .hash(&self.merkle_tree_pubkey, &event.output_leaf_indices[i])
                        .unwrap(),
                )
                .expect("insert failed");
        }

        self.events.push(event);
        indices
    }

    /// Check compressed_accounts in the queue array which are not nullified yet
    /// Iterate over these compressed_accounts and nullify them
    pub async fn nullify_compressed_accounts(&mut self, context: &mut ProgramTestContext) {
        let indexed_array = unsafe {
            get_hash_set::<u16, account_compression::IndexedArrayAccount>(
                context,
                self.indexed_array_pubkey,
            )
            .await
        };
        let merkle_tree_account =
            AccountZeroCopy::<StateMerkleTreeAccount>::new(context, self.merkle_tree_pubkey).await;
        let merkle_tree = merkle_tree_account
            .deserialized()
            .copy_merkle_tree()
            .unwrap();
        let change_log_index = merkle_tree.current_changelog_index as u64;

        let mut compressed_accounts_to_nullify = Vec::new();

        for (i, element) in indexed_array.iter() {
            if element.sequence_number().is_none() {
                compressed_accounts_to_nullify.push((i, element.value_bytes()));
            }
        }

        for (index_in_indexed_array, compressed_account) in compressed_accounts_to_nullify.iter() {
            let leaf_index = self
                .merkle_tree
                .get_leaf_index(&compressed_account)
                .unwrap();
            let proof: Vec<[u8; 32]> = self
                .merkle_tree
                .get_proof_of_leaf(leaf_index, false)
                .unwrap()
                .to_array::<16>()
                .unwrap()
                .to_vec();

            let instructions = [
                account_compression::nullify_leaves::sdk_nullify::create_nullify_instruction(
                    vec![change_log_index].as_slice(),
                    vec![(*index_in_indexed_array) as u16].as_slice(),
                    vec![0u64].as_slice(),
                    vec![proof].as_slice(),
                    &context.payer.pubkey(),
                    &self.merkle_tree_pubkey,
                    &self.indexed_array_pubkey,
                ),
            ];

            create_and_send_transaction(
                context,
                &instructions,
                &self.payer.pubkey(),
                &[&self.payer],
            )
            .await
            .unwrap();
            let indexed_array = unsafe {
                get_hash_set::<u16, account_compression::IndexedArrayAccount>(
                    context,
                    self.indexed_array_pubkey,
                )
                .await
            };
            let array_element = indexed_array
                .by_value_index(*index_in_indexed_array, Some(merkle_tree.sequence_number))
                .unwrap();
            assert_eq!(&array_element.value_bytes(), compressed_account);
            let merkle_tree_account =
                AccountZeroCopy::<StateMerkleTreeAccount>::new(context, self.merkle_tree_pubkey)
                    .await;
            assert_eq!(
                array_element.sequence_number(),
                Some(
                    merkle_tree_account
                        .deserialized()
                        .load_merkle_tree()
                        .unwrap()
                        .sequence_number
                        + STATE_MERKLE_TREE_ROOTS as usize
                )
            );
        }
    }
}

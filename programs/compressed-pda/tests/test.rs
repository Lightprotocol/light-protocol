#![cfg(feature = "test-sbf")]

use std::{assert_eq, println, vec::Vec};

use account_compression::{
    utils::constants::{STATE_MERKLE_TREE_HEIGHT, STATE_MERKLE_TREE_ROOTS},
    StateMerkleTreeAccount,
};
use anchor_lang::AnchorDeserialize;
use circuitlib_rs::{
    gnark::{
        constants::{INCLUSION_PATH, SERVER_ADDRESS},
        helpers::{health_check, kill_gnark_server, spawn_gnark_server},
        inclusion_json_formatter::InclusionJsonStruct,
        proof_helpers::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
    },
    inclusion::merkle_inclusion_proof_inputs::{InclusionMerkleProofInputs, InclusionProofInputs},
};
use light_test_utils::{
    create_and_send_transaction, test_env::setup_test_programs_with_accounts, AccountZeroCopy,
};
use num_bigint::BigInt;
use num_traits::ops::bytes::FromBytes;
use psp_compressed_pda::{
    event::PublicTransactionEvent,
    sdk::{create_execute_compressed_instruction, create_execute_compressed_opt_instruction},
    utils::CompressedProof,
    utxo::{OutUtxo, Utxo},
};
use reqwest::Client;
use solana_cli_output::CliAccount;
use solana_program_test::ProgramTestContext;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};
use tokio::fs::write as async_write;

// TODO: use lazy_static to spawn the server once

/// Tests Execute compressed transaction:
/// 1. should succeed: with out utxo(0 lamports), no in utxo
/// 2. should fail: in utxo and invalid zkp
/// 3. should fail: in utxo and invalid signer
/// 4. should succeed: in utxo inserted in (1.) and valid zkp
#[tokio::test]
async fn test_execute_compressed_transaction() {
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
        None,
    );
    let in_utxos = vec![Utxo {
        lamports: 0,
        owner: payer_pubkey,
        blinding: [1u8; 32],
        data: None,
        address: None,
    }];

    let out_utxos = vec![OutUtxo {
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
        &out_utxos,
        &Vec::new(),
        &Vec::new(),
        &vec![merkle_tree_pubkey],
        &vec![0u16],
        &proof_mock,
    );

    // TODO: add function to create_send_transaction_update_indexer
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.last_blockhash,
    );
    let res = solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await;
    // Wait until now to reduce startup lag by prover server
    let mut mock_indexer = mock_indexer.await;
    mock_indexer.add_lamport_utxos(
        res.unwrap()
            .metadata
            .unwrap()
            .return_data
            .unwrap()
            .data
            .to_vec(),
    );
    assert_eq!(mock_indexer.utxos.len(), 1);
    // TODO: assert all utxo properties
    // check invalid proof
    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &in_utxos,
        &out_utxos,
        &vec![merkle_tree_pubkey],
        &vec![indexed_array_pubkey],
        &vec![merkle_tree_pubkey],
        &vec![0u16],
        &proof_mock,
    );

    let res =
        create_and_send_transaction(&mut context, &[instruction], &payer_pubkey, &[&payer]).await;
    assert!(res.is_err());

    // check invalid signer for in utxo
    let invalid_signer_utxos = vec![Utxo {
        lamports: 0,
        owner: Pubkey::new_unique(),
        blinding: [1u8; 32],
        data: None,
        address: None,
    }];

    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &invalid_signer_utxos,
        &out_utxos,
        &vec![merkle_tree_pubkey],
        &vec![indexed_array_pubkey],
        &vec![merkle_tree_pubkey],
        &vec![0u16],
        &proof_mock,
    );

    let res =
        create_and_send_transaction(&mut context, &[instruction], &payer.pubkey(), &[&payer]).await;
    assert!(res.is_err());

    // create Merkle proof
    // get zkp from server
    // create instruction as usual with correct zkp
    let in_utxo = mock_indexer.utxos[0].clone();
    let (root_indices, proof) = mock_indexer
        .create_proof_for_utxos(&[in_utxo.hash()], &mut context)
        .await;
    let mut in_utxos = vec![in_utxo];
    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &in_utxos,
        &out_utxos,
        &vec![merkle_tree_pubkey],
        &vec![indexed_array_pubkey],
        &vec![merkle_tree_pubkey],
        &root_indices,
        &proof,
    );
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.last_blockhash,
    );
    println!("Transaction with zkp -------------------------");

    let res = solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await;
    mock_indexer.add_lamport_utxos(
        res.unwrap()
            .metadata
            .unwrap()
            .return_data
            .unwrap()
            .data
            .to_vec(),
    );

    println!("Double spend -------------------------");
    let out_utxos = vec![OutUtxo {
        lamports: 0,
        owner: Pubkey::new_unique(),
        data: None,
        address: None,
    }];
    // double spend
    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &in_utxos,
        &out_utxos,
        &vec![merkle_tree_pubkey],
        &vec![indexed_array_pubkey],
        &vec![merkle_tree_pubkey],
        &root_indices,
        &proof,
    );
    let res =
        create_and_send_transaction(&mut context, &[instruction], &payer.pubkey(), &[&payer]).await;
    assert!(res.is_err());
    let out_utxos = vec![OutUtxo {
        lamports: 0,
        owner: Pubkey::new_unique(),
        data: None,
        address: None,
    }];
    in_utxos[0].blinding = [6u8; 32];
    // invalid utxo
    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &in_utxos,
        &out_utxos,
        &vec![merkle_tree_pubkey],
        &vec![indexed_array_pubkey],
        &vec![merkle_tree_pubkey],
        &root_indices,
        &proof,
    );
    let res =
        create_and_send_transaction(&mut context, &[instruction], &payer.pubkey(), &[&payer]).await;
    assert!(res.is_err());
}

#[ignore = "currently not used, todo maintain"]
#[tokio::test]
async fn test_create_execute_compressed_transaction_2() {
    let env: light_test_utils::test_env::EnvWithAccounts =
        setup_test_programs_with_accounts().await;
    let mut context = env.context;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let indexed_array_pubkey = env.indexed_array_pubkey;
    let mut in_utxo = Utxo {
        lamports: 0,
        owner: payer_pubkey,
        blinding: [0u8; 32],
        data: None,
        address: None,
    };
    in_utxo.update_blinding(merkle_tree_pubkey, 0).unwrap();

    let in_utxos = vec![in_utxo];

    let out_utxos = vec![OutUtxo {
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

    let instruction = create_execute_compressed_opt_instruction(
        &payer_pubkey,
        &in_utxos,
        &out_utxos,
        &vec![merkle_tree_pubkey],
        &vec![indexed_array_pubkey],
        &vec![merkle_tree_pubkey],
        &vec![0u32],
        &vec![0u16],
        &proof_mock,
    );

    create_and_send_transaction(&mut context, &[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    let invalid_signer_utxos = vec![Utxo {
        lamports: 0,
        owner: Pubkey::new_unique(),
        blinding: [1u8; 32],
        data: None,
        address: None,
    }];
    let instruction = create_execute_compressed_opt_instruction(
        &payer_pubkey,
        &invalid_signer_utxos,
        &out_utxos,
        &vec![merkle_tree_pubkey],
        &vec![indexed_array_pubkey],
        &vec![merkle_tree_pubkey],
        &vec![0u32],
        &vec![0u16],
        &proof_mock,
    );
    let res =
        create_and_send_transaction(&mut context, &[instruction], &payer.pubkey(), &[&payer]).await;
    assert!(res.is_err());
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
    pub utxos: Vec<Utxo>,
    pub nullified_utxos: Vec<Utxo>,
    // pub token_utxos: Vec<TokenUtxo>,
    // pub token_nullified_utxos: Vec<TokenUtxo>,
    pub events: Vec<PublicTransactionEvent>,
    pub merkle_tree: light_merkle_tree_reference::MerkleTree<light_hasher::Poseidon>,
    pub gnark_server: std::process::Child,
}

impl MockIndexer {
    async fn new(
        merkle_tree_pubkey: Pubkey,
        indexed_array_pubkey: Pubkey,
        payer: Keypair,
        startup_time: Option<u64>,
    ) -> Self {
        let gnark_server =
            spawn_gnark_server("../../circuit-lib/circuitlib-rs/scripts/prover.sh", 0);
        let merkle_tree = light_merkle_tree_reference::MerkleTree::<light_hasher::Poseidon>::new(
            STATE_MERKLE_TREE_HEIGHT,
            STATE_MERKLE_TREE_ROOTS,
        )
        .unwrap();
        if startup_time.is_some() {
            tokio::time::sleep(tokio::time::Duration::from_secs(startup_time.unwrap())).await;
        }
        Self {
            merkle_tree_pubkey,
            indexed_array_pubkey,
            payer,
            utxos: vec![],
            nullified_utxos: vec![],
            events: vec![],
            // token_utxos: vec![],
            // token_nullified_utxos: vec![],
            merkle_tree,
            gnark_server,
        }
    }
    pub fn kill_gnark_server(&mut self) {
        kill_gnark_server(&mut self.gnark_server);
    }

    pub async fn create_proof_for_utxos(
        &self,
        utxos: &[[u8; 32]],
        context: &mut ProgramTestContext,
    ) -> (Vec<u16>, CompressedProof) {
        // waiting for server to start
        health_check().await;

        let client = Client::new();

        let mut inclusion_proofs = Vec::<InclusionMerkleProofInputs>::new();
        for utxo in utxos.iter() {
            let leaf_index = self.merkle_tree.get_leaf_index(utxo).unwrap();
            let proof = self.merkle_tree.get_proof_of_leaf(leaf_index).unwrap();
            inclusion_proofs.push(InclusionMerkleProofInputs {
                root: BigInt::from_be_bytes(self.merkle_tree.root().unwrap().as_slice()),
                leaf: BigInt::from_be_bytes(utxo),
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

        let merkle_tree_account = light_test_utils::AccountZeroCopy::<StateMerkleTreeAccount>::new(
            context,
            self.merkle_tree_pubkey,
        )
        .await;
        let merkle_tree = merkle_tree_account
            .deserialized()
            .copy_merkle_tree()
            .unwrap();
        assert_eq!(
            self.merkle_tree.root().unwrap(),
            merkle_tree.root().unwrap(),
            "Local Merkle tree root is not equal to latest onchain root"
        );

        let root_indices: Vec<u16> = vec![merkle_tree.current_root_index as u16; utxos.len()];
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
    /// adds the out_utxos to the utxos
    /// removes the in_utxos from the utxos
    /// adds the in_utxos to the nullified_utxos
    pub fn add_lamport_utxos(&mut self, event_bytes: Vec<u8>) {
        let event_bytes = event_bytes.clone();
        let event = PublicTransactionEvent::deserialize(&mut event_bytes.as_slice()).unwrap();
        self.add_event_and_utxos(event);
    }

    pub fn add_event_and_utxos(&mut self, event: PublicTransactionEvent) -> Vec<usize> {
        for utxo in event.in_utxos.iter() {
            let index = self
                .utxos
                .iter()
                .position(|x| x == utxo)
                .expect("utxo not found");
            self.utxos.remove(index);
            // TODO: nullify utxo in Merkle tree, not implemented yet
            self.nullified_utxos.push(utxo.clone());
            // let index = self.utxos.iter().position(|x| x == utxo);
            // match index {
            //     Some(index) => {
            //         let token_utxo_element = self.token_utxos[index].clone();
            //         self.token_utxos.remove(index);
            //         self.token_nullified_utxos.push(token_utxo_element);
            //     }
            //     None => {}
            // }
        }
        let mut indices = Vec::with_capacity(event.out_utxos.len());
        for utxo in event.out_utxos.iter() {
            self.utxos.push(utxo.clone());
            indices.push(self.utxos.len() - 1);
            self.merkle_tree
                .append(&utxo.hash())
                .expect("insert failed");
        }

        self.events.push(event);
        indices
    }

    /// Check utxos in the queue array which are not nullified yet
    /// Iterate over these utxos and nullify them
    pub async fn nullify_utxos(&mut self, context: &mut ProgramTestContext) {
        let array = AccountZeroCopy::<account_compression::IndexedArrayAccount>::new(
            context,
            self.indexed_array_pubkey,
        )
        .await;
        let indexed_array = array.deserialized().indexed_array;
        let merkle_tree_account = light_test_utils::AccountZeroCopy::<StateMerkleTreeAccount>::new(
            context,
            self.merkle_tree_pubkey,
        )
        .await;
        let merkle_tree = merkle_tree_account
            .deserialized()
            .copy_merkle_tree()
            .unwrap();
        let change_log_index = merkle_tree.current_changelog_index as u64;

        let mut utxo_to_nullify = Vec::new();

        for (i, element) in indexed_array.iter().enumerate() {
            if element.merkle_tree_overwrite_sequence_number == 0 && element.element != [0u8; 32] {
                utxo_to_nullify.push((i, element));
            }
        }

        for (index_in_indexed_array, utxo) in utxo_to_nullify.iter() {
            let leaf_index = self.merkle_tree.get_leaf_index(&utxo.element).unwrap();
            let proof: Vec<[u8; 32]> = self
                .merkle_tree
                .get_proof_of_leaf(leaf_index)
                .unwrap()
                .to_array::<26>()
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
            let array = AccountZeroCopy::<account_compression::IndexedArrayAccount>::new(
                context,
                self.indexed_array_pubkey,
            )
            .await;
            let indexed_array = array.deserialized().indexed_array;
            assert_eq!(indexed_array[*index_in_indexed_array].element, utxo.element);
            let merkle_tree_account =
                light_test_utils::AccountZeroCopy::<StateMerkleTreeAccount>::new(
                    context,
                    self.merkle_tree_pubkey,
                )
                .await;
            assert_eq!(
                indexed_array[*index_in_indexed_array].merkle_tree_overwrite_sequence_number,
                merkle_tree_account
                    .deserialized()
                    .load_merkle_tree()
                    .unwrap()
                    .sequence_number as u64
                    + account_compression::utils::constants::STATE_MERKLE_TREE_ROOTS as u64
            );
        }
    }
}

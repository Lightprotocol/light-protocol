use light_hasher::{Hasher, Poseidon};
use light_merkle_tree_reference::MerkleTree;
use light_prover_client::batch_append::{calculate_hash_chain, get_batch_append_inputs};
use light_prover_client::batch_append_2::get_batch_append2_inputs;
use light_prover_client::batch_update::get_batch_update_inputs;
use light_prover_client::gnark::batch_append_2_json_formatter::BatchAppend2ProofInputsJson;
use light_prover_client::gnark::batch_append_json_formatter::append_inputs_string;
use light_prover_client::gnark::batch_update_json_formatter::update_inputs_string;
use light_prover_client::gnark::helpers::{spawn_prover, ProofType, ProverConfig};
use light_prover_client::{
    gnark::{
        constants::{PROVE_PATH, SERVER_ADDRESS},
        inclusion_json_formatter::inclusion_inputs_string,
    },
    helpers::init_logger,
};
use log::info;
use reqwest::Client;
use serial_test::serial;

#[tokio::test]
#[ignore]
async fn prove_inclusion_full() {
    init_logger();
    spawn_prover(
        false,
        ProverConfig {
            run_mode: None,
            circuits: vec![ProofType::Inclusion, { ProofType::BatchUpdateTest }],
        },
    )
    .await;
    let client = Client::new();
    for number_of_utxos in &[1, 2, 3, 4, 8] {
        let (inputs, _) = inclusion_inputs_string(*number_of_utxos as usize);
        let response_result = client
            .post(&format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(inputs)
            .send()
            .await
            .expect("Failed to execute request.");
        assert!(response_result.status().is_success());
    }
}

#[serial]
#[tokio::test]
async fn prove_inclusion() {
    init_logger();
    spawn_prover(
        true,
        ProverConfig {
            run_mode: None,
            circuits: vec![
                ProofType::Inclusion,
                ProofType::BatchUpdateTest,
                ProofType::BatchAppendTest,
            ],
        },
    )
    .await;
    let client = Client::new();
    let (inputs, _) = inclusion_inputs_string(1);
    let response_result = client
        .post(&format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(inputs)
        .send()
        .await
        .expect("Failed to execute request.");
    assert!(response_result.status().is_success());
}

#[serial]
#[tokio::test]
async fn prove_batch_update() {
    init_logger();
    spawn_prover(
        false,
        ProverConfig {
            run_mode: None,
            circuits: vec![ProofType::BatchUpdateTest],
        },
    )
    .await;
    const HEIGHT: usize = 26;
    const CANOPY: usize = 0;
    let num_insertions = 10;
    let tx_hash = [0u8; 32];

    info!("initializing merkle tree");
    let mut merkle_tree = MerkleTree::<Poseidon>::new(HEIGHT, CANOPY);
    for _ in 0..2 {
        let mut leaves = vec![];
        let mut old_leaves = vec![];
        let mut nullifiers = vec![];
        for i in 0..num_insertions {
            let mut bn: [u8; 32] = [0; 32];
            bn[31] = i as u8;
            let leaf: [u8; 32] = Poseidon::hash(&bn).unwrap();
            leaves.push(leaf);
            old_leaves.push(leaf);
            merkle_tree.append(&leaf).unwrap();

            let nullifier =
                Poseidon::hashv(&[&leaf, &(i as usize).to_be_bytes(), &tx_hash]).unwrap();
            nullifiers.push(nullifier);
        }

        let mut merkle_proofs = vec![];
        let mut path_indices = vec![];
        for index in 0..leaves.len() {
            let proof = merkle_tree.get_proof_of_leaf(index, true).unwrap();
            merkle_proofs.push(proof.to_vec());
            path_indices.push(index as u32);
        }
        let root = merkle_tree.root();
        let leaves_hashchain = calculate_hash_chain(&nullifiers);
        let inputs = get_batch_update_inputs::<HEIGHT>(
            root,
            vec![tx_hash; num_insertions],
            leaves,
            leaves_hashchain,
            old_leaves,
            merkle_proofs,
            path_indices,
            num_insertions as u32,
        );
        let client = Client::new();
        let inputs = update_inputs_string(&inputs);

        let response_result = client
            .post(&format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(inputs)
            .send()
            .await
            .expect("Failed to execute request.");

        let status = response_result.status();
        let body = response_result.text().await.unwrap();
        assert!(
            status.is_success(),
            "Batch append proof generation failed. Status: {}, Body: {}",
            status,
            body
        );
    }
}

#[serial]
#[tokio::test]
async fn prove_batch_append() {
    init_logger();
    println!("spawning prover");
    spawn_prover(
        true,
        ProverConfig {
            run_mode: None,
            circuits: vec![ProofType::BatchAppendTest],
        },
    )
    .await;
    println!("prover spawned");

    const HEIGHT: usize = 26;
    const CANOPY: usize = 0;
    let num_insertions = 10;

    // Do multiple rounds of batch appends
    for _ in 0..2 {
        info!("initializing merkle tree for append.");
        let merkle_tree = MerkleTree::<Poseidon>::new(HEIGHT, CANOPY);

        let old_subtrees = merkle_tree.get_subtrees();
        let mut leaves = vec![];

        // Create leaves for this batch append
        for i in 0..num_insertions {
            let mut bn: [u8; 32] = [0; 32];
            bn[31] = i as u8;
            let leaf: [u8; 32] = Poseidon::hash(&bn).unwrap();
            leaves.push(leaf);
        }

        let leaves_hashchain = calculate_hash_chain(&leaves);

        // Generate inputs for batch append operation
        let inputs = get_batch_append_inputs::<HEIGHT>(
            merkle_tree.layers[0].len(),
            old_subtrees.try_into().unwrap(),
            leaves,
            leaves_hashchain,
        );

        // Send proof request to the server
        let client = Client::new();
        let inputs = append_inputs_string(&inputs);
        let response_result = client
            .post(&format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(inputs)
            .send()
            .await
            .expect("Failed to execute request.");

        let status = response_result.status();
        let body = response_result.text().await.unwrap();
        assert!(
            status.is_success(),
            "Batch append proof generation failed. Status: {}, Body: {}",
            status,
            body
        );
    }
}

#[serial]
#[tokio::test]
async fn prove_batch_two_append() {
    init_logger();

    // Spawn the prover with specific configuration
    spawn_prover(
        false,
        ProverConfig {
            run_mode: None,
            circuits: vec![ProofType::BatchAppend2Test],
        },
    )
    .await;

    const HEIGHT: usize = 26;
    const CANOPY: usize = 0;
    let num_insertions = 10;
    info!("Initializing Merkle tree for append.");
    let mut merkle_tree = MerkleTree::<Poseidon>::new(HEIGHT, CANOPY);
    let mut current_index = 0;
    for i in 0..2 {
        let mut leaves = vec![];
        let mut old_leaves = vec![];

        // Create leaves and append them to the Merkle tree
        for i in 0..num_insertions {
            let mut bn: [u8; 32] = [0; 32];
            bn[31] = i as u8;
            let leaf: [u8; 32] = Poseidon::hash(&bn).unwrap();
            // assuming old leaves are all zero (not nullified)
            leaves.push(leaf);
            // Append nullifier or ero value
            if i % 2 == 0 {
                let nullifier = Poseidon::hashv(&[&leaf, &[0u8; 32]]).unwrap();
                merkle_tree.append(&nullifier).unwrap();
            } else {
                merkle_tree.append(&[0u8; 32]).unwrap();
            }
        }

        // Generate Merkle proofs and prepare path indices
        let mut merkle_proofs = vec![];
        for index in current_index..current_index + num_insertions {
            let proof = merkle_tree.get_proof_of_leaf(index, true).unwrap();
            let leaf = merkle_tree.get_leaf(index).unwrap();
            old_leaves.push(leaf);
            merkle_proofs.push(proof.to_vec());
        }

        // Retrieve tree root and compute leaves hash chain
        let root = merkle_tree.root();
        let leaves_hashchain = calculate_hash_chain(&leaves);

        // Generate inputs for BatchAppend2Circuit
        let inputs = get_batch_append2_inputs::<HEIGHT>(
            root,
            (i * num_insertions) as u32,
            leaves.clone(),
            leaves_hashchain,
            old_leaves.clone(),
            merkle_proofs.clone(),
            num_insertions as u32,
        );

        // Serialize inputs to JSON
        let client = Client::new();
        let inputs_json = BatchAppend2ProofInputsJson::from_inputs(&inputs).to_string();
        // Send proof request to server
        let response_result = client
            .post(&format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(inputs_json)
            .send()
            .await
            .expect("Failed to execute request.");

        let status = response_result.status();
        let body = response_result.text().await.unwrap();
        assert!(
            status.is_success(),
            "Batch append2 proof generation failed. Status: {}, Body: {}",
            status,
            body
        );
        current_index += num_insertions;
    }
}

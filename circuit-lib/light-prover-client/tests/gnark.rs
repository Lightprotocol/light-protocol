use light_hasher::{Hasher, Poseidon};
use light_merkle_tree_reference::MerkleTree;
use light_prover_client::batch_append::{calculate_hash_chain, get_batch_append_inputs};
use light_prover_client::batch_update::get_batch_update_inputs;
use light_prover_client::gnark::batch_append_json_formatter::append_inputs_string;
use light_prover_client::gnark::batch_update_json_formatter::update_inputs_string;
use light_prover_client::gnark::helpers::{spawn_prover, ProofType};
use light_prover_client::{
    gnark::{
        constants::{PROVE_PATH, SERVER_ADDRESS},
        inclusion_json_formatter::inclusion_inputs_string,
    },
    helpers::init_logger,
};
use log::info;
use reqwest::Client;

#[tokio::test]
#[ignore]
async fn prove_inclusion_full() {
    init_logger();
    spawn_prover(false, &[ProofType::Inclusion]).await;
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

#[tokio::test]
async fn test_inclusion_batch_append_batch_update() {
    init_logger();
    spawn_prover(
        false,
        &[
            ProofType::Inclusion,
            ProofType::BatchUpdate,
            ProofType::BatchAppend,
        ],
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

    spawn_prover(false, &[ProofType::BatchUpdate]).await;
    const HEIGHT: usize = 10;
    const CANOPY: usize = 0;
    let num_insertions = 10;

    info!("initializing merkle tree");
    let mut merkle_tree = MerkleTree::<Poseidon>::new(HEIGHT, CANOPY);

    let mut leaves = vec![];
    for i in 0..num_insertions {
        let mut bn: [u8; 32] = [0; 32];
        bn[31] = i as u8;
        let leaf: [u8; 32] = Poseidon::hash(&bn).unwrap();
        leaves.push(leaf);
        merkle_tree.append(&leaf).unwrap();
    }

    let mut merkle_proofs = vec![];
    let mut path_indices = vec![];
    for index in 0..leaves.len() {
        let proof = merkle_tree.get_proof_of_leaf(index, true).unwrap();
        merkle_proofs.push(proof.to_vec());
        path_indices.push(index as u32);
    }
    let root = merkle_tree.root();
    let leaves_hashchain = calculate_hash_chain(&leaves);
    let inputs = get_batch_update_inputs::<HEIGHT>(
        root,
        leaves,
        leaves_hashchain,
        merkle_proofs,
        path_indices,
        num_insertions,
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
    assert!(response_result.status().is_success());

    let num_insertions = 10;

    info!("initializing merkle tree");
    let merkle_tree = MerkleTree::<Poseidon>::new(HEIGHT, CANOPY);

    let old_subtrees = merkle_tree.get_subtrees();
    let mut leaves = vec![];
    for i in 0..num_insertions {
        let mut bn: [u8; 32] = [0; 32];
        bn[31] = i as u8;
        let leaf: [u8; 32] = Poseidon::hash(&bn).unwrap();
        leaves.push(leaf);
    }

    let leaves_hashchain = calculate_hash_chain(&leaves);
    let inputs = get_batch_append_inputs::<HEIGHT>(
        merkle_tree.layers[0].len(),
        old_subtrees.try_into().unwrap(),
        leaves,
        leaves_hashchain,
    );
    let client = Client::new();
    let inputs = append_inputs_string(&inputs);
    let response_result = client
        .post(&format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(inputs)
        .send()
        .await
        .expect("Failed to execute request.");
    assert!(response_result.status().is_success());
}

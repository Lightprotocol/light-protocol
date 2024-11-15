use light_hasher::{Hasher, Poseidon};
use light_merkle_tree_reference::MerkleTree;
use light_prover_client::batch_append_with_proofs::get_batch_append_with_proofs_inputs;
use light_prover_client::batch_append_with_subtrees::{
    calculate_hash_chain, get_batch_append_with_subtrees_inputs,
};
use light_prover_client::batch_update::get_batch_update_inputs;
use light_prover_client::gnark::batch_append_with_proofs_json_formatter::BatchAppendWithProofsInputsJson;
use light_prover_client::gnark::batch_append_with_subtrees_json_formatter::append_inputs_string;
use light_prover_client::gnark::batch_update_json_formatter::update_inputs_string;
use light_prover_client::{
    batch_address_append::{get_batch_address_append_inputs_from_tree, get_test_batch_address_append_inputs},
    gnark::batch_address_append_json_formatter::to_json,
};

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
use num_bigint::ToBigUint;
use light_utils::bigint::bigint_to_be_bytes_array;

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
            circuits: vec![ProofType::Inclusion],
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
        true,
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
            circuits: vec![ProofType::BatchAppendWithSubtreesTest],
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
        let inputs = get_batch_append_with_subtrees_inputs::<HEIGHT>(
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
        true,
        ProverConfig {
            run_mode: None,
            circuits: vec![ProofType::BatchAppendWithProofsTest],
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
            let leaf = merkle_tree.get_leaf(index);
            old_leaves.push(leaf);
            merkle_proofs.push(proof.to_vec());
        }

        // Retrieve tree root and compute leaves hash chain
        let root = merkle_tree.root();
        let leaves_hashchain = calculate_hash_chain(&leaves);

        // Generate inputs for BatchAppendWithProofsCircuit
        let inputs = get_batch_append_with_proofs_inputs::<HEIGHT>(
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
        let inputs_json = BatchAppendWithProofsInputsJson::from_inputs(&inputs).to_string();
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
            "Batch append proof generation failed. Status: {}, Body: {}",
            status,
            body
        );
        current_index += num_insertions;
    }
}

#[test]
pub fn print_circuit_test_data_json_formatted() {
    let addresses = vec![31_u32.to_biguint().unwrap(), 30_u32.to_biguint().unwrap()];
    let start_index = 2;
    let tree_height = 4;
    
    let inputs = get_test_batch_address_append_inputs(
        addresses,
        start_index,
        tree_height,
    );

    let json_output = to_json(&inputs);
    println!("{}", json_output);
}

#[test]
pub fn print_circuit_test_data_with_existing_tree() {
    use light_indexed_merkle_tree::{array::IndexedArray, reference::IndexedMerkleTree};
    use light_hasher::Poseidon;

    const TREE_HEIGHT: usize = 4;

    let new_element_values = vec![31_u32.to_biguint().unwrap(), 30_u32.to_biguint().unwrap()];

    let mut relayer_indexing_array = IndexedArray::<Poseidon, usize>::default();
    relayer_indexing_array.init().unwrap();
    let mut relayer_merkle_tree = IndexedMerkleTree::<Poseidon, usize>::new(TREE_HEIGHT, 0).unwrap();
    relayer_merkle_tree.init().unwrap();

    let start_index = relayer_merkle_tree.merkle_tree.rightmost_index;

    let current_root = relayer_merkle_tree.root();
    
    let mut low_element_values = Vec::new();
    let mut low_element_indices = Vec::new();
    let mut low_element_next_indices = Vec::new();
    let mut low_element_next_values = Vec::new();
    let mut low_element_proofs:  Vec<Vec<[u8; 32]>> = Vec::new();

    for new_element_value in &new_element_values {
        let non_inclusion_proof = relayer_merkle_tree
            .get_non_inclusion_proof(new_element_value, &relayer_indexing_array)
            .unwrap();

        low_element_values.push(non_inclusion_proof.leaf_lower_range_value);
        low_element_indices.push(non_inclusion_proof.leaf_index);
        low_element_next_indices.push(non_inclusion_proof.next_index);
        low_element_next_values.push(non_inclusion_proof.leaf_higher_range_value);
    
        low_element_proofs.push(non_inclusion_proof.merkle_proof.as_slice().to_vec());
    }

    let new_element_values = new_element_values.iter().map(|v|bigint_to_be_bytes_array::<32>(&v).unwrap()).collect();

    let inputs = get_batch_address_append_inputs_from_tree::<TREE_HEIGHT>(
        start_index,
        current_root,
        low_element_values,
        low_element_next_values,
        low_element_indices,
        low_element_next_indices,
        low_element_proofs,
        new_element_values,
        relayer_merkle_tree.merkle_tree.get_subtrees().try_into().unwrap(),
    );

    let json_output = to_json(&inputs);

    let reference_output = r#"{
  "BatchSize": 2,
  "HashchainHash": "0x1e94e9fed8440d50ff872bedcc6a6c460f9c6688ac167f68e288057e63109410",
  "LowElementIndices": [
    "0x0",
    "0x0"
  ],
  "LowElementNextIndices": [
    "0x1",
    "0x2"
  ],
  "LowElementNextValues": [
    "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    "0x1f"
  ],
  "LowElementProofs": [
    [
      "0x1ea416eeb40218b540c1cfb8dbe91f6d54e8a29edc30a39e326b4057a7d963f5",
      "0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864",
      "0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1",
      "0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238"
    ],
    [
      "0x1ea416eeb40218b540c1cfb8dbe91f6d54e8a29edc30a39e326b4057a7d963f5",
      "0x864f3eb12bb83a5cdc9ff6fdc8b985aa4b87292c5eef49201065277170e8c51",
      "0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1",
      "0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238"
    ]
  ],
  "LowElementValues": [
    "0x0",
    "0x0"
  ],
  "NewElementProofs": [
    [
      "0x0",
      "0x2cfd59ee6c304f7f1e82d9e7e857a380e991fb02728f09324baffef2807e74fa",
      "0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1",
      "0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238"
    ],
    [
      "0x29794d28dddbdb020ec3974ecc41bcf64fb695eb222bde71f2a130e92852c0eb",
      "0x15920e98b921491171b9b2b0a8ac1545e10b58e9c058822b6de9f4179bbd2e7c",
      "0x1069673dcdb12263df301a6ff584a7ec261a44cb9dc68df067a4774460b1f1e1",
      "0x18f43331537ee2af2e3d758d50f72106467c6eea50371dd528d57eb2b856d238"
    ]
  ],
  "NewElementValues": [
    "0x1f",
    "0x1e"
  ],
  "NewRoot": "0x2a62d5241a6d3659df612b996ad729abe32f425bfec249f060983013ba2cfdb8",
  "OldRoot": "0x909e8762fb09c626001b19f6441a2cd2da21b1622c6970ec9c4863ec9c09855",
  "PublicInputHash": "0x31a64ce5adc664d1092fd7353a76b4fe0a3e63ad0cf313d66a6bc89e5e4a840",
  "StartIndex": 2,
  "TreeHeight": 4
}"#;

    println!("{}", json_output);

    assert_eq!(json_output, reference_output);
}
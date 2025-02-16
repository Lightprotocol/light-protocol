use light_batched_merkle_tree::constants::{
    DEFAULT_BATCH_ADDRESS_TREE_HEIGHT, DEFAULT_BATCH_STATE_TREE_HEIGHT,
};
use light_compressed_account::{
    bigint::bigint_to_be_bytes_array, hash_chain::create_hash_chain_from_slice,
};
use light_hasher::{Hasher, Poseidon};
use light_merkle_tree_reference::MerkleTree;
use light_prover_client::{
    batch_address_append::{
        get_batch_address_append_circuit_inputs, get_test_batch_address_append_inputs,
    },
    batch_append_with_proofs::get_batch_append_with_proofs_inputs,
    batch_update::get_batch_update_inputs,
    gnark::{
        batch_address_append_json_formatter::to_json,
        batch_append_with_proofs_json_formatter::BatchAppendWithProofsInputsJson,
        batch_update_json_formatter::update_inputs_string,
        combined_json_formatter::combined_inputs_string,
        combined_json_formatter_legacy::combined_inputs_string as combined_inputs_string_legacy,
        constants::{PROVE_PATH, SERVER_ADDRESS},
        helpers::{spawn_prover, ProofType, ProverConfig},
        inclusion_json_formatter::inclusion_inputs_string,
        inclusion_json_formatter_legacy,
        non_inclusion_json_formatter_legacy::non_inclusion_inputs_string,
    },
    helpers::init_logger,
};
use log::info;
use num_bigint::ToBigUint;
use reqwest::Client;
use serial_test::serial;

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
    for number_of_utxos in &[1, 2, 3, 4, 8] {
        let inputs = inclusion_inputs_string(*number_of_utxos as usize);
        let response_result = client
            .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(inputs)
            .send()
            .await
            .expect("Failed to execute request.");
        assert!(response_result.status().is_success());
    }

    // legacy height 26
    {
        for number_of_utxos in &[1, 2, 3, 4, 8] {
            let inputs =
                inclusion_json_formatter_legacy::inclusion_inputs_string(*number_of_utxos as usize);
            let response_result = client
                .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
                .header("Content-Type", "text/plain; charset=utf-8")
                .body(inputs)
                .send()
                .await
                .expect("Failed to execute request.");
            assert!(response_result.status().is_success());
        }
    }
}

#[serial]
#[tokio::test]
async fn prove_combined() {
    init_logger();
    spawn_prover(
        true,
        ProverConfig {
            run_mode: None,
            circuits: vec![ProofType::Combined],
        },
    )
    .await;
    let client = Client::new();
    {
        for i in 1..=4 {
            for non_i in 1..=2 {
                let inputs = combined_inputs_string_legacy(i, non_i);
                let response_result = client
                    .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
                    .header("Content-Type", "text/plain; charset=utf-8")
                    .body(inputs)
                    .send()
                    .await
                    .expect("Failed to execute request.");
                assert!(response_result.status().is_success());
            }
        }
    }
    {
        for i in 1..=4 {
            for non_i in 1..=2 {
                let inputs = combined_inputs_string(i, non_i);
                let response_result = client
                    .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
                    .header("Content-Type", "text/plain; charset=utf-8")
                    .body(inputs)
                    .send()
                    .await
                    .expect("Failed to execute request.");
                assert!(response_result.status().is_success());
            }
        }
    }
}

#[serial]
#[tokio::test]
async fn prove_non_inclusion() {
    init_logger();
    spawn_prover(
        true,
        ProverConfig {
            run_mode: None,
            circuits: vec![ProofType::NonInclusion],
        },
    )
    .await;
    let client = Client::new();
    // legacy height 26
    {
        for i in 1..=2 {
            let (inputs, _) = non_inclusion_inputs_string(i);

            let response_result = client
                .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
                .header("Content-Type", "text/plain; charset=utf-8")
                .body(inputs)
                .send()
                .await
                .expect("Failed to execute request.");
            assert!(response_result.status().is_success());
        }
    }
    // height 40
    {
        for i in [1, 2].iter() {
            let inputs =
            light_prover_client::gnark::non_inclusion_json_formatter::non_inclusion_inputs_string(
                i.to_owned(),
            );

            let response_result = client
                .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
                .header("Content-Type", "text/plain; charset=utf-8")
                .body(inputs)
                .send()
                .await
                .expect("Failed to execute request.");
            assert!(response_result.status().is_success());
        }
    }
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
    const HEIGHT: usize = DEFAULT_BATCH_STATE_TREE_HEIGHT as usize;
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

            #[allow(clippy::unnecessary_cast)]
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
        let leaves_hashchain = create_hash_chain_from_slice(&nullifiers).unwrap();
        let inputs = get_batch_update_inputs::<HEIGHT>(
            root,
            vec![tx_hash; num_insertions],
            leaves,
            leaves_hashchain,
            old_leaves,
            merkle_proofs,
            path_indices,
            num_insertions as u32,
        )
        .unwrap();
        let client = Client::new();
        let inputs = update_inputs_string(&inputs);
        let response_result = client
            .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
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
async fn prove_batch_append_with_proofs() {
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

    const HEIGHT: usize = DEFAULT_BATCH_STATE_TREE_HEIGHT as usize;
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
            let leaf = merkle_tree.leaf(index);
            old_leaves.push(leaf);
            merkle_proofs.push(proof.to_vec());
        }

        // Retrieve tree root and compute leaves hash chain
        let root = merkle_tree.root();
        let leaves_hashchain = create_hash_chain_from_slice(&leaves).unwrap();

        // Generate inputs for BatchAppendWithProofsCircuit
        let inputs = get_batch_append_with_proofs_inputs::<HEIGHT>(
            root,
            (i * num_insertions) as u32,
            leaves.clone(),
            leaves_hashchain,
            old_leaves.clone(),
            merkle_proofs.clone(),
            num_insertions as u32,
        )
        .unwrap();

        // Serialize inputs to JSON
        let client = Client::new();
        let inputs_json = BatchAppendWithProofsInputsJson::from_inputs(&inputs).to_string();
        // Send proof request to server
        let response_result = client
            .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
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

    let inputs = get_test_batch_address_append_inputs(addresses, start_index, tree_height);

    let json_output = to_json(&inputs);
    println!("{}", json_output);
}

#[serial]
#[tokio::test]
async fn prove_batch_address_append() {
    use light_hasher::Poseidon;
    use light_indexed_merkle_tree::{array::IndexedArray, reference::IndexedMerkleTree};

    init_logger();
    println!("spawning prover");
    spawn_prover(
        true,
        ProverConfig {
            run_mode: None,
            circuits: vec![ProofType::BatchAddressAppendTest],
        },
    )
    .await;

    // Initialize test data
    let mut new_element_values = vec![];
    let zkp_batch_size = 10;
    for i in 1..zkp_batch_size + 1 {
        new_element_values.push(i.to_biguint().unwrap());
    }

    // Initialize indexing structures
    let mut relayer_indexing_array = IndexedArray::<Poseidon, usize>::default();
    relayer_indexing_array.init().unwrap();
    let mut relayer_merkle_tree =
        IndexedMerkleTree::<Poseidon, usize>::new(DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize, 0)
            .unwrap();
    relayer_merkle_tree.init().unwrap();

    let start_index = relayer_merkle_tree.merkle_tree.rightmost_index;
    let current_root = relayer_merkle_tree.root();

    // Prepare proof components
    let mut low_element_values = Vec::new();
    let mut low_element_indices = Vec::new();
    let mut low_element_next_indices = Vec::new();
    let mut low_element_next_values = Vec::new();
    let mut low_element_proofs: Vec<Vec<[u8; 32]>> = Vec::new();

    // Generate non-inclusion proofs for each element
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

    // Convert big integers to byte arrays
    let new_element_values = new_element_values
        .iter()
        .map(|v| bigint_to_be_bytes_array::<32>(v).unwrap())
        .collect::<Vec<_>>();
    let hash_chain = create_hash_chain_from_slice(&new_element_values).unwrap();
    let batch_start_index = start_index;
    // Generate circuit inputs
    let inputs =
        get_batch_address_append_circuit_inputs::<{ DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize }>(
            start_index,
            current_root,
            low_element_values,
            low_element_next_values,
            low_element_indices,
            low_element_next_indices,
            low_element_proofs,
            new_element_values,
            relayer_merkle_tree
                .merkle_tree
                .get_subtrees()
                .try_into()
                .unwrap(),
            hash_chain,
            batch_start_index,
            zkp_batch_size,
        )
        .unwrap();
    // Convert inputs to JSON format
    let inputs_json = to_json(&inputs);
    // Send proof request to server
    let client = Client::new();
    let response_result = client
        .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(inputs_json)
        .send()
        .await
        .expect("Failed to execute request.");

    // Verify response
    let status = response_result.status();
    let body = response_result.text().await.unwrap();
    assert!(
        status.is_success(),
        "Batch address append proof generation failed. Status: {}, Body: {}",
        status,
        body
    );
}

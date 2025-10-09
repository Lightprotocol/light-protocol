use light_hasher::{
    bigint::bigint_to_be_bytes_array, hash_chain::create_hash_chain_from_slice, Poseidon,
};
use light_prover_client::{
    constants::{DEFAULT_BATCH_ADDRESS_TREE_HEIGHT, PROVE_PATH, SERVER_ADDRESS},
    proof_types::batch_address_append::{
        get_batch_address_append_circuit_inputs, to_json, BatchAddressAppendInputs,
    },
    prover::spawn_prover,
};
use light_sparse_merkle_tree::{
    changelog::ChangelogEntry, indexed_changelog::IndexedChangelogEntry, SparseMerkleTree,
};
use num_bigint::{BigUint, ToBigUint};
use reqwest::Client;
use serial_test::serial;
mod init_merkle_tree;

#[serial]
#[tokio::test]
async fn prove_batch_address_append() {
    use light_hasher::Poseidon;
    use light_merkle_tree_reference::indexed::IndexedMerkleTree;

    println!("spawning prover");
    spawn_prover().await;

    // Initialize test data
    let mut new_element_values = vec![];
    let zkp_batch_size = 10;
    for i in 1..zkp_batch_size + 1 {
        new_element_values.push(num_bigint::ToBigUint::to_biguint(&i).unwrap());
    }

    // Initialize indexing structures
    let relayer_merkle_tree =
        IndexedMerkleTree::<Poseidon, usize>::new(DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize, 0)
            .unwrap();

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
            .get_non_inclusion_proof(new_element_value)
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

    let subtrees: [[u8; 32]; DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize] = relayer_merkle_tree
        .merkle_tree
        .get_subtrees()
        .try_into()
        .unwrap();
    let mut sparse_merkle_tree = SparseMerkleTree::<
        Poseidon,
        { DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize },
    >::new(subtrees, start_index);

    let mut changelog: Vec<ChangelogEntry<{ DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize }>> =
        Vec::new();
    let mut indexed_changelog: Vec<
        IndexedChangelogEntry<usize, { DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize }>,
    > = Vec::new();

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
            &mut sparse_merkle_tree,
            hash_chain,
            zkp_batch_size,
            &mut changelog,
            &mut indexed_changelog,
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

#[test]
pub fn print_circuit_test_data_json_formatted() {
    let addresses = vec![31_u32.to_biguint().unwrap(), 30_u32.to_biguint().unwrap()];
    let start_index = 2;
    let tree_height = 4;

    let inputs = get_test_batch_address_append_inputs(addresses, start_index, tree_height, None);

    let json_output = to_json(&inputs);
    println!("{}", json_output);
}

pub fn get_test_batch_address_append_inputs(
    addresses: Vec<BigUint>,
    start_index: usize,
    tree_height: usize,
    tree: Option<light_merkle_tree_reference::indexed::IndexedMerkleTree<Poseidon, usize>>,
) -> BatchAddressAppendInputs {
    let mut relayer_merkle_tree = light_merkle_tree_reference::indexed::IndexedMerkleTree::<
        Poseidon,
        usize,
    >::new(tree_height, 0)
    .unwrap();

    if let Some(tree) = tree {
        relayer_merkle_tree = tree;
    }

    let old_root = relayer_merkle_tree.root();

    let mut low_element_values = Vec::new();
    let mut low_element_indices = Vec::new();
    let mut low_element_next_indices = Vec::new();
    let mut low_element_next_values = Vec::new();
    let mut low_element_proofs = Vec::new();
    let mut new_element_values = Vec::new();
    let mut new_element_proofs = Vec::new();

    for address in &addresses {
        let non_inclusion_proof = relayer_merkle_tree
            .get_non_inclusion_proof(address)
            .unwrap();
        relayer_merkle_tree
            .verify_non_inclusion_proof(&non_inclusion_proof)
            .unwrap();

        low_element_values.push(BigUint::from_bytes_be(
            &non_inclusion_proof.leaf_lower_range_value,
        ));
        low_element_indices.push(non_inclusion_proof.leaf_index.into());
        low_element_next_indices.push(non_inclusion_proof.next_index.into());
        low_element_next_values.push(BigUint::from_bytes_be(
            &non_inclusion_proof.leaf_higher_range_value,
        ));

        let proof: Vec<BigUint> = non_inclusion_proof
            .merkle_proof
            .iter()
            .map(|proof| BigUint::from_bytes_be(proof))
            .collect();
        low_element_proofs.push(proof);

        let new_proof = relayer_merkle_tree
            .get_proof_of_leaf(relayer_merkle_tree.merkle_tree.rightmost_index - 1, true)
            .unwrap();

        let new_proof: Vec<BigUint> = new_proof
            .iter()
            .map(|proof| BigUint::from_bytes_be(proof))
            .collect();
        new_element_proofs.push(new_proof);
        new_element_values.push(address.clone());
    }

    let new_root = relayer_merkle_tree.root();

    let addresses_bytes = addresses
        .iter()
        .map(|x| bigint_to_be_bytes_array::<32>(x).unwrap())
        .collect::<Vec<_>>();

    let leaves_hashchain = create_hash_chain_from_slice(&addresses_bytes).unwrap();
    let hash_chain_inputs = vec![
        old_root,
        new_root,
        leaves_hashchain,
        bigint_to_be_bytes_array::<32>(&start_index.into()).unwrap(),
    ];
    let public_input_hash = create_hash_chain_from_slice(hash_chain_inputs.as_slice()).unwrap();

    BatchAddressAppendInputs {
        batch_size: addresses.len(),
        hashchain_hash: BigUint::from_bytes_be(&leaves_hashchain),
        low_element_values,
        low_element_indices,
        low_element_next_indices,
        low_element_next_values,
        low_element_proofs,
        new_element_values,
        new_element_proofs,
        new_root: BigUint::from_bytes_be(&new_root),
        old_root: BigUint::from_bytes_be(&old_root),
        public_input_hash: BigUint::from_bytes_be(&public_input_hash),
        start_index,
        tree_height,
    }
}

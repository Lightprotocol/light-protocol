use light_hasher::{hash_chain::create_hash_chain_from_slice, Hasher, Poseidon};
use light_merkle_tree_reference::MerkleTree;
use light_prover_client::{
    constants::{DEFAULT_BATCH_STATE_TREE_HEIGHT, PROVE_PATH, SERVER_ADDRESS},
    proof_types::batch_append::{get_batch_append_inputs, BatchAppendInputsJson},
    prover::spawn_prover,
};
use reqwest::Client;
use serial_test::serial;
mod init_merkle_tree;

#[serial]
#[tokio::test]
async fn prove_batch_append_with_proofs() {
    spawn_prover().await;

    const HEIGHT: usize = DEFAULT_BATCH_STATE_TREE_HEIGHT as usize;
    const CANOPY: usize = 0;
    let num_insertions = 10;
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

        // Generate inputs for BatchAppendCircuit
        let (inputs, _) = get_batch_append_inputs::<HEIGHT>(
            root,
            (i * num_insertions) as u32,
            leaves.clone(),
            leaves_hashchain,
            old_leaves.clone(),
            merkle_proofs.clone(),
            num_insertions as u32,
            &[],
        )
        .unwrap();

        // Serialize inputs to JSON
        let client = Client::new();
        let inputs_json = BatchAppendInputsJson::from_inputs(&inputs).to_string();
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

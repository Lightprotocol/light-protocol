use light_hasher::{hash_chain::create_hash_chain_from_slice, Hasher, Poseidon};
use light_merkle_tree_reference::MerkleTree;
use light_prover_client::{
    constants::{DEFAULT_BATCH_STATE_TREE_HEIGHT, PROVE_PATH, SERVER_ADDRESS},
    proof_types::batch_update::{get_batch_update_inputs, update_inputs_string},
    prover::spawn_prover,
};
use reqwest::Client;
use serial_test::serial;
mod init_merkle_tree;

#[serial]
#[tokio::test]
async fn prove_batch_update() {
    spawn_prover().await;
    const HEIGHT: usize = DEFAULT_BATCH_STATE_TREE_HEIGHT as usize;
    const CANOPY: usize = 0;
    let num_insertions = 10;
    let tx_hash = [0u8; 32];

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
        let (inputs, _) = get_batch_update_inputs::<HEIGHT>(
            root,
            vec![tx_hash; num_insertions],
            leaves,
            leaves_hashchain,
            old_leaves,
            merkle_proofs,
            path_indices,
            num_insertions as u32,
            &[],
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

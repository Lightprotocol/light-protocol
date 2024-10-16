use std::fmt::Error;

use light_hasher::Poseidon;
use light_merkle_tree_reference::MerkleTree;
use reqwest::Client;

use crate::{
    batch_append::{calculate_hash_chain, get_batch_append_inputs},
    batch_update::get_batch_update_inputs,
    gnark::{
        batch_append_json_formatter::append_inputs_string,
        batch_update_json_formatter::update_inputs_string,
        constants::{PROVE_PATH, SERVER_ADDRESS},
        proof_helpers::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
    },
};

// TODO: rename to MockBatchedForester
pub struct MockIndexer<const HEIGHT: usize> {
    pub merkle_tree: MerkleTree<Poseidon>,
    pub input_queue_leaves: Vec<[u8; 32]>,
}

impl<const HEIGHT: usize> MockIndexer<HEIGHT> {
    pub fn new() -> Self {
        let merkle_tree = MerkleTree::<Poseidon>::new(HEIGHT, 0);
        let input_queue_leaves = vec![];
        Self {
            merkle_tree,
            input_queue_leaves,
        }
    }
    pub async fn get_batched_append_proof(
        &mut self,
        next_index: usize,
        leaves_hashchain: [u8; 32],
        leaves: Vec<[u8; 32]>,
        num_zkp_updates: u32,
        batch_size: u32,
    ) -> Result<CompressedProof, Error> {
        let start = num_zkp_updates as usize * batch_size as usize;
        let end = start + batch_size as usize;
        let leaves = leaves[start..end].to_vec();
        let sub_trees = self.merkle_tree.get_subtrees().try_into().unwrap();
        let local_leaves_hashchain = calculate_hash_chain(&leaves);
        assert_eq!(leaves_hashchain, local_leaves_hashchain);
        for leaf in leaves.iter() {
            self.merkle_tree.append(&leaf).unwrap();
        }
        let inputs =
            get_batch_append_inputs::<HEIGHT>(next_index, sub_trees, leaves, leaves_hashchain);
        let client = Client::new();
        let inputs = append_inputs_string(&inputs);

        let response_result = client
            .post(&format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(inputs)
            .send()
            .await
            .expect("Failed to execute request.");
        if response_result.status().is_success() {
            let body = response_result.text().await.unwrap();
            let proof_json = deserialize_gnark_proof_json(&body).unwrap();
            let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
            let (proof_a, proof_b, proof_c) = compress_proof(&proof_a, &proof_b, &proof_c);
            return Ok(CompressedProof {
                a: proof_a,
                b: proof_b,
                c: proof_c,
            });
        }
        Err(Error)
    }

    pub async fn get_batched_update_proof(
        &mut self,
        batch_size: u32,
        leaves_hashchain: [u8; 32],
    ) -> Result<(CompressedProof, [u8; 32]), Error> {
        let mut merkle_proofs = vec![];
        let mut path_indices = vec![];
        let leaves = self.input_queue_leaves[..batch_size as usize].to_vec();
        for leaf in leaves.iter() {
            let index = self.merkle_tree.get_leaf_index(&leaf).unwrap();
            let proof = self.merkle_tree.get_proof_of_leaf(index, true).unwrap();
            merkle_proofs.push(proof.to_vec());
            path_indices.push(index as u32);
            self.input_queue_leaves.remove(0);
            self.merkle_tree.update(&[0u8; 32], index).unwrap();
        }
        let root = self.merkle_tree.root();
        let local_leaves_hashchain = calculate_hash_chain(&leaves);
        assert_eq!(leaves_hashchain, local_leaves_hashchain);
        let inputs = get_batch_update_inputs::<HEIGHT>(
            root,
            leaves,
            leaves_hashchain,
            merkle_proofs,
            path_indices,
            batch_size,
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
        if response_result.status().is_success() {
            let body = response_result.text().await.unwrap();
            let proof_json = deserialize_gnark_proof_json(&body).unwrap();
            let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
            let (proof_a, proof_b, proof_c) = compress_proof(&proof_a, &proof_b, &proof_c);
            return Ok((
                CompressedProof {
                    a: proof_a,
                    b: proof_b,
                    c: proof_c,
                },
                root,
            ));
        }
        Err(Error)
    }
}

pub struct CompressedProof {
    pub a: [u8; 32],
    pub b: [u8; 64],
    pub c: [u8; 32],
}

use light_compressed_account::{
    hash_chain::create_hash_chain_from_slice, instruction_data::compressed_proof::CompressedProof,
};
use light_hasher::{bigint::bigint_to_be_bytes_array, Hasher, Poseidon};
use light_merkle_tree_reference::{indexed::IndexedMerkleTree, MerkleTree};
use light_prover_client::{
    errors::ProverClientError,
    proof_client::ProofClient,
    proof_types::{
        batch_address_append::get_batch_address_append_circuit_inputs,
        batch_append::get_batch_append_inputs, batch_update::get_batch_update_inputs,
    },
};
use light_sparse_merkle_tree::{
    changelog::ChangelogEntry, indexed_changelog::IndexedChangelogEntry, SparseMerkleTree,
};
use num_bigint::BigUint;

#[derive(Clone, Debug)]
pub struct MockBatchedForester<const HEIGHT: usize> {
    pub merkle_tree: MerkleTree<Poseidon>,
    pub input_queue_leaves: Vec<([u8; 32], usize)>,
    /// Indices of leaves which in merkle tree which are active.
    pub output_queue_leaves: Vec<[u8; 32]>,
    pub active_leaves: Vec<[u8; 32]>,
    pub tx_events: Vec<MockTxEvent>,
}

#[derive(Debug, Clone)]
pub struct MockTxEvent {
    pub tx_hash: [u8; 32],
    pub inputs: Vec<[u8; 32]>,
    pub outputs: Vec<[u8; 32]>,
}

impl<const HEIGHT: usize> Default for MockBatchedForester<HEIGHT> {
    fn default() -> Self {
        let merkle_tree = MerkleTree::<Poseidon>::new(HEIGHT, 0);
        let input_queue_leaves = vec![];
        Self {
            merkle_tree,
            input_queue_leaves,
            output_queue_leaves: vec![],
            active_leaves: vec![],
            tx_events: vec![],
        }
    }
}

impl<const HEIGHT: usize> MockBatchedForester<HEIGHT> {
    pub async fn get_batched_append_proof(
        &mut self,
        account_next_index: usize,
        num_zkp_updates: u32,
        batch_size: u32,
        leaves_hashchain: [u8; 32],
        max_num_zkp_updates: u32,
    ) -> Result<(CompressedProof, [u8; 32]), ProverClientError> {
        let leaves = self.output_queue_leaves.to_vec();
        let start = num_zkp_updates as usize * batch_size as usize;
        let end = start + batch_size as usize;
        if end > leaves.len() {
            return Err(ProverClientError::GenericError(format!(
                "end index {} out of bounds for leaves length {}",
                end,
                leaves.len()
            )));
        }
        let leaves = leaves[start..end].to_vec();
        // if batch is complete, remove leaves from mock output queue
        if num_zkp_updates == max_num_zkp_updates - 1 {
            for _ in 0..max_num_zkp_updates * batch_size {
                self.output_queue_leaves.remove(0);
            }
        }
        let local_leaves_hashchain = create_hash_chain_from_slice(&leaves)?;
        assert_eq!(leaves_hashchain, local_leaves_hashchain);

        let old_root = self.merkle_tree.root();
        let mut old_leaves = vec![];
        let mut merkle_proofs = vec![];
        for i in account_next_index..account_next_index + batch_size as usize {
            match self.merkle_tree.get_leaf(i) {
                Ok(leaf) => {
                    old_leaves.push(leaf);
                }
                Err(_) => {
                    old_leaves.push([0u8; 32]);
                    if i <= self.merkle_tree.get_next_index() {
                        self.merkle_tree.append(&[0u8; 32]).unwrap();
                    }
                }
            }
            let proof = self.merkle_tree.get_proof_of_leaf(i, true).unwrap();
            merkle_proofs.push(proof.to_vec());
        }
        // Insert new leaves into the merkle tree. Every leaf which is not [0u8; 32] has already been nullified hence shouldn't be updated.
        for (i, leaf) in leaves.iter().enumerate() {
            if old_leaves[i] == [0u8; 32] {
                let index = account_next_index + i;
                self.merkle_tree.update(leaf, index).unwrap();
            }
        }
        let (circuit_inputs, _) = match get_batch_append_inputs::<HEIGHT>(
            old_root,
            account_next_index as u32,
            leaves,
            local_leaves_hashchain,
            old_leaves,
            merkle_proofs,
            batch_size,
            &[],
        ) {
            Ok(val) => val,
            Err(e) => {
                return Err(ProverClientError::GenericError(format!(
                    "get_batch_append_inputs error: {:?}",
                    e
                )));
            }
        };
        let computed_new_root =
            match bigint_to_be_bytes_array::<32>(&circuit_inputs.new_root.to_biguint().unwrap()) {
                Ok(val) => val,
                Err(e) => {
                    return Err(ProverClientError::GenericError(format!(
                        "bigint_to_be_bytes_array error: {:?}",
                        e
                    )));
                }
            };

        assert_eq!(computed_new_root, self.merkle_tree.root());

        let proof_result = match ProofClient::local()
            .generate_batch_append_proof(circuit_inputs)
            .await
        {
            Ok(val) => val,
            Err(e) => {
                return Err(ProverClientError::GenericError(format!(
                    "generate_batch_append_proof error: {:?}",
                    e
                )));
            }
        };
        let proof = CompressedProof {
            a: proof_result.0.a,
            b: proof_result.0.b,
            c: proof_result.0.c,
        };
        Ok((proof, proof_result.1))
    }

    pub async fn get_batched_update_proof(
        &mut self,
        batch_size: u32,
        leaves_hashchain: [u8; 32],
    ) -> Result<(CompressedProof, [u8; 32]), ProverClientError> {
        let mut merkle_proofs = vec![];
        let mut path_indices = vec![];
        let leaves = self.input_queue_leaves[..batch_size as usize].to_vec();
        let old_root = self.merkle_tree.root();
        let mut nullifiers = Vec::new();
        let mut tx_hashes = Vec::new();
        let mut old_leaves = Vec::new();
        for (leaf, index) in leaves.iter() {
            let index = *index;
            // + 2 because next index is + 1 and we need to init the leaf in
            //   pos[index]
            if self.merkle_tree.get_next_index() < index + 2 {
                old_leaves.push([0u8; 32]);
            } else {
                old_leaves.push(*leaf);
            }
            // Handle case that we nullify a leaf which has not been inserted yet.
            while self.merkle_tree.get_next_index() < index + 2 {
                self.merkle_tree.append(&[0u8; 32]).unwrap();
            }
            let proof = self.merkle_tree.get_proof_of_leaf(index, true).unwrap();
            merkle_proofs.push(proof.to_vec());
            path_indices.push(index as u32);
            self.input_queue_leaves.remove(0);
            let event = self
                .tx_events
                .iter()
                .find(|tx_event| tx_event.inputs.contains(leaf))
                .expect("No event for leaf found.");
            let index_bytes = index.to_be_bytes();
            let nullifier = Poseidon::hashv(&[leaf, &index_bytes, &event.tx_hash]).unwrap();
            tx_hashes.push(event.tx_hash);
            nullifiers.push(nullifier);
            self.merkle_tree.update(&nullifier, index).unwrap();
        }
        // local_leaves_hashchain is only used for a test assertion.
        let local_nullifier_hashchain = create_hash_chain_from_slice(&nullifiers)?;
        assert_eq!(leaves_hashchain, local_nullifier_hashchain);
        let (inputs, _) = get_batch_update_inputs::<HEIGHT>(
            old_root,
            tx_hashes,
            leaves.iter().map(|(leaf, _)| *leaf).collect(),
            leaves_hashchain,
            old_leaves,
            merkle_proofs,
            path_indices,
            batch_size,
            &[],
        )?;
        let proof_result = ProofClient::local()
            .generate_batch_update_proof(inputs)
            .await?;
        let new_root = self.merkle_tree.root();
        let proof = CompressedProof {
            a: proof_result.0.a,
            b: proof_result.0.b,
            c: proof_result.0.c,
        };
        Ok((proof, new_root))
    }
}

#[derive(Clone, Debug)]
pub struct MockBatchedAddressForester<const HEIGHT: usize> {
    pub merkle_tree: IndexedMerkleTree<Poseidon, usize>,
    pub queue_leaves: Vec<[u8; 32]>,
}

impl<const HEIGHT: usize> Default for MockBatchedAddressForester<HEIGHT> {
    fn default() -> Self {
        let merkle_tree = IndexedMerkleTree::<Poseidon, usize>::new(HEIGHT, 0).unwrap();
        let queue_leaves = vec![];
        Self {
            merkle_tree,
            queue_leaves,
        }
    }
}

impl<const HEIGHT: usize> MockBatchedAddressForester<HEIGHT> {
    pub async fn get_batched_address_proof(
        &mut self,
        _batch_size: u32,
        zkp_batch_size: u32,
        leaves_hashchain: [u8; 32],
        start_index: usize,
        batch_start_index: usize,
        current_root: [u8; 32],
    ) -> Result<(CompressedProof, [u8; 32]), ProverClientError> {
        let new_element_values = self.queue_leaves[..zkp_batch_size as usize].to_vec();
        assert_eq!(
            self.merkle_tree.merkle_tree.rightmost_index,
            batch_start_index
        );
        assert!(
            batch_start_index >= 1,
            "start index should be greater than 2 else tree is not inited"
        );
        let mut low_element_values = Vec::new();
        let mut low_element_indices = Vec::new();
        let mut low_element_next_indices = Vec::new();
        let mut low_element_next_values = Vec::new();
        let mut low_element_proofs: Vec<Vec<[u8; 32]>> = Vec::new();
        for new_element_value in &new_element_values {
            let non_inclusion_proof = self
                .merkle_tree
                .get_non_inclusion_proof(&BigUint::from_bytes_be(new_element_value.as_slice()))
                .unwrap();
            low_element_values.push(non_inclusion_proof.leaf_lower_range_value);
            low_element_indices.push(non_inclusion_proof.leaf_index);
            low_element_next_indices.push(non_inclusion_proof.next_index);
            low_element_next_values.push(non_inclusion_proof.leaf_higher_range_value);
            low_element_proofs.push(non_inclusion_proof.merkle_proof.as_slice().to_vec());
        }
        let subtrees = self.merkle_tree.merkle_tree.get_subtrees();
        let mut merkle_tree = match <[[u8; 32]; HEIGHT]>::try_from(subtrees) {
            Ok(arr) => SparseMerkleTree::<Poseidon, HEIGHT>::new(arr, start_index),
            Err(e) => {
                return Err(ProverClientError::GenericError(format!(
                    "get_subtrees/try_from error: {:?}",
                    e
                )));
            }
        };
        let mut changelog: Vec<ChangelogEntry<HEIGHT>> = Vec::new();
        let mut indexed_changelog: Vec<IndexedChangelogEntry<usize, HEIGHT>> = Vec::new();
        let inputs = match get_batch_address_append_circuit_inputs::<HEIGHT>(
            start_index,
            current_root,
            low_element_values,
            low_element_next_values,
            low_element_indices,
            low_element_next_indices,
            low_element_proofs,
            new_element_values.clone(),
            &mut merkle_tree,
            leaves_hashchain,
            zkp_batch_size as usize,
            &mut changelog,
            &mut indexed_changelog,
        ) {
            Ok(val) => val,
            Err(e) => {
                return Err(ProverClientError::GenericError(format!(
                    "get_batch_address_append_circuit_inputs error: {:?}",
                    e
                )));
            }
        };
        let proof_result = match ProofClient::local()
            .generate_batch_address_append_proof(inputs)
            .await
        {
            Ok(val) => val,
            Err(e) => {
                return Err(ProverClientError::GenericError(format!(
                    "generate_batch_address_append_proof error: {:?}",
                    e
                )));
            }
        };
        let proof = CompressedProof {
            a: proof_result.0.a,
            b: proof_result.0.b,
            c: proof_result.0.c,
        };
        Ok((proof, proof_result.1))
    }

    pub fn finalize_batch_address_update(&mut self, batch_size: usize) {
        println!("finalize batch address update");
        let new_element_values = self.queue_leaves[..batch_size].to_vec();
        println!("removing leaves from queue {}", batch_size);
        for _ in 0..batch_size {
            self.queue_leaves.remove(0);
        }
        println!("new queue length {}", self.queue_leaves.len());
        for new_element_value in &new_element_values {
            self.merkle_tree
                .append(&BigUint::from_bytes_be(new_element_value))
                .unwrap();
        }
        println!(
            "new next index {}",
            self.merkle_tree.merkle_tree.get_next_index()
        );
    }
}

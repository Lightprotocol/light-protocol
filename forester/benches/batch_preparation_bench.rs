#![feature(test)]
extern crate test;

use forester::processor::v2::coordinator::{
    batch_preparation::prepare_append_batch,
    tree_state::TreeState,
    types::{AppendQueueData, PreparationState},
};
use light_client::indexer::{MerkleProofWithContext, OutputQueueDataV2};
use light_hasher::{hash_chain::create_hash_chain_from_slice, Hasher, Poseidon};
use test::Bencher;

/*
  M4 Max:

  Single batch (500 elements):
  - Time: ~153.6 ms per iteration
  - Per element: ~307 μs

  10 batches (5,000 elements):
  - Time: ~1.54 seconds per iteration
  - Per batch: ~153.7 ms
  - Per element: ~307 μs

  60 batches (30,000 elements):
  - Time: ~9.66 seconds per iteration
  - Per batch: ~161 ms
  - Per element: ~322 μs
*/

fn create_test_append_data(num_batches: usize, zkp_batch_size: u16) -> AppendQueueData {
    let total_elements = num_batches * zkp_batch_size as usize;

    let mut queue_elements = Vec::with_capacity(total_elements);
    for i in 0..total_elements {
        let account_hash = Poseidon::hashv(&[&(i as u64).to_le_bytes()]).unwrap();

        let merkle_proof: Vec<[u8; 32]> = (0..32)
            .map(|level| Poseidon::hashv(&[&(level as u64).to_le_bytes(), &account_hash]).unwrap())
            .collect();

        let root = Poseidon::hashv(&[&account_hash]).unwrap();
        let merkle_tree = Poseidon::hashv(&[b"merkle_tree"]).unwrap();

        queue_elements.push(MerkleProofWithContext {
            proof: merkle_proof,
            root,
            leaf_index: i as u64,
            leaf: account_hash,
            merkle_tree,
            root_seq: 0,
            tx_hash: None,
            account_hash,
        });
    }

    let mut leaves_hash_chains = Vec::with_capacity(num_batches);
    for batch_idx in 0..num_batches {
        let start = batch_idx * zkp_batch_size as usize;
        let end = start + zkp_batch_size as usize;
        let batch_leaves: Vec<[u8; 32]> = queue_elements[start..end]
            .iter()
            .map(|elem| elem.account_hash)
            .collect();

        let hash_chain =
            create_hash_chain_from_slice(&batch_leaves).expect("Failed to create hash chain");
        leaves_hash_chains.push(hash_chain);
    }

    AppendQueueData {
        queue_elements,
        leaves_hash_chains,
        zkp_batch_size,
    }
}

fn create_initial_tree_state(append_data: &AppendQueueData) -> (TreeState, Vec<u64>) {
    let num_elements = append_data.total_elements();

    let account_hashes: Vec<[u8; 32]> = append_data
        .queue_elements
        .iter()
        .map(|elem| elem.account_hash)
        .collect();

    let old_leaves: Vec<[u8; 32]> = (0..num_elements)
        .map(|i| Poseidon::hashv(&[b"old_leaf", &(i as u64).to_le_bytes()]).unwrap())
        .collect();

    let output_queue = OutputQueueDataV2 {
        nodes: vec![],
        node_hashes: vec![],
        leaf_indices: (0..num_elements as u64).collect(),
        account_hashes,
        old_leaves,
        initial_root: Poseidon::hashv(&[b"genesis_root"]).unwrap(),
        first_queue_index: 0,
    };

    let tree_state = TreeState::from_v2_response(Some(&output_queue), None)
        .expect("Failed to create tree state");

    let append_leaf_indices: Vec<u64> = (0..num_elements as u64).collect();

    (tree_state, append_leaf_indices)
}

#[bench]
fn bench_prepare_60_batches_500_elements(b: &mut Bencher) {
    const NUM_BATCHES: usize = 60;
    const ZKP_BATCH_SIZE: u16 = 500;

    let append_data = create_test_append_data(NUM_BATCHES, ZKP_BATCH_SIZE);
    let (tree_state, append_leaf_indices) = create_initial_tree_state(&append_data);

    b.iter(|| {
        let mut state = PreparationState::new(tree_state.clone(), append_leaf_indices.clone());

        for _batch_idx in 0..NUM_BATCHES {
            let result = prepare_append_batch(&append_data, &mut state);

            test::black_box(result.expect("Batch preparation failed"));
        }
    });
}

#[bench]
fn bench_prepare_10_batches_500_elements(b: &mut Bencher) {
    const NUM_BATCHES: usize = 10;
    const ZKP_BATCH_SIZE: u16 = 500;

    let append_data = create_test_append_data(NUM_BATCHES, ZKP_BATCH_SIZE);
    let (tree_state, append_leaf_indices) = create_initial_tree_state(&append_data);

    b.iter(|| {
        let mut state = PreparationState::new(tree_state.clone(), append_leaf_indices.clone());

        for _batch_idx in 0..NUM_BATCHES {
            let result = prepare_append_batch(&append_data, &mut state);
            test::black_box(result.expect("Batch preparation failed"));
        }
    });
}

#[bench]
fn bench_prepare_single_batch_500_elements(b: &mut Bencher) {
    const NUM_BATCHES: usize = 1;
    const ZKP_BATCH_SIZE: u16 = 500;

    let append_data = create_test_append_data(NUM_BATCHES, ZKP_BATCH_SIZE);
    let (tree_state, append_leaf_indices) = create_initial_tree_state(&append_data);

    b.iter(|| {
        let mut state = PreparationState::new(tree_state.clone(), append_leaf_indices.clone());
        let result = prepare_append_batch(&append_data, &mut state);
        test::black_box(result.expect("Batch preparation failed"));
    });
}

#[test]
fn test_batch_preparation_correctness() {
    const NUM_BATCHES: usize = 5;
    const ZKP_BATCH_SIZE: u16 = 100;

    let append_data = create_test_append_data(NUM_BATCHES, ZKP_BATCH_SIZE);
    let (tree_state, append_leaf_indices) = create_initial_tree_state(&append_data);

    let mut state = PreparationState::new(tree_state.clone(), append_leaf_indices.clone());

    for batch_idx in 0..NUM_BATCHES {
        let result = prepare_append_batch(&append_data, &mut state);
        assert!(
            result.is_ok(),
            "Batch {} preparation failed: {:?}",
            batch_idx,
            result.err()
        );

        let circuit_inputs = result.unwrap();

        assert_eq!(
            circuit_inputs.leaves.len(),
            ZKP_BATCH_SIZE as usize,
            "Batch {} has wrong number of leaves",
            batch_idx
        );

        assert_eq!(
            state.append_batch_index,
            batch_idx + 1,
            "Batch index not incremented correctly"
        );
    }

    assert_eq!(state.append_batch_index, NUM_BATCHES);
}

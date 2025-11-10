#![allow(unused_assignments)]

use light_array_map::ArrayMap;
use light_batched_merkle_tree::{
    batch::BatchState,
    constants::{DEFAULT_BATCH_STATE_TREE_HEIGHT, NUM_BATCHES},
    errors::BatchedMerkleTreeError,
    initialize_state_tree::{
        init_batched_state_merkle_tree_accounts,
        test_utils::get_state_merkle_tree_account_size_from_params,
        InitStateTreeAccountsInstructionData,
    },
    merkle_tree::{BatchedMerkleTreeAccount, InstructionDataBatchAppendInputs},
    queue::{test_utils::get_output_queue_account_size_from_params, BatchedQueueAccount},
};
use light_bloom_filter::BloomFilter;
use light_compressed_account::{
    hash_chain::create_hash_chain_from_slice, instruction_data::compressed_proof::CompressedProof,
    pubkey::Pubkey,
};
use light_prover_client::prover::spawn_prover;
use light_test_utils::mock_batched_forester::{MockBatchedForester, MockTxEvent};
use rand::rngs::StdRng;
use serial_test::serial;

use crate::e2e_tests::shared::*;

#[serial]
#[tokio::test]
async fn test_fill_state_queues_completely() {
    spawn_prover().await;
    let mut current_slot = 1;
    let roothistory_capacity = vec![17, 80];
    for root_history_capacity in roothistory_capacity {
        let mut mock_indexer =
            MockBatchedForester::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>::default();

        let mut params = InitStateTreeAccountsInstructionData::test_default();
        params.output_queue_batch_size = params.input_queue_batch_size * 10;
        // Root history capacity which is greater than the input updates
        params.root_history_capacity = root_history_capacity;

        let owner = Pubkey::new_unique();

        let queue_account_size = get_output_queue_account_size_from_params(params);

        let mut output_queue_account_data = vec![0; queue_account_size];
        let output_queue_pubkey = Pubkey::new_unique();

        let mt_account_size = get_state_merkle_tree_account_size_from_params(params);
        let mut mt_account_data = vec![0; mt_account_size];
        let mt_pubkey = Pubkey::new_unique();

        let merkle_tree_rent = 1_000_000_000;
        let queue_rent = 1_000_000_000;
        let additional_bytes_rent = 1000;

        init_batched_state_merkle_tree_accounts(
            owner,
            params,
            &mut output_queue_account_data,
            output_queue_pubkey,
            queue_rent,
            &mut mt_account_data,
            mt_pubkey,
            merkle_tree_rent,
            additional_bytes_rent,
        )
        .unwrap();
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(0);

        // Track roots created during each batch insertion (batch_index -> roots)
        let mut batch_roots: ArrayMap<u32, Vec<[u8; 32]>, 2> = ArrayMap::new();

        let num_tx = NUM_BATCHES as u64 * params.output_queue_batch_size;

        // Fill up complete output queue
        for _ in 0..num_tx {
            // Output queue

            let rnd_bytes = get_rnd_bytes(&mut rng);
            let mut pre_output_queue_account_data = output_queue_account_data.clone();
            let pre_output_account =
                BatchedQueueAccount::output_from_bytes(&mut pre_output_queue_account_data).unwrap();
            let pre_account = *pre_output_account.get_metadata();
            let pre_value_store = pre_output_account.value_vecs;
            let pre_hash_chains = pre_output_account.hash_chain_stores;

            let mut output_account =
                BatchedQueueAccount::output_from_bytes(&mut output_queue_account_data).unwrap();

            output_account
                .insert_into_current_batch(&rnd_bytes, &current_slot)
                .unwrap();
            assert_output_queue_insert(
                pre_account,
                pre_value_store,
                pre_hash_chains,
                BatchedQueueAccount::output_from_bytes(
                    &mut output_queue_account_data.clone(), // clone so that data cannot be modified
                )
                .unwrap(),
                vec![rnd_bytes],
                current_slot,
            )
            .unwrap();
            current_slot += 1;
            mock_indexer.output_queue_leaves.push(rnd_bytes);
        }
        let rnd_bytes = get_rnd_bytes(&mut rng);
        let mut output_account =
            BatchedQueueAccount::output_from_bytes(&mut output_queue_account_data).unwrap();

        let result = output_account.insert_into_current_batch(&rnd_bytes, &current_slot);
        assert_eq!(result.unwrap_err(), BatchedMerkleTreeError::BatchNotReady);

        output_account
            .batch_metadata
            .batches
            .iter()
            .for_each(|b| assert_eq!(b.get_state(), BatchState::Full));

        // Batch insert output queue into merkle tree.
        for _ in 0..output_account
            .get_metadata()
            .batch_metadata
            .get_num_zkp_batches()
        {
            println!("Output update -----------------------------");
            let mut pre_mt_account_data = mt_account_data.clone();
            let mut account =
                BatchedMerkleTreeAccount::state_from_bytes(&mut pre_mt_account_data, &mt_pubkey)
                    .unwrap();
            let mut pre_output_queue_state = output_queue_account_data.clone();
            let output_account =
                BatchedQueueAccount::output_from_bytes(&mut output_queue_account_data).unwrap();
            let next_index = account.get_metadata().next_index;
            let next_full_batch = output_account
                .get_metadata()
                .batch_metadata
                .pending_batch_index;
            let batch = output_account
                .batch_metadata
                .batches
                .get(next_full_batch as usize)
                .unwrap();
            let leaves = mock_indexer.output_queue_leaves.clone();
            let leaves_hash_chain = output_account
                .hash_chain_stores
                .get(next_full_batch as usize)
                .unwrap()
                .get(batch.get_num_inserted_zkps() as usize)
                .unwrap();
            let (proof, new_root) = mock_indexer
                .get_batched_append_proof(
                    next_index as usize,
                    batch.get_num_inserted_zkps() as u32,
                    batch.zkp_batch_size as u32,
                    *leaves_hash_chain,
                    batch.get_num_zkp_batches() as u32,
                )
                .await
                .unwrap();
            let start = batch.get_num_inserted_zkps() as usize * batch.zkp_batch_size as usize;
            let end = start + batch.zkp_batch_size as usize;
            for leaf in &leaves[start..end] {
                // Storing the leaf in the output queue indexer so that it
                // can be inserted into the input queue later.
                mock_indexer.active_leaves.push(*leaf);
            }

            let instruction_data = InstructionDataBatchAppendInputs {
                new_root,
                compressed_proof: CompressedProof {
                    a: proof.a,
                    b: proof.b,
                    c: proof.c,
                },
            };

            println!("Output update -----------------------------");
            let queue_account =
                &mut BatchedQueueAccount::output_from_bytes(&mut pre_output_queue_state).unwrap();
            let output_res =
                account.update_tree_from_output_queue_account(queue_account, instruction_data);
            assert!(output_res.is_ok());

            assert_eq!(
                *account.root_history.last().unwrap(),
                mock_indexer.merkle_tree.root()
            );

            // Track root for this batch
            let batch_idx = next_full_batch as u32;
            if let Some(roots) = batch_roots.get_mut_by_key(&batch_idx) {
                roots.push(new_root);
            } else {
                batch_roots.insert(batch_idx, vec![new_root], ()).unwrap();
            }

            output_queue_account_data = pre_output_queue_state;
            mt_account_data = pre_mt_account_data;
        }

        // Fill up complete input queue.
        let num_tx = NUM_BATCHES as u64 * params.input_queue_batch_size;
        let mut first_value = [0u8; 32];
        for tx in 0..num_tx {
            println!("Input insert ----------------------------- {}", tx);
            let (_, leaf) = get_random_leaf(&mut rng, &mut mock_indexer.active_leaves);
            let leaf_index = mock_indexer.merkle_tree.get_leaf_index(&leaf).unwrap();

            let mut pre_mt_account_data = mt_account_data.clone();
            let pre_merkle_tree_account =
                BatchedMerkleTreeAccount::state_from_bytes(&mut pre_mt_account_data, &mt_pubkey)
                    .unwrap();
            let pre_account = *pre_merkle_tree_account.get_metadata();
            let pre_roots = pre_merkle_tree_account
                .root_history
                .iter()
                .cloned()
                .collect();
            let pre_hash_chains = pre_merkle_tree_account.hash_chain_stores;
            let tx_hash = create_hash_chain_from_slice(&[leaf]).unwrap();
            // Index input queue insert event
            mock_indexer.input_queue_leaves.push((leaf, leaf_index));
            mock_indexer.tx_events.push(MockTxEvent {
                inputs: vec![leaf],
                outputs: vec![],
                tx_hash,
            });
            println!("leaf {:?}", leaf);
            println!("leaf_index {:?}", leaf_index);
            let mut merkle_tree_account =
                BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();

            merkle_tree_account
                .insert_nullifier_into_queue(
                    &leaf.to_vec().try_into().unwrap(),
                    leaf_index as u64,
                    &tx_hash,
                    &current_slot,
                )
                .unwrap();
            println!("current slot {:?}", current_slot);
            assert_nullifier_queue_insert(
                pre_account,
                &mut [],
                pre_roots,
                pre_hash_chains,
                merkle_tree_account,
                vec![leaf],
                vec![leaf_index as u64],
                tx_hash,
                vec![true],
                vec![],
                &current_slot,
            )
            .unwrap();
            current_slot += 1;
            println!("leaf {:?}", leaf);
            // Insert the same value twice
            {
                // copy data so that failing test doesn't affect the state of
                // subsequent tests
                let mut mt_account_data = mt_account_data.clone();
                let mut merkle_tree_account =
                    BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &mt_pubkey)
                        .unwrap();
                let result = merkle_tree_account.insert_nullifier_into_queue(
                    &leaf.to_vec().try_into().unwrap(),
                    leaf_index as u64,
                    &tx_hash,
                    &current_slot,
                );
                result.unwrap_err();
                // assert_eq!(
                //     result.unwrap_err(),
                //     BatchedMerkleTreeError::BatchInsertFailed.into()
                // );
            }
            // Try to insert first value into any batch
            if tx == 0 {
                first_value = leaf;
            } else {
                let mut mt_account_data = mt_account_data.clone();
                let mut merkle_tree_account =
                    BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &mt_pubkey)
                        .unwrap();
                let result = merkle_tree_account.insert_nullifier_into_queue(
                    &first_value.to_vec().try_into().unwrap(),
                    leaf_index as u64,
                    &tx_hash,
                    &current_slot,
                );
                // assert_eq!(
                //     result.unwrap_err(),
                //     BatchedMerkleTreeError::BatchInsertFailed.into()
                // );
                result.unwrap_err();
                // assert_eq!(result.unwrap_err(), BloomFilterError::Full.into());
            }
        }
        // Assert input queue is full and doesn't accept more inserts
        {
            let merkle_tree_account =
                &mut BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();
            let rnd_bytes = get_rnd_bytes(&mut rng);
            let tx_hash = get_rnd_bytes(&mut rng);
            let result = merkle_tree_account.insert_nullifier_into_queue(
                &rnd_bytes,
                0,
                &tx_hash,
                &current_slot,
            );
            assert_eq!(result.unwrap_err(), BatchedMerkleTreeError::BatchNotReady);
        }
        // Root of the final batch of first input queue batch
        let mut first_input_batch_update_root_value = [0u8; 32];
        let num_updates =
            params.input_queue_batch_size / params.input_queue_zkp_batch_size * NUM_BATCHES as u64;
        for i in 0..num_updates {
            println!("input update ----------------------------- {}", i);

            perform_input_update(
                &mut mt_account_data,
                &mut mock_indexer,
                false,
                mt_pubkey,
                &mut batch_roots,
            )
            .await;

            let merkle_tree_account =
                &mut BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();

            // after 5 updates the first batch is completely inserted
            // As soon as we switch to inserting the second batch we zero out the first batch since
            // the second batch is completely full.
            if i >= 5 {
                let batch = merkle_tree_account.queue_batches.batches.first().unwrap();
                assert!(batch.bloom_filter_is_zeroed());

                // Assert that none of the unsafe roots from batch 0 exist in root history
                if let Some(unsafe_roots) = batch_roots.get_by_key(&0) {
                    for unsafe_root in unsafe_roots {
                        assert!(
                            !merkle_tree_account
                                .root_history
                                .iter()
                                .any(|x| *x == *unsafe_root),
                            "Unsafe root from batch 0 should be zeroed: {:?}",
                            unsafe_root
                        );
                    }
                }
            } else {
                let batch = merkle_tree_account.queue_batches.batches.first().unwrap();
                assert!(!batch.bloom_filter_is_zeroed());
            }
            let batch_one = &merkle_tree_account.queue_batches.batches[1];
            assert!(!batch_one.bloom_filter_is_zeroed());

            println!(
                "performed input queue batched update {} created root {:?}",
                i,
                mock_indexer.merkle_tree.root()
            );
            if i == 4 {
                first_input_batch_update_root_value = mock_indexer.merkle_tree.root();
            }
            let merkle_tree_account =
                BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();
            println!(
                "root {:?}",
                merkle_tree_account.root_history.last().unwrap()
            );
            println!(
                "root last index {:?}",
                merkle_tree_account.root_history.last_index()
            );
        }
        // assert all bloom_filters are inserted
        {
            let merkle_tree_account =
                &mut BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();
            for (i, batch) in merkle_tree_account.queue_batches.batches.iter().enumerate() {
                assert_eq!(batch.get_state(), BatchState::Inserted);
                if i == 0 {
                    assert!(batch.bloom_filter_is_zeroed());
                } else {
                    assert!(!batch.bloom_filter_is_zeroed());
                }
            }
        }
        // do one insert and expect that roots until  merkle_tree_account.batches[0].root_index are zero
        {
            let merkle_tree_account =
                &mut BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();
            let pre_batch_zero = *merkle_tree_account.queue_batches.batches.first().unwrap();

            let value = &get_rnd_bytes(&mut rng);
            let tx_hash = &get_rnd_bytes(&mut rng);
            merkle_tree_account
                .insert_nullifier_into_queue(value, 0, tx_hash, &current_slot)
                .unwrap();
            {
                let post_batch = *merkle_tree_account.queue_batches.batches.first().unwrap();
                assert_eq!(post_batch.get_state(), BatchState::Fill);
                assert_eq!(post_batch.get_num_inserted_zkp_batch(), 1);
                let bloom_filter_store =
                    merkle_tree_account.bloom_filter_stores.get_mut(0).unwrap();
                let mut bloom_filter = BloomFilter::new(
                    params.bloom_filter_num_iters as usize,
                    params.bloom_filter_capacity,
                    bloom_filter_store,
                )
                .unwrap();
                assert!(bloom_filter.contains(value));
            }

            for root in merkle_tree_account.root_history.iter() {
                println!("root {:?}", root);
            }
            println!(
                "root in root index {:?}",
                merkle_tree_account.root_history[pre_batch_zero.root_index as usize]
            );
            for batch_idx in 0..NUM_BATCHES as u32 {
                println!("batch idx {:?}", batch_idx);
                if let Some(roots) = batch_roots.get_by_key(&batch_idx) {
                    for root in roots.iter() {
                        println!("tracked root {:?}", root);
                    }
                } else {
                    println!("No roots found for batch {}", batch_idx);
                }
            }
            // check that all roots have been overwritten except the root index
            // of the update
            let root_history_len: u32 = merkle_tree_account.root_history.len() as u32;
            let start = merkle_tree_account.root_history.last_index() as u32;
            println!("start {:?}", start);
            for root in start + 1..pre_batch_zero.root_index + root_history_len {
                println!("actual index {:?}", root);
                let index = root % root_history_len;

                if index == pre_batch_zero.root_index {
                    let root_index = pre_batch_zero.root_index as usize;

                    assert_eq!(
                        merkle_tree_account.root_history[root_index],
                        first_input_batch_update_root_value
                    );
                    assert_eq!(merkle_tree_account.root_history[root_index], [0u8; 32]);
                    // First non zeroed root.
                    assert_ne!(merkle_tree_account.root_history[root_index + 1], [0u8; 32]);
                    break;
                }
                println!("index {:?}", index);
                assert_eq!(merkle_tree_account.root_history[index as usize], [0u8; 32]);
            }
        }
    }
}

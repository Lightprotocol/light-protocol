#![allow(unused_assignments)]

use light_batched_merkle_tree::{
    batch::BatchState,
    constants::{DEFAULT_BATCH_ADDRESS_TREE_HEIGHT, NUM_BATCHES},
    errors::BatchedMerkleTreeError,
    initialize_address_tree::{
        get_address_merkle_tree_account_size_from_params, init_batched_address_merkle_tree_account,
        InitAddressTreeAccountsInstructionData,
    },
    merkle_tree::BatchedMerkleTreeAccount,
};
use light_bloom_filter::BloomFilterError;
use light_compressed_account::pubkey::Pubkey;
use light_prover_client::prover::spawn_prover;
use light_test_utils::mock_batched_forester::MockBatchedAddressForester;
use rand::rngs::StdRng;
use serial_test::serial;

use crate::e2e_tests::shared::*;

#[serial]
#[tokio::test]
async fn test_fill_address_tree_completely() {
    spawn_prover().await;
    let mut current_slot = 1;
    let roothistory_capacity = vec![17, 80];
    for root_history_capacity in roothistory_capacity {
        let mut mock_indexer =
            MockBatchedAddressForester::<{ DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize }>::default();

        let mut params = InitAddressTreeAccountsInstructionData::test_default();
        // Root history capacity which is greater than the input updates
        params.root_history_capacity = root_history_capacity;

        let owner = Pubkey::new_unique();

        let mt_account_size = get_address_merkle_tree_account_size_from_params(params);
        let mut mt_account_data = vec![0; mt_account_size];
        let mt_pubkey = Pubkey::new_unique();

        let merkle_tree_rent = 1_000_000_000;

        init_batched_address_merkle_tree_account(
            owner,
            params,
            &mut mt_account_data,
            merkle_tree_rent,
            mt_pubkey,
        )
        .unwrap();
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(0);

        let num_tx = NUM_BATCHES * params.input_queue_batch_size as usize;
        let mut first_value = [0u8; 32];
        for tx in 0..num_tx {
            println!("Input insert -----------------------------");
            let mut rnd_address = get_rnd_bytes(&mut rng);
            rnd_address[0] = 0;

            let mut pre_account_data = mt_account_data.clone();
            let pre_merkle_tree_account =
                BatchedMerkleTreeAccount::address_from_bytes(&mut pre_account_data, &mt_pubkey)
                    .unwrap();
            let pre_account = *pre_merkle_tree_account.get_metadata();
            let pre_roots = pre_merkle_tree_account
                .root_history
                .iter()
                .cloned()
                .collect();
            let pre_hash_chains = pre_merkle_tree_account.hash_chain_stores;
            let mut merkle_tree_account =
                BatchedMerkleTreeAccount::address_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();
            merkle_tree_account
                .insert_address_into_queue(&rnd_address, &current_slot)
                .unwrap();
            assert_input_queue_insert(
                pre_account,
                &mut [],
                pre_roots,
                pre_hash_chains,
                merkle_tree_account,
                vec![rnd_address],
                vec![rnd_address],
                vec![true],
                vec![],
                &current_slot,
            )
            .unwrap();
            current_slot += 1;
            mock_indexer.queue_leaves.push(rnd_address);

            // Insert the same value twice
            {
                // copy data so that failing test doesn't affect the state of
                // subsequent tests
                let mut mt_account_data = mt_account_data.clone();
                let mut merkle_tree_account =
                    BatchedMerkleTreeAccount::address_from_bytes(&mut mt_account_data, &mt_pubkey)
                        .unwrap();
                let result =
                    merkle_tree_account.insert_address_into_queue(&rnd_address, &current_slot);
                println!("tx {}", tx);
                println!("errors {:?}", result);
                if tx == params.input_queue_batch_size as usize * 2 - 1 {
                    // Error when the value is already inserted into the other batch.
                    assert_eq!(result.unwrap_err(), BatchedMerkleTreeError::BatchNotReady);
                } else if tx == params.input_queue_batch_size as usize - 1 {
                    // Error when the value is already inserted into the other batch.
                    // This occurs only when we switch the batch in this test.
                    assert_eq!(
                        result.unwrap_err(),
                        BatchedMerkleTreeError::NonInclusionCheckFailed
                    );
                } else {
                    // Error when inserting into the bloom filter directly twice.
                    assert_eq!(result.unwrap_err(), BloomFilterError::Full.into());
                }

                current_slot += 1;
            }
            // Try to insert first value into any batch
            if tx == 0 {
                first_value = rnd_address;
            } else {
                let mut mt_account_data = mt_account_data.clone();
                let mut merkle_tree_account =
                    BatchedMerkleTreeAccount::address_from_bytes(&mut mt_account_data, &mt_pubkey)
                        .unwrap();

                let result = merkle_tree_account.insert_address_into_queue(
                    &first_value.to_vec().try_into().unwrap(),
                    &current_slot,
                );
                println!("tx {}", tx);
                println!("result {:?}", result);
                if tx == params.input_queue_batch_size as usize * 2 - 1 {
                    // Error when the value is already inserted into the other batch.
                    assert_eq!(result.unwrap_err(), BatchedMerkleTreeError::BatchNotReady);
                } else if tx >= params.input_queue_batch_size as usize - 1
                // || tx == params.input_queue_batch_size as usize
                {
                    // Error when the value is already inserted into the other batch.
                    // This occurs only when we switch the batch in this test.
                    assert_eq!(
                        result.unwrap_err(),
                        BatchedMerkleTreeError::NonInclusionCheckFailed
                    );
                } else {
                    // Error when inserting into the bloom filter directly twice.
                    assert_eq!(result.unwrap_err(), BloomFilterError::Full.into());
                }
                current_slot += 1;

                // assert_eq!(result.unwrap_err(), BloomFilterError::Full.into());
            }
        }
        // Assert input queue is full and doesn't accept more inserts
        {
            let merkle_tree_account =
                &mut BatchedMerkleTreeAccount::address_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();
            let rnd_bytes = get_rnd_bytes(&mut rng);
            let result = merkle_tree_account.insert_address_into_queue(&rnd_bytes, &current_slot);
            assert_eq!(result.unwrap_err(), BatchedMerkleTreeError::BatchNotReady);
        }
        // Root of the final batch of first input queue batch
        let mut first_input_batch_update_root_value = [0u8; 32];
        let num_updates = 10;
        let mut batch_roots: Vec<(u32, Vec<[u8; 32]>)> = {
            let merkle_tree_account =
                BatchedMerkleTreeAccount::address_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();
            let initial_root = *merkle_tree_account.root_history.last().unwrap();
            vec![(0, vec![initial_root])]
        };
        for i in 0..num_updates {
            println!("address update ----------------------------- {}", i);
            perform_address_update(
                &mut mt_account_data,
                &mut mock_indexer,
                mt_pubkey,
                &mut batch_roots,
            )
            .await;
            if i == 4 {
                first_input_batch_update_root_value = mock_indexer.merkle_tree.root();
            }
            let merkle_tree_account =
                BatchedMerkleTreeAccount::address_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();

            let batch = merkle_tree_account.queue_batches.batches.first().unwrap();
            // assert other batch is not zeroed
            let batch_one = merkle_tree_account.queue_batches.batches.get(1).unwrap();
            assert!(!batch_one.bloom_filter_is_zeroed());

            // after 5 updates the first batch is completely inserted
            // As soon as we switch to inserting the second batch we zero out the first batch since
            // the second batch is completely full.
            if i >= 5 {
                assert!(batch.bloom_filter_is_zeroed());

                // Assert that all unsafe roots from batch 0 are zeroed
                let (_, unsafe_roots) = batch_roots.iter().find(|(idx, _)| *idx == 0).unwrap();
                assert_eq!(unsafe_roots.len(), 6, "batch_roots {:?}", batch_roots);
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
            } else {
                assert!(!batch.bloom_filter_is_zeroed());
            }
        }
        // assert all bloom_filters are inserted
        {
            let merkle_tree_account =
                &mut BatchedMerkleTreeAccount::address_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();
            for (i, batch) in merkle_tree_account.queue_batches.batches.iter().enumerate() {
                assert_eq!(batch.get_state(), BatchState::Inserted);
                if i == 0 {
                    // first batch is zeroed out since the second batch is full
                    assert!(batch.bloom_filter_is_zeroed());
                } else {
                    // second batch is not zeroed out since the first batch is empty
                    assert!(!batch.bloom_filter_is_zeroed());
                }
            }
        }
        {
            let merkle_tree_account =
                &mut BatchedMerkleTreeAccount::address_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();
            println!("root history {:?}", merkle_tree_account.root_history);
            let pre_batch_zero = *merkle_tree_account.queue_batches.batches.first().unwrap();

            for root in merkle_tree_account.root_history.iter() {
                println!("root {:?}", root);
            }
            println!(
                "root in root index {:?}",
                merkle_tree_account.root_history[pre_batch_zero.root_index as usize]
            );
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
                    assert_eq!(merkle_tree_account.root_history[root_index - 1], [0u8; 32]);
                    break;
                }
                println!("index {:?}", index);
                assert_eq!(merkle_tree_account.root_history[index as usize], [0u8; 32]);
            }
        }
    }
}

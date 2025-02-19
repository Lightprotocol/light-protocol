#![allow(unused_assignments)]
use std::cmp::min;

use light_batched_merkle_tree::{
    batch::BatchState,
    constants::{
        ACCOUNT_COMPRESSION_PROGRAM_ID, DEFAULT_BATCH_ADDRESS_TREE_HEIGHT,
        DEFAULT_BATCH_STATE_TREE_HEIGHT, NUM_BATCHES,
    },
    errors::BatchedMerkleTreeError,
    initialize_address_tree::{
        get_address_merkle_tree_account_size_from_params, init_batched_address_merkle_tree_account,
        InitAddressTreeAccountsInstructionData,
    },
    initialize_state_tree::{
        get_state_merkle_tree_account_size_from_params, init_batched_state_merkle_tree_accounts,
        InitStateTreeAccountsInstructionData,
    },
    merkle_tree::{
        assert_batch_adress_event, assert_batch_append_event_event, assert_nullify_event,
        get_merkle_tree_account_size_default, BatchedMerkleTreeAccount,
        InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs,
    },
    merkle_tree_metadata::BatchedMerkleTreeMetadata,
    queue::{
        get_output_queue_account_size_default, get_output_queue_account_size_from_params,
        BatchedQueueAccount, BatchedQueueMetadata,
    },
};
use light_bloom_filter::{BloomFilter, BloomFilterError};
use light_compressed_account::{
    hash_chain::create_hash_chain_from_slice, instruction_data::compressed_proof::CompressedProof,
    pubkey::Pubkey,
};
use light_hasher::{Hasher, Poseidon};
use light_merkle_tree_reference::MerkleTree;
use light_prover_client::{
    gnark::helpers::{spawn_prover, ProofType, ProverConfig},
    mock_batched_forester::{MockBatchedAddressForester, MockBatchedForester, MockTxEvent},
};
use light_zero_copy::vec::ZeroCopyVecU64;
use rand::{rngs::StdRng, Rng};
use serial_test::serial;

#[allow(clippy::too_many_arguments)]
pub fn assert_nullifier_queue_insert(
    pre_account: BatchedMerkleTreeMetadata,
    pre_value_vecs: &mut [ZeroCopyVecU64<[u8; 32]>],
    pre_roots: Vec<[u8; 32]>,
    pre_hash_chains: [ZeroCopyVecU64<[u8; 32]>; 2],
    merkle_tree_account: BatchedMerkleTreeAccount,
    bloom_filter_insert_values: Vec<[u8; 32]>,
    leaf_indices: Vec<u64>,
    tx_hash: [u8; 32],
    input_is_in_tree: Vec<bool>,
    array_indices: Vec<usize>,
    current_slot: &u64,
) -> Result<(), BatchedMerkleTreeError> {
    let mut leaf_hash_chain_insert_values = vec![];
    for (insert_value, leaf_index) in bloom_filter_insert_values.iter().zip(leaf_indices.iter()) {
        let nullifier =
            Poseidon::hashv(&[insert_value.as_slice(), &leaf_index.to_be_bytes(), &tx_hash])
                .unwrap();
        leaf_hash_chain_insert_values.push(nullifier);
    }
    assert_input_queue_insert(
        pre_account,
        pre_value_vecs,
        pre_roots,
        pre_hash_chains,
        merkle_tree_account,
        bloom_filter_insert_values,
        leaf_hash_chain_insert_values,
        input_is_in_tree,
        array_indices,
        current_slot,
    )
}
/// Insert into input queue:
/// 1. New value exists in the current batch bloom_filter
/// 2. New value does not exist in the other batch bloom_filters
#[allow(clippy::too_many_arguments)]
pub fn assert_input_queue_insert(
    mut pre_account: BatchedMerkleTreeMetadata,
    pre_value_vecs: &mut [ZeroCopyVecU64<[u8; 32]>],
    pre_roots: Vec<[u8; 32]>,
    mut pre_hash_chains: [ZeroCopyVecU64<[u8; 32]>; 2],
    mut merkle_tree_account: BatchedMerkleTreeAccount,
    bloom_filter_insert_values: Vec<[u8; 32]>,
    leaf_hash_chain_insert_values: Vec<[u8; 32]>,
    input_is_in_tree: Vec<bool>,
    array_indices: Vec<usize>,
    current_slot: &u64,
) -> Result<(), BatchedMerkleTreeError> {
    let mut should_be_zeroed = false;
    for (i, insert_value) in bloom_filter_insert_values.iter().enumerate() {
        if !input_is_in_tree[i] {
            let value_vec_index = array_indices[i];
            assert!(
                pre_value_vecs.iter_mut().any(|value_vec| {
                    if value_vec.len() > value_vec_index {
                        {
                            if value_vec[value_vec_index] == *insert_value {
                                value_vec[value_vec_index] = [0u8; 32];
                                true
                            } else {
                                false
                            }
                        }
                    } else {
                        false
                    }
                }),
                "Value not in value vec."
            );
        }

        let post_roots: Vec<[u8; 32]> = merkle_tree_account.root_history.iter().cloned().collect();
        // if root buffer changed it must be only overwritten by [0u8;32]
        if post_roots != pre_roots {
            let only_zero_overwrites = post_roots
                .iter()
                .zip(pre_roots.iter())
                .all(|(post, pre)| *post == *pre || *post == [0u8; 32]);
            println!("pre_roots: {:?}", pre_roots);
            println!("post_roots: {:?}", post_roots);
            if !only_zero_overwrites {
                panic!("Root buffer changed.")
            }
        }

        let inserted_batch_index =
            pre_account.queue_batches.currently_processing_batch_index as usize;
        let expected_batch = pre_account
            .queue_batches
            .batches
            .get_mut(inserted_batch_index)
            .unwrap();

        pre_account.queue_batches.next_index += 1;

        println!(
            "assert input queue batch update: expected_batch: {:?}",
            expected_batch
        );
        println!(
            "assert input queue batch update: expected_batch.get_num_inserted_elements(): {}",
            expected_batch.get_num_inserted_elements()
        );
        println!(
            "assert input queue batch update: expected_batch.batch_size / 2: {}",
            expected_batch.batch_size / 2
        );

        if !should_be_zeroed && expected_batch.get_state() == BatchState::Inserted {
            should_be_zeroed =
                expected_batch.get_num_inserted_elements() == expected_batch.batch_size / 2;
        }
        println!(
            "assert input queue batch update: should_be_zeroed: {}",
            should_be_zeroed
        );
        if expected_batch.get_state() == BatchState::Inserted {
            println!("assert input queue batch update: clearing batch");
            pre_hash_chains[inserted_batch_index].clear();
            expected_batch.advance_state_to_fill(None).unwrap();
            expected_batch.set_start_slot(current_slot);
            println!("setting start slot to {}", current_slot);
        } else if expected_batch.get_state() == BatchState::Fill
            && !expected_batch.start_slot_is_set()
        {
            // Batch is filled for the first time
            expected_batch.set_start_slot(current_slot);
        }
        println!(
            "assert input queue batch update: inserted_batch_index: {}",
            inserted_batch_index
        );
        // New value exists in the current batch bloom filter
        let mut bloom_filter = BloomFilter::new(
            merkle_tree_account.queue_batches.batches[inserted_batch_index].num_iters as usize,
            merkle_tree_account.queue_batches.batches[inserted_batch_index].bloom_filter_capacity,
            merkle_tree_account.bloom_filter_stores[inserted_batch_index],
        )
        .unwrap();
        println!(
            "assert input queue batch update: insert_value: {:?}",
            insert_value
        );
        assert!(bloom_filter.contains(insert_value));
        let pre_hash_chain = pre_hash_chains.get_mut(inserted_batch_index).unwrap();
        expected_batch.add_to_hash_chain(&leaf_hash_chain_insert_values[i], pre_hash_chain)?;

        let num_iters =
            merkle_tree_account.queue_batches.batches[inserted_batch_index].num_iters as usize;
        let bloom_filter_capacity =
            merkle_tree_account.queue_batches.batches[inserted_batch_index].bloom_filter_capacity;
        // New value does not exist in the other batch bloom_filters
        for (i, store) in merkle_tree_account
            .bloom_filter_stores
            .iter_mut()
            .enumerate()
        {
            // Skip current batch it is already checked above
            if i != inserted_batch_index {
                let mut bloom_filter =
                    BloomFilter::new(num_iters, bloom_filter_capacity, store).unwrap();
                assert!(!bloom_filter.contains(insert_value));
            }
        }
        // if the currently processing batch changed it should
        // increment by one and the old batch should be ready to
        // update
        if expected_batch.get_current_zkp_batch_index() == expected_batch.get_num_zkp_batches() {
            assert_eq!(
                merkle_tree_account.queue_batches.batches
                    [pre_account.queue_batches.currently_processing_batch_index as usize]
                    .get_state(),
                BatchState::Full
            );
            pre_account.queue_batches.currently_processing_batch_index += 1;
            pre_account.queue_batches.currently_processing_batch_index %=
                pre_account.queue_batches.num_batches;
            assert_eq!(
                merkle_tree_account.queue_batches.batches[inserted_batch_index],
                *expected_batch
            );
            assert_eq!(
                merkle_tree_account.hash_chain_stores[inserted_batch_index]
                    .last()
                    .unwrap(),
                pre_hash_chain.last().unwrap(),
                "Hashchain store inconsistent."
            );
        }
    }

    assert_eq!(
        *merkle_tree_account.get_metadata(),
        pre_account,
        "BatchedMerkleTreeMetadata changed."
    );
    let inserted_batch_index = pre_account.queue_batches.currently_processing_batch_index as usize;
    let mut expected_batch = pre_account.queue_batches.batches[inserted_batch_index];
    if should_be_zeroed {
        expected_batch.set_bloom_filter_to_zeroed();
    }
    assert_eq!(
        merkle_tree_account.queue_batches.batches[inserted_batch_index],
        expected_batch
    );
    let other_batch = if inserted_batch_index == 0 { 1 } else { 0 };
    assert_eq!(
        merkle_tree_account.queue_batches.batches[other_batch],
        pre_account.queue_batches.batches[other_batch]
    );
    assert_eq!(
        merkle_tree_account.hash_chain_stores, pre_hash_chains,
        "Hashchain store inconsistent."
    );
    Ok(())
}

/// Expected behavior for insert into output queue:
/// - add value to value array
/// - batch.num_inserted += 1
/// - if batch is full after insertion advance state to ReadyToUpdateTree
pub fn assert_output_queue_insert(
    mut pre_account: BatchedQueueMetadata,
    // mut pre_batches: Vec<Batch>,
    mut pre_value_store: [ZeroCopyVecU64<[u8; 32]>; 2],
    mut pre_hash_chains: [ZeroCopyVecU64<[u8; 32]>; 2],
    mut output_account: BatchedQueueAccount,
    insert_values: Vec<[u8; 32]>,
    current_slot: u64,
) -> Result<(), BatchedMerkleTreeError> {
    for batch in output_account.batch_metadata.batches.iter_mut() {
        println!("output_account.batch: {:?}", batch);
    }
    for batch in pre_account.batch_metadata.batches.iter() {
        println!("pre_batch: {:?}", batch);
    }
    for insert_value in insert_values.iter() {
        // if the currently processing batch changed it should
        // increment by one and the old batch should be ready to
        // update

        let inserted_batch_index =
            pre_account.batch_metadata.currently_processing_batch_index as usize;
        let expected_batch = &mut pre_account.batch_metadata.batches[inserted_batch_index];
        let pre_value_store = pre_value_store.get_mut(inserted_batch_index).unwrap();
        let pre_hash_chain = pre_hash_chains.get_mut(inserted_batch_index).unwrap();
        if expected_batch.get_state() == BatchState::Inserted {
            expected_batch
                .advance_state_to_fill(Some(pre_account.batch_metadata.next_index))
                .unwrap();
            pre_value_store.clear();
            pre_hash_chain.clear();
        }
        pre_account.batch_metadata.next_index += 1;
        expected_batch.store_and_hash_value(
            insert_value,
            pre_value_store,
            pre_hash_chain,
            &current_slot,
        )?;

        let other_batch = if inserted_batch_index == 0 { 1 } else { 0 };
        assert!(output_account.value_vecs[inserted_batch_index]
            .as_mut_slice()
            .to_vec()
            .contains(insert_value));
        assert!(!output_account.value_vecs[other_batch]
            .as_mut_slice()
            .to_vec()
            .contains(insert_value));
        if expected_batch.get_num_zkp_batches() == expected_batch.get_current_zkp_batch_index() {
            assert_eq!(
                output_account.batch_metadata.batches
                    [pre_account.batch_metadata.currently_processing_batch_index as usize]
                    .get_state(),
                BatchState::Full
            );
            pre_account.batch_metadata.currently_processing_batch_index += 1;
            pre_account.batch_metadata.currently_processing_batch_index %=
                pre_account.batch_metadata.num_batches;
            assert_eq!(
                output_account.batch_metadata.batches[inserted_batch_index],
                *expected_batch
            );
        }
    }
    assert_eq!(
        *output_account.get_metadata(),
        pre_account,
        "BatchedQueueAccount changed."
    );
    assert_eq!(pre_hash_chains, output_account.hash_chain_stores);
    for (i, (value_store, pre)) in output_account
        .value_vecs
        .iter()
        .zip(pre_value_store.iter())
        .enumerate()
    {
        for (j, (value, pre_value)) in value_store.iter().zip(pre.iter()).enumerate() {
            assert_eq!(
                *value, *pre_value,
                "{} {} \n value store {:?}\n pre {:?}",
                i, j, value_store, pre
            );
        }
    }
    assert_eq!(pre_value_store, output_account.value_vecs);
    Ok(())
}

#[derive(Debug, PartialEq, Clone)]
pub struct MockTransactionInputs {
    inputs: Vec<[u8; 32]>,
    outputs: Vec<[u8; 32]>,
}

pub fn simulate_transaction(
    instruction_data: MockTransactionInputs,
    merkle_tree_account_data: &mut [u8],
    output_queue_account_data: &mut [u8],
    reference_merkle_tree: &MerkleTree<Poseidon>,
    current_slot: &mut u64,
    mt_pubkey: &Pubkey,
) -> Result<MockTxEvent, BatchedMerkleTreeError> {
    let mut output_account =
        BatchedQueueAccount::output_from_bytes(output_queue_account_data).unwrap();
    let mut merkle_tree_account =
        BatchedMerkleTreeAccount::state_from_bytes(merkle_tree_account_data, mt_pubkey).unwrap();
    let flattened_inputs = instruction_data
        .inputs
        .iter()
        .cloned()
        .chain(instruction_data.outputs.iter().cloned())
        .collect::<Vec<[u8; 32]>>();
    let tx_hash = create_hash_chain_from_slice(flattened_inputs.as_slice())?;

    for input in instruction_data.inputs.iter() {
        // zkp inclusion in Merkle tree
        let inclusion = reference_merkle_tree.get_leaf_index(input);
        let leaf_index = if let Some(leaf_index) = inclusion {
            leaf_index as u64
        } else {
            println!("simulate_transaction: inclusion is none");
            let mut included = false;
            let mut leaf_index = 0;
            let start_indices = output_account
                .batch_metadata
                .batches
                .iter()
                .map(|batch| batch.start_index)
                .collect::<Vec<u64>>();

            for (batch_index, value_vec) in output_account.value_vecs.iter_mut().enumerate() {
                for (value_index, value) in value_vec.iter_mut().enumerate() {
                    if *value == *input {
                        let batch_start_index = start_indices[batch_index];
                        included = true;
                        println!("overwriting value: {:?}", value);
                        *value = [0u8; 32];
                        leaf_index = value_index as u64 + batch_start_index;
                    }
                }
            }
            if !included {
                panic!("Value not included in any output queue or trees.");
            }
            leaf_index
        };

        println!(
            "sim tx input: \n {:?} \nleaf index : {:?}, \ntx hash {:?}",
            input, leaf_index, tx_hash,
        );
        merkle_tree_account.insert_nullifier_into_queue(
            input,
            leaf_index,
            &tx_hash,
            current_slot,
        )?;
    }

    for output in instruction_data.outputs.iter() {
        let leaf_index = output_account.batch_metadata.next_index;
        println!(
            "sim tx output: \n  {:?} \nleaf index : {:?}",
            output, leaf_index
        );
        output_account.insert_into_current_batch(output, current_slot)?;
    }
    Ok(MockTxEvent {
        inputs: instruction_data.inputs.clone(),
        outputs: instruction_data.outputs.clone(),
        tx_hash,
    })
}

#[serial]
#[tokio::test]
async fn test_simulate_transactions() {
    spawn_prover(
        true,
        ProverConfig {
            run_mode: None,
            circuits: vec![
                ProofType::BatchAppendWithProofsTest,
                ProofType::BatchUpdateTest,
            ],
        },
    )
    .await;
    let mut mock_indexer =
        MockBatchedForester::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>::default();

    let num_tx = 2200;
    let owner = Pubkey::new_unique();

    let queue_account_size = get_output_queue_account_size_default();

    let mut output_queue_account_data = vec![0; queue_account_size];
    let output_queue_pubkey = Pubkey::new_unique();

    let mt_account_size = get_merkle_tree_account_size_default();
    let mut mt_account_data = vec![0; mt_account_size];
    let mt_pubkey = ACCOUNT_COMPRESSION_PROGRAM_ID.into();

    let params = InitStateTreeAccountsInstructionData::test_default();

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
    let mut in_ready_for_update = false;
    let mut out_ready_for_update = false;
    let mut num_output_updates = 0;
    let mut num_input_updates = 0;
    let mut num_input_values = 0;
    let mut num_output_values = 0;
    let mut current_slot = rng.gen();

    for tx in 0..num_tx {
        println!("tx: {}", tx);
        println!("num_input_updates: {}", num_input_updates);
        println!("num_output_updates: {}", num_output_updates);
        {
            println!("Simulate tx {} -----------------------------", tx);
            println!("Num inserted values: {}", num_input_values);
            println!("Num input updates: {}", num_input_updates);
            println!("Num output updates: {}", num_output_updates);
            println!("Num output values: {}", num_output_values);
            let number_of_outputs = rng.gen_range(0..7);
            let mut outputs = vec![];
            for _ in 0..number_of_outputs {
                outputs.push(get_rnd_bytes(&mut rng));
            }
            let number_of_inputs = if rng.gen_bool(0.5) {
                if !mock_indexer.active_leaves.is_empty() {
                    let x = min(mock_indexer.active_leaves.len(), 5);
                    rng.gen_range(0..x)
                } else {
                    0
                }
            } else {
                0
            };

            let mut inputs = vec![];
            let mut input_is_in_tree = vec![];
            let mut leaf_indices = vec![];
            let mut array_indices = vec![];
            let mut retries = min(10, mock_indexer.active_leaves.len());
            while inputs.len() < number_of_inputs && retries > 0 {
                let (_, leaf) = get_random_leaf(&mut rng, &mut mock_indexer.active_leaves);
                let inserted = mock_indexer.merkle_tree.get_leaf_index(&leaf);
                if let Some(leaf_index) = inserted {
                    inputs.push(leaf);
                    leaf_indices.push(leaf_index as u64);
                    input_is_in_tree.push(true);
                    array_indices.push(0);
                } else if rng.gen_bool(0.1) {
                    inputs.push(leaf);
                    let output_queue =
                        BatchedQueueAccount::output_from_bytes(&mut output_queue_account_data)
                            .unwrap();
                    let mut leaf_array_index = 0;
                    let mut batch_index = 0;
                    for (i, vec) in output_queue.value_vecs.iter().enumerate() {
                        let pos = vec.iter().position(|value| *value == leaf);
                        if let Some(pos) = pos {
                            leaf_array_index = pos;
                            batch_index = i;
                            break;
                        }
                        if i == output_queue.value_vecs.len() - 1 {
                            panic!("Leaf not found in output queue.");
                        }
                    }
                    let batch = output_queue
                        .batch_metadata
                        .batches
                        .get(batch_index)
                        .unwrap();
                    array_indices.push(leaf_array_index);
                    let leaf_index: u64 = batch.start_index + leaf_array_index as u64;
                    leaf_indices.push(leaf_index);
                    input_is_in_tree.push(false);
                }
                retries -= 1;
            }
            let number_of_inputs = inputs.len();
            println!("number_of_inputs: {}", number_of_inputs);

            let instruction_data = MockTransactionInputs {
                inputs: inputs.clone(),
                outputs: outputs.clone(),
            };

            let merkle_tree_account =
                BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();
            println!(
                "input queue: {:?}",
                merkle_tree_account.queue_batches.batches[0].get_num_inserted_zkp_batch()
            );

            let mut pre_mt_data = mt_account_data.clone();
            let mut pre_account_bytes = output_queue_account_data.clone();

            let pre_output_account =
                BatchedQueueAccount::output_from_bytes(&mut pre_account_bytes).unwrap();
            let pre_output_metadata = *pre_output_account.get_metadata();
            let mut pre_output_value_stores = pre_output_account.value_vecs;
            let pre_output_hash_chains = pre_output_account.hash_chain_stores;

            let mut pre_mt_account_bytes = mt_account_data.clone();
            let pre_merkle_tree_account =
                BatchedMerkleTreeAccount::state_from_bytes(&mut pre_mt_account_bytes, &mt_pubkey)
                    .unwrap();
            let pre_mt_account = *pre_merkle_tree_account.get_metadata();
            let pre_roots = pre_merkle_tree_account
                .root_history
                .iter()
                .cloned()
                .collect();
            let pre_mt_hash_chains = pre_merkle_tree_account.hash_chain_stores;

            if !outputs.is_empty() || !inputs.is_empty() {
                println!("Simulating tx with inputs: {:?}", instruction_data);
                let event = simulate_transaction(
                    instruction_data,
                    &mut pre_mt_data,
                    &mut output_queue_account_data,
                    &mock_indexer.merkle_tree,
                    &mut current_slot,
                    &mt_pubkey,
                )
                .unwrap();
                mock_indexer.tx_events.push(event.clone());

                if !inputs.is_empty() {
                    let merkle_tree_account =
                        BatchedMerkleTreeAccount::state_from_bytes(&mut pre_mt_data, &mt_pubkey)
                            .unwrap();
                    println!("inputs: {:?}", inputs);
                    assert_nullifier_queue_insert(
                        pre_mt_account,
                        &mut pre_output_value_stores, // mut to remove values proven by index
                        pre_roots,
                        pre_mt_hash_chains,
                        merkle_tree_account,
                        inputs.clone(),
                        leaf_indices.clone(),
                        event.tx_hash,
                        input_is_in_tree,
                        array_indices,
                        &current_slot,
                    )
                    .unwrap();
                }

                if !outputs.is_empty() {
                    assert_output_queue_insert(
                        pre_output_metadata,
                        pre_output_value_stores,
                        pre_output_hash_chains,
                        BatchedQueueAccount::output_from_bytes(
                            &mut output_queue_account_data.clone(), // clone so that data cannot be modified
                        )
                        .unwrap(),
                        outputs.clone(),
                        current_slot,
                    )
                    .unwrap();
                }

                for i in 0..number_of_inputs {
                    mock_indexer
                        .input_queue_leaves
                        .push((inputs[i], leaf_indices[i] as usize));
                }
                for output in outputs.iter() {
                    mock_indexer.active_leaves.push(*output);
                    mock_indexer.output_queue_leaves.push(*output);
                }

                num_output_values += number_of_outputs;
                num_input_values += number_of_inputs;
                let merkle_tree_account =
                    BatchedMerkleTreeAccount::state_from_bytes(&mut pre_mt_data, &mt_pubkey)
                        .unwrap();
                in_ready_for_update = merkle_tree_account
                    .queue_batches
                    .batches
                    .iter()
                    .any(|batch| batch.get_first_ready_zkp_batch().is_ok());
                let output_account =
                    BatchedQueueAccount::output_from_bytes(&mut output_queue_account_data).unwrap();
                out_ready_for_update = output_account
                    .batch_metadata
                    .batches
                    .iter()
                    .any(|batch| batch.get_first_ready_zkp_batch().is_ok());

                mt_account_data = pre_mt_data.clone();
            } else {
                println!("Skipping simulate tx for no inputs or outputs");
            }
            current_slot += 1;
        }

        if in_ready_for_update && rng.gen_bool(1.0) {
            println!("Input update -----------------------------");
            println!("Num inserted values: {}", num_input_values);
            println!("Num input updates: {}", num_input_updates);
            println!("Num output updates: {}", num_output_updates);
            println!("Num output values: {}", num_output_values);
            let mut pre_mt_account_data = mt_account_data.clone();
            let old_account =
                BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();
            let (input_res, new_root) = {
                let mut account = BatchedMerkleTreeAccount::state_from_bytes(
                    &mut pre_mt_account_data,
                    &mt_pubkey,
                )
                .unwrap();
                println!("batches {:?}", account.queue_batches.batches);

                let next_full_batch = account.get_metadata().queue_batches.pending_batch_index;
                let batch = account
                    .queue_batches
                    .batches
                    .get(next_full_batch as usize)
                    .unwrap();
                println!(
                    "account
                        .hash_chain_stores {:?}",
                    account.hash_chain_stores
                );
                println!("hash_chain store len {:?}", account.hash_chain_stores.len());
                println!(
                    "batch.get_num_inserted_zkps() as usize {:?}",
                    batch.get_num_inserted_zkps() as usize
                );
                let leaves_hash_chain = account
                    .hash_chain_stores
                    .get(next_full_batch as usize)
                    .unwrap()
                    .get(batch.get_num_inserted_zkps() as usize)
                    .unwrap();

                let (proof, new_root) = mock_indexer
                    .get_batched_update_proof(
                        account.get_metadata().queue_batches.zkp_batch_size as u32,
                        *leaves_hash_chain,
                    )
                    .await
                    .unwrap();
                let instruction_data = InstructionDataBatchNullifyInputs {
                    new_root,
                    compressed_proof: CompressedProof {
                        a: proof.a,
                        b: proof.b,
                        c: proof.c,
                    },
                };

                (
                    account.update_tree_from_input_queue(instruction_data),
                    new_root,
                )
            };
            println!("Input update -----------------------------");
            println!("res {:?}", input_res);
            assert!(input_res.is_ok());
            let nullify_event = input_res.unwrap();
            in_ready_for_update = false;
            // assert Merkle tree
            // sequence number increased X
            // next index increased X
            // current root index increased X
            // One root changed one didn't

            let account =
                BatchedMerkleTreeAccount::state_from_bytes(&mut pre_mt_account_data, &mt_pubkey)
                    .unwrap();
            assert_nullify_event(nullify_event, new_root, &old_account, mt_pubkey);
            assert_merkle_tree_update(old_account, account, None, None, new_root);
            mt_account_data = pre_mt_account_data.clone();

            num_input_updates += 1;
        }

        if out_ready_for_update && rng.gen_bool(1.0) {
            println!("Output update -----------------------------");
            println!("Num inserted values: {}", num_input_values);
            println!("Num input updates: {}", num_input_updates);
            println!("Num output updates: {}", num_output_updates);
            println!("Num output values: {}", num_output_values);

            let mut pre_mt_account_data = mt_account_data.clone();
            let mut account =
                BatchedMerkleTreeAccount::state_from_bytes(&mut pre_mt_account_data, &mt_pubkey)
                    .unwrap();
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

            let instruction_data = InstructionDataBatchAppendInputs {
                new_root,
                compressed_proof: CompressedProof {
                    a: proof.a,
                    b: proof.b,
                    c: proof.c,
                },
            };

            let mut pre_output_queue_state = output_queue_account_data.clone();
            println!("Output update -----------------------------");

            let queue_account =
                &mut BatchedQueueAccount::output_from_bytes(&mut pre_output_queue_state).unwrap();
            let output_res =
                account.update_tree_from_output_queue_account(queue_account, instruction_data);
            println!("output_res: {:?}", output_res);
            assert!(output_res.is_ok());
            let batch_append_event = output_res.unwrap();

            assert_eq!(
                *account.root_history.last().unwrap(),
                mock_indexer.merkle_tree.root()
            );
            let output_account =
                BatchedQueueAccount::output_from_bytes(&mut pre_output_queue_state).unwrap();
            let old_output_account =
                BatchedQueueAccount::output_from_bytes(&mut output_queue_account_data).unwrap();

            let old_account =
                BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();

            println!("batch 0: {:?}", output_account.batch_metadata.batches[0]);
            println!("batch 1: {:?}", output_account.batch_metadata.batches[1]);
            assert_batch_append_event_event(
                batch_append_event,
                new_root,
                &old_output_account,
                &old_account,
                mt_pubkey,
            );
            assert_merkle_tree_update(
                old_account,
                account,
                Some(old_output_account),
                Some(output_account),
                new_root,
            );

            output_queue_account_data = pre_output_queue_state;
            mt_account_data = pre_mt_account_data;
            out_ready_for_update = false;
            num_output_updates += 1;
        }
    }
    let output_account =
        BatchedQueueAccount::output_from_bytes(&mut output_queue_account_data).unwrap();
    println!("batch 0: {:?}", output_account.batch_metadata.batches[0]);
    println!("batch 1: {:?}", output_account.batch_metadata.batches[1]);
    println!("num_output_updates: {}", num_output_updates);
    println!("num_input_updates: {}", num_input_updates);
    println!("num_output_values: {}", num_output_values);
    println!("num_input_values: {}", num_input_values);
}

// Get random leaf that is not in the input queue.
pub fn get_random_leaf(rng: &mut StdRng, active_leaves: &mut Vec<[u8; 32]>) -> (usize, [u8; 32]) {
    if active_leaves.is_empty() {
        return (0, [0u8; 32]);
    }
    let index = rng.gen_range(0..active_leaves.len());
    // get random leaf from vector and remove it
    (index, active_leaves.remove(index))
}

/// queues with a counter which keeps things below X tps and an if that
/// executes tree updates when possible.
#[serial]
#[tokio::test]
async fn test_e2e() {
    spawn_prover(
        true,
        ProverConfig {
            run_mode: None,
            circuits: vec![
                ProofType::BatchAppendWithProofsTest,
                ProofType::BatchUpdateTest,
            ],
        },
    )
    .await;
    let mut mock_indexer =
        MockBatchedForester::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>::default();

    let num_tx = 2200;
    let owner = Pubkey::new_unique();

    let queue_account_size = get_output_queue_account_size_default();

    let mut output_queue_account_data = vec![0; queue_account_size];
    let output_queue_pubkey = Pubkey::new_unique();

    let mt_account_size = get_merkle_tree_account_size_default();
    let mut mt_account_data = vec![0; mt_account_size];
    let mt_pubkey = Pubkey::new_unique();

    let params = InitStateTreeAccountsInstructionData::test_default();

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
    let mut in_ready_for_update;
    let mut out_ready_for_update;
    let mut num_output_updates = 0;
    let mut num_input_updates = 0;
    let mut num_input_values = 0;
    let mut num_output_values = 0;
    let mut current_slot = rng.gen();

    for tx in 0..num_tx {
        println!("tx: {}", tx);
        println!("num_input_updates: {}", num_input_updates);
        println!("num_output_updates: {}", num_output_updates);
        // Output queue
        {
            if rng.gen_bool(0.5) {
                println!("Output insert -----------------------------");
                println!("num_output_values: {}", num_output_values);
                let rnd_bytes = get_rnd_bytes(&mut rng);
                let mut pre_account_bytes = output_queue_account_data.clone();
                let pre_output_account =
                    BatchedQueueAccount::output_from_bytes(&mut pre_account_bytes).unwrap();
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
                num_output_values += 1;
                mock_indexer.output_queue_leaves.push(rnd_bytes);
            }
            let output_account =
                BatchedQueueAccount::output_from_bytes(&mut output_queue_account_data).unwrap();
            out_ready_for_update = output_account
                .batch_metadata
                .batches
                .iter()
                .any(|batch| batch.get_state() == BatchState::Full);
        }

        // Input queue
        {
            let mut pre_account_bytes = mt_account_data.clone();

            if rng.gen_bool(0.5) && !mock_indexer.active_leaves.is_empty() {
                println!("Input insert -----------------------------");
                let (_, leaf) = get_random_leaf(&mut rng, &mut mock_indexer.active_leaves);

                let pre_mt_account =
                    BatchedMerkleTreeAccount::state_from_bytes(&mut pre_account_bytes, &mt_pubkey)
                        .unwrap();
                let pre_account = *pre_mt_account.get_metadata();
                let pre_hash_chains = pre_mt_account.hash_chain_stores;
                let pre_roots = pre_mt_account.root_history.iter().cloned().collect();
                let tx_hash = create_hash_chain_from_slice(vec![leaf].as_slice()).unwrap();
                let leaf_index = mock_indexer.merkle_tree.get_leaf_index(&leaf).unwrap();
                mock_indexer.input_queue_leaves.push((leaf, leaf_index));
                mock_indexer.tx_events.push(MockTxEvent {
                    inputs: vec![leaf],
                    outputs: vec![],
                    tx_hash,
                });
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

                {
                    let mut mt_account_data = mt_account_data.clone();
                    let merkle_tree_account = BatchedMerkleTreeAccount::state_from_bytes(
                        &mut mt_account_data,
                        &mt_pubkey,
                    )
                    .unwrap();
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
                }
                num_input_values += 1;
            }
            let merkle_tree_account =
                BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();

            in_ready_for_update = merkle_tree_account
                .queue_batches
                .batches
                .iter()
                .any(|batch| batch.get_state() == BatchState::Full);
        }

        if in_ready_for_update {
            println!("Input update -----------------------------");
            println!("Num inserted values: {}", num_input_values);
            println!("Num input updates: {}", num_input_updates);
            println!("Num output updates: {}", num_output_updates);
            println!("Num output values: {}", num_output_values);
            let mut pre_mt_account_data = mt_account_data.clone();
            in_ready_for_update = false;
            perform_input_update(&mut pre_mt_account_data, &mut mock_indexer, true, mt_pubkey)
                .await;
            mt_account_data = pre_mt_account_data.clone();

            num_input_updates += 1;
        }

        if out_ready_for_update {
            println!("Output update -----------------------------");
            println!("Num inserted values: {}", num_input_values);
            println!("Num input updates: {}", num_input_updates);
            println!("Num output updates: {}", num_output_updates);
            println!("Num output values: {}", num_output_values);
            let mut pre_mt_account_data = mt_account_data.clone();
            let mut account =
                BatchedMerkleTreeAccount::state_from_bytes(&mut pre_mt_account_data, &mt_pubkey)
                    .unwrap();
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
            let leaves = output_account
                .value_vecs
                .get(next_full_batch as usize)
                .unwrap()
                .to_vec();
            println!("leaves {:?}", leaves.len());
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

            let mut pre_output_queue_state = output_queue_account_data.clone();
            println!("Output update -----------------------------");

            let queue_account =
                &mut BatchedQueueAccount::output_from_bytes(&mut pre_output_queue_state).unwrap();
            let output_res =
                account.update_tree_from_output_queue_account(queue_account, instruction_data);

            assert_eq!(
                *account.root_history.last().unwrap(),
                mock_indexer.merkle_tree.root()
            );
            println!(
                "post update: sequence number: {}",
                account.get_metadata().sequence_number
            );
            println!("output_res {:?}", output_res);
            assert!(output_res.is_ok());

            println!("output update success {}", num_output_updates);
            println!("num_output_values: {}", num_output_values);
            println!("num_input_values: {}", num_input_values);
            let output_account =
                BatchedQueueAccount::output_from_bytes(&mut pre_output_queue_state).unwrap();
            let old_output_account =
                BatchedQueueAccount::output_from_bytes(&mut output_queue_account_data).unwrap();

            let old_account =
                BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();

            println!("batch 0: {:?}", output_account.batch_metadata.batches[0]);
            println!("batch 1: {:?}", output_account.batch_metadata.batches[1]);
            assert_merkle_tree_update(
                old_account,
                account,
                Some(old_output_account),
                Some(output_account),
                new_root,
            );

            output_queue_account_data = pre_output_queue_state;
            mt_account_data = pre_mt_account_data;
            out_ready_for_update = false;
            num_output_updates += 1;
        }
    }
    let output_account =
        BatchedQueueAccount::output_from_bytes(&mut output_queue_account_data).unwrap();
    println!("batch 0: {:?}", output_account.batch_metadata.batches[0]);
    println!("batch 1: {:?}", output_account.batch_metadata.batches[1]);
    println!("num_output_updates: {}", num_output_updates);
    println!("num_input_updates: {}", num_input_updates);
    println!("num_output_values: {}", num_output_values);
    println!("num_input_values: {}", num_input_values);
}
pub async fn perform_input_update(
    mt_account_data: &mut [u8],
    mock_indexer: &mut MockBatchedForester<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>,
    enable_assert: bool,
    mt_pubkey: Pubkey,
) {
    let mut cloned_mt_account_data = (*mt_account_data).to_vec();
    let old_account = BatchedMerkleTreeAccount::state_from_bytes(
        cloned_mt_account_data.as_mut_slice(),
        &mt_pubkey,
    )
    .unwrap();
    let (input_res, root) = {
        let mut account =
            BatchedMerkleTreeAccount::state_from_bytes(mt_account_data, &mt_pubkey).unwrap();

        let next_full_batch = account.get_metadata().queue_batches.pending_batch_index;
        let batch = account
            .queue_batches
            .batches
            .get(next_full_batch as usize)
            .unwrap();
        let leaves_hash_chain = account
            .hash_chain_stores
            .get(next_full_batch as usize)
            .unwrap()
            .get(batch.get_num_inserted_zkps() as usize)
            .unwrap();
        let (proof, new_root) = mock_indexer
            .get_batched_update_proof(
                account.get_metadata().queue_batches.zkp_batch_size as u32,
                *leaves_hash_chain,
            )
            .await
            .unwrap();
        let instruction_data = InstructionDataBatchNullifyInputs {
            new_root,
            compressed_proof: CompressedProof {
                a: proof.a,
                b: proof.b,
                c: proof.c,
            },
        };

        (
            account.update_tree_from_input_queue(instruction_data),
            new_root,
        )
    };
    println!("Input update -----------------------------");
    println!("res {:?}", input_res);
    assert!(input_res.is_ok());

    // assert Merkle tree
    // sequence number increased X
    // next index increased X
    // current root index increased X
    // One root changed one didn't

    let account = BatchedMerkleTreeAccount::state_from_bytes(mt_account_data, &mt_pubkey).unwrap();
    if enable_assert {
        assert_merkle_tree_update(old_account, account, None, None, root);
    }
}

pub async fn perform_address_update(
    mt_account_data: &mut [u8],
    mock_indexer: &mut MockBatchedAddressForester<40>,
    mt_pubkey: Pubkey,
) {
    println!("pre address update -----------------------------");
    let mut cloned_mt_account_data = (*mt_account_data).to_vec();
    let old_account = BatchedMerkleTreeAccount::address_from_bytes(
        cloned_mt_account_data.as_mut_slice(),
        &mt_pubkey,
    )
    .unwrap();
    let (input_res, new_root, _pre_next_full_batch) = {
        let mut account =
            BatchedMerkleTreeAccount::address_from_bytes(mt_account_data, &mt_pubkey).unwrap();

        let next_full_batch = account.get_metadata().queue_batches.pending_batch_index;
        let next_index = account.get_metadata().next_index;
        println!("next index {:?}", next_index);
        let batch = account
            .queue_batches
            .batches
            .get(next_full_batch as usize)
            .unwrap();
        let batch_start_index =
            batch.start_index + batch.get_num_inserted_zkps() * batch.zkp_batch_size;
        println!("batch start index {}", batch_start_index);
        let leaves_hash_chain = account
            .hash_chain_stores
            .get(next_full_batch as usize)
            .unwrap()
            .get(batch.get_num_inserted_zkps() as usize)
            .unwrap();
        let current_root = account.root_history.last().unwrap();
        let (proof, new_root) = mock_indexer
            .get_batched_address_proof(
                account.get_metadata().queue_batches.batch_size as u32,
                account.get_metadata().queue_batches.zkp_batch_size as u32,
                *leaves_hash_chain,
                next_index as usize, // % batch.zkp_batch_size as usize
                batch_start_index as usize,
                *current_root,
            )
            .await
            .unwrap();

        mock_indexer.finalize_batch_address_update(10);
        let instruction_data = InstructionDataBatchNullifyInputs {
            new_root,
            compressed_proof: CompressedProof {
                a: proof.a,
                b: proof.b,
                c: proof.c,
            },
        };

        (
            account.update_tree_from_address_queue(instruction_data),
            new_root,
            next_full_batch,
        )
    };
    println!("post address update -----------------------------");
    println!("res {:?}", input_res);
    assert!(input_res.is_ok());
    let event = input_res.unwrap();
    assert_batch_adress_event(event, new_root, &old_account, mt_pubkey);

    // assert Merkle tree
    // sequence number increased X
    // next index increased X
    // current root index increased X
    // One root changed one didn't

    let account =
        BatchedMerkleTreeAccount::address_from_bytes(mt_account_data, &mt_pubkey).unwrap();

    assert_address_merkle_tree_update(old_account, account, new_root);
}

fn assert_merkle_tree_update(
    mut old_account: BatchedMerkleTreeAccount,
    account: BatchedMerkleTreeAccount,
    old_queue_account: Option<BatchedQueueAccount>,
    queue_account: Option<BatchedQueueAccount>,
    root: [u8; 32],
) {
    let input_queue_previous_batch_state =
        old_account.queue_batches.get_previous_batch().get_state();
    let input_queue_current_batch = old_account.queue_batches.get_current_batch();
    let previous_batch_index = old_account.queue_batches.get_previous_batch_index();
    let is_half_full = input_queue_current_batch.get_num_inserted_elements()
        >= input_queue_current_batch.batch_size / 2;
    if is_half_full
        && input_queue_previous_batch_state == BatchState::Inserted
        && !old_account
            .queue_batches
            .get_previous_batch()
            .bloom_filter_is_zeroed()
    {
        old_account
            .queue_batches
            .get_previous_batch_mut()
            .set_bloom_filter_to_zeroed();
        old_account.bloom_filter_stores[previous_batch_index]
            .iter_mut()
            .for_each(|elem| {
                *elem = 0;
            });
        let previous_full_batch = old_account
            .queue_batches
            .batches
            .get(previous_batch_index)
            .unwrap();
        let sequence_number = previous_full_batch.sequence_number;
        let overlapping_roots_exits = sequence_number > old_account.sequence_number;
        if overlapping_roots_exits {
            let mut oldest_root_index = old_account.root_history.first_index();
            // 2.1. Get, num of remaining roots.
            //    Remaining roots have not been updated since
            //    the update of the previous batch hence enable to prove
            //    inclusion of values nullified in the previous batch.
            let num_remaining_roots = sequence_number - old_account.sequence_number;
            // 2.2. Zero out roots oldest to first safe root index.
            //      Skip one iteration we don't need to zero out
            //      the first safe root.
            for _ in 1..num_remaining_roots {
                old_account.root_history[oldest_root_index] = [0u8; 32];
                oldest_root_index += 1;
                oldest_root_index %= old_account.root_history.len();
            }
        }
    }
    // Output queue update
    if let Some(mut old_queue_account) = old_queue_account {
        let queue_account = queue_account.unwrap();
        let old_full_batch_index = old_queue_account.batch_metadata.pending_batch_index;
        let old_full_batch = old_queue_account
            .batch_metadata
            .batches
            .get_mut(old_full_batch_index as usize)
            .unwrap();
        old_full_batch
            .mark_as_inserted_in_merkle_tree(
                account.sequence_number,
                account.root_history.last_index() as u32,
                old_account.root_history.capacity() as u32,
            )
            .unwrap();

        if old_full_batch.get_state() == BatchState::Inserted {
            old_queue_account.batch_metadata.pending_batch_index += 1;
            old_queue_account.batch_metadata.pending_batch_index %= 2;
        }
        assert_eq!(
            queue_account.get_metadata(),
            old_queue_account.get_metadata()
        );
        assert_eq!(queue_account, old_queue_account);
        // Only the output queue appends state
        let zkp_batch_size = old_account.queue_batches.zkp_batch_size;
        old_account.next_index += zkp_batch_size;
    } else {
        // Input queue update
        let old_full_batch_index = old_account.queue_batches.pending_batch_index;
        let history_capacity = old_account.root_history.capacity();
        let previous_full_batch_index = if old_full_batch_index == 0 { 1 } else { 0 };

        let old_full_batch = old_account
            .queue_batches
            .batches
            .get_mut(old_full_batch_index as usize)
            .unwrap();

        old_full_batch
            .mark_as_inserted_in_merkle_tree(
                account.sequence_number,
                account.root_history.last_index() as u32,
                history_capacity as u32,
            )
            .unwrap();
        println!(
            "current batch {:?}",
            old_full_batch.get_num_inserted_elements()
        );

        if old_full_batch.get_state() == BatchState::Inserted {
            old_account.queue_batches.pending_batch_index += 1;
            old_account.queue_batches.pending_batch_index %= 2;
        }
        let old_full_batch_index = old_account.queue_batches.pending_batch_index;

        let old_full_batch = old_account
            .queue_batches
            .batches
            .get_mut(old_full_batch_index as usize)
            .unwrap();
        let zeroed_batch =
            old_full_batch.get_num_inserted_elements() >= old_full_batch.batch_size / 2;
        println!("zeroed_batch: {:?}", zeroed_batch);

        // let current_batch = old_account.queue_batches.get_current_batch();

        let state = account.queue_batches.batches[previous_full_batch_index].get_state();
        let previous_batch = old_account
            .queue_batches
            .batches
            .get_mut(previous_full_batch_index)
            .unwrap();
        if zeroed_batch && state == BatchState::Inserted {
            previous_batch.set_bloom_filter_to_zeroed();
            let sequence_number = previous_batch.sequence_number;
            let overlapping_roots_exits = sequence_number > old_account.sequence_number;
            if overlapping_roots_exits {
                old_account.bloom_filter_stores[previous_full_batch_index]
                    .iter_mut()
                    .for_each(|elem| {
                        *elem = 0;
                    });

                let mut oldest_root_index = old_account.root_history.first_index();

                let num_remaining_roots = sequence_number - old_account.sequence_number;
                for _ in 1..num_remaining_roots {
                    println!("zeroing out root index: {}", oldest_root_index);
                    old_account.root_history[oldest_root_index] = [0u8; 32];
                    oldest_root_index += 1;
                    oldest_root_index %= old_account.root_history.len();
                }
            }
        }
    }

    old_account.sequence_number += 1;
    old_account.root_history.push(root);
    assert_eq!(account.get_metadata(), old_account.get_metadata());
    assert_eq!(account, old_account);
    assert_eq!(*account.root_history.last().unwrap(), root);
}

fn assert_address_merkle_tree_update(
    mut old_account: BatchedMerkleTreeAccount,
    account: BatchedMerkleTreeAccount,
    root: [u8; 32],
) {
    {
        // Input queue update
        let old_full_batch_index = old_account.queue_batches.pending_batch_index;
        let history_capacity = old_account.root_history.capacity();

        let old_full_batch = old_account
            .queue_batches
            .batches
            .get_mut(old_full_batch_index as usize)
            .unwrap();

        old_full_batch
            .mark_as_inserted_in_merkle_tree(
                account.sequence_number,
                account.root_history.last_index() as u32,
                history_capacity as u32,
            )
            .unwrap();
        if old_full_batch.get_state() == BatchState::Inserted {
            old_account.queue_batches.pending_batch_index += 1;
            old_account.queue_batches.pending_batch_index %= 2;
        }
        let old_full_batch_index = old_account.queue_batches.pending_batch_index;

        let previous_full_batch_index = if old_full_batch_index == 0 { 1 } else { 0 };

        let old_full_batch_index = old_account.queue_batches.pending_batch_index;

        let old_full_batch = old_account
            .queue_batches
            .batches
            .get_mut(old_full_batch_index as usize)
            .unwrap();
        let zeroed_batch =
            old_full_batch.get_num_inserted_elements() >= old_full_batch.batch_size / 2;
        println!("zeroed_batch: {:?}", zeroed_batch);
        let state = account.queue_batches.batches[previous_full_batch_index].get_state();
        let previous_batch = old_account
            .queue_batches
            .batches
            .get_mut(previous_full_batch_index)
            .unwrap();
        if zeroed_batch && state == BatchState::Inserted {
            previous_batch.set_bloom_filter_to_zeroed();
            let sequence_number = previous_batch.sequence_number;
            let overlapping_roots_exits = sequence_number > old_account.sequence_number;
            if overlapping_roots_exits {
                old_account.bloom_filter_stores[previous_full_batch_index]
                    .iter_mut()
                    .for_each(|elem| {
                        *elem = 0;
                    });

                let mut oldest_root_index = old_account.root_history.first_index();

                let num_remaining_roots = sequence_number - old_account.sequence_number;
                for _ in 1..num_remaining_roots {
                    println!("zeroing out root index: {}", oldest_root_index);
                    old_account.root_history[oldest_root_index] = [0u8; 32];
                    oldest_root_index += 1;
                    oldest_root_index %= old_account.root_history.len();
                }
            }
        }
    }

    old_account.sequence_number += 1;
    old_account.next_index += old_account.queue_batches.zkp_batch_size;
    old_account.root_history.push(root);
    assert_eq!(account.get_metadata(), old_account.get_metadata());
    assert_eq!(account, old_account);
    assert_eq!(*account.root_history.last().unwrap(), root);
}

pub fn get_rnd_bytes(rng: &mut StdRng) -> [u8; 32] {
    let mut rnd_bytes = rng.gen::<[u8; 32]>();
    rnd_bytes[0] = 0;
    rnd_bytes
}

#[serial]
#[tokio::test]
async fn test_fill_state_queues_completely() {
    spawn_prover(
        true,
        ProverConfig {
            run_mode: None,
            circuits: vec![
                ProofType::BatchAppendWithProofsTest,
                ProofType::BatchUpdateTest,
            ],
        },
    )
    .await;
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
            perform_input_update(&mut mt_account_data, &mut mock_indexer, false, mt_pubkey).await;
            // after 5 updates the first batch is completely inserted
            // As soon as we switch to inserting the second batch we zero out the first batch since
            // the second batch is completely full.
            if i >= 4 {
                let merkle_tree_account = &mut BatchedMerkleTreeAccount::state_from_bytes(
                    &mut mt_account_data,
                    &mt_pubkey,
                )
                .unwrap();
                let batch = merkle_tree_account.queue_batches.batches.first().unwrap();
                assert!(batch.bloom_filter_is_zeroed());
            } else {
                let merkle_tree_account = &mut BatchedMerkleTreeAccount::state_from_bytes(
                    &mut mt_account_data,
                    &mt_pubkey,
                )
                .unwrap();
                let batch = merkle_tree_account.queue_batches.batches.first().unwrap();
                assert!(!batch.bloom_filter_is_zeroed());
            }
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

#[serial]
#[tokio::test]
async fn test_fill_address_tree_completely() {
    spawn_prover(
        true,
        ProverConfig {
            run_mode: None,
            circuits: vec![ProofType::BatchAddressAppendTest],
        },
    )
    .await;
    let mut current_slot = 1;
    let roothistory_capacity = vec![17, 80]; //
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
        for i in 0..num_updates {
            println!("address update ----------------------------- {}", i);
            perform_address_update(&mut mt_account_data, &mut mock_indexer, mt_pubkey).await;
            if i == 4 {
                first_input_batch_update_root_value = mock_indexer.merkle_tree.root();
            }
            let merkle_tree_account =
                BatchedMerkleTreeAccount::address_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();
            let batch = merkle_tree_account.queue_batches.batches.first().unwrap();
            let batch_one = merkle_tree_account.queue_batches.batches.get(1).unwrap();
            assert!(!batch_one.bloom_filter_is_zeroed());

            // after 5 updates the first batch is completely inserted
            // As soon as we switch to inserting the second batch we zero out the first batch since
            // the second batch is completely full.
            if i >= 4 {
                assert!(batch.bloom_filter_is_zeroed());
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
                    assert!(batch.bloom_filter_is_zeroed());
                } else {
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

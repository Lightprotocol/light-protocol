#![allow(unused_assignments)]
use std::cmp::min;

use crate::e2e_tests::shared::*;
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
        init_batched_state_merkle_tree_accounts,
        test_utils::get_state_merkle_tree_account_size_from_params,
        InitStateTreeAccountsInstructionData,
    },
    merkle_tree::{
        assert_batch_adress_event, assert_batch_append_event_event, assert_nullify_event,
        test_utils::get_merkle_tree_account_size_default, BatchedMerkleTreeAccount,
        InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs,
    },
    merkle_tree_metadata::BatchedMerkleTreeMetadata,
    queue::{
        test_utils::{
            get_output_queue_account_size_default, get_output_queue_account_size_from_params,
        },
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
use light_prover_client::prover::spawn_prover;
use light_test_utils::mock_batched_forester::{
    MockBatchedAddressForester, MockBatchedForester, MockTxEvent,
};
use light_zero_copy::vec::ZeroCopyVecU64;
use rand::{rngs::StdRng, Rng};
use serial_test::serial;

/// queues with a counter which keeps things below X tps and an if that
/// executes tree updates when possible.
#[serial]
#[tokio::test]
async fn test_e2e() {
    spawn_prover().await;
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

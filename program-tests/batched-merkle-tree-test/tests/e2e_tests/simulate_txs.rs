#![allow(unused_assignments)]
use std::cmp::min;

use light_array_map::ArrayMap;
use light_batched_merkle_tree::{
    constants::{ACCOUNT_COMPRESSION_PROGRAM_ID, DEFAULT_BATCH_STATE_TREE_HEIGHT},
    errors::BatchedMerkleTreeError,
    initialize_state_tree::{
        init_batched_state_merkle_tree_accounts, InitStateTreeAccountsInstructionData,
    },
    merkle_tree::{
        assert_batch_append_event_event, assert_nullify_event,
        test_utils::get_merkle_tree_account_size_default, BatchedMerkleTreeAccount,
        InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs,
    },
    queue::{test_utils::get_output_queue_account_size_default, BatchedQueueAccount},
};
use light_compressed_account::{
    hash_chain::create_hash_chain_from_slice, instruction_data::compressed_proof::CompressedProof,
    pubkey::Pubkey,
};
use light_hasher::Poseidon;
use light_merkle_tree_reference::MerkleTree;
use light_prover_client::prover::spawn_prover;
use light_test_utils::mock_batched_forester::{MockBatchedForester, MockTxEvent};
use rand::{rngs::StdRng, Rng};
use serial_test::serial;

use crate::e2e_tests::shared::*;

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

    // Track roots created during each batch insertion (batch_index -> roots)
    let mut batch_roots: ArrayMap<u32, Vec<[u8; 32]>, 2> = ArrayMap::new();

    // Track the initial root for batch 0
    // For StateV2 trees, this is the zero bytes root for the tree height
    {
        let initial_root =
            light_hasher::Poseidon::zero_bytes()[DEFAULT_BATCH_STATE_TREE_HEIGHT as usize];
        use light_hasher::Hasher;
        batch_roots.insert(0, vec![initial_root], ()).unwrap();
        println!("Initial root {:?} tracked for batch 0", initial_root);
    }

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
            assert_merkle_tree_update(old_account, account, None, None, new_root, &mut batch_roots);
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
                &mut batch_roots,
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

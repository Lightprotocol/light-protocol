#![allow(unused_assignments)]

use light_array_map::ArrayMap;
use light_batched_merkle_tree::{
    batch::BatchState,
    constants::DEFAULT_BATCH_STATE_TREE_HEIGHT,
    errors::BatchedMerkleTreeError,
    merkle_tree::{
        assert_batch_adress_event, BatchedMerkleTreeAccount, InstructionDataBatchNullifyInputs,
    },
    merkle_tree_metadata::BatchedMerkleTreeMetadata,
    queue::{BatchedQueueAccount, BatchedQueueMetadata},
};
use light_bloom_filter::BloomFilter;
use light_compressed_account::{
    instruction_data::compressed_proof::CompressedProof, pubkey::Pubkey,
};
use light_hasher::{Hasher, Poseidon};
use light_test_utils::mock_batched_forester::{MockBatchedAddressForester, MockBatchedForester};
use light_zero_copy::vec::ZeroCopyVecU64;
use rand::{rngs::StdRng, Rng};

pub async fn perform_address_update(
    mt_account_data: &mut [u8],
    mock_indexer: &mut MockBatchedAddressForester<40>,
    mt_pubkey: Pubkey,
    batch_roots: &mut Vec<(u32, Vec<[u8; 32]>)>,
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
                next_index as usize,
                batch_start_index as usize,
                *current_root,
            )
            .await
            .unwrap();

        mock_indexer.finalize_batch_address_update(10);
        assert_eq!(mock_indexer.merkle_tree.root(), new_root);
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

    let batch_index_for_this_root = _pre_next_full_batch as u32;
    if let Some((_idx, roots)) = batch_roots
        .iter_mut()
        .find(|(idx, _)| *idx == batch_index_for_this_root)
    {
        roots.push(new_root);
    } else {
        batch_roots.push((batch_index_for_this_root, vec![new_root]));
    }

    assert_address_merkle_tree_update(old_account, account, new_root, batch_roots);
}

pub fn assert_merkle_tree_update(
    mut old_account: BatchedMerkleTreeAccount,
    account: BatchedMerkleTreeAccount,
    old_queue_account: Option<BatchedQueueAccount>,
    queue_account: Option<BatchedQueueAccount>,
    root: [u8; 32],
    batch_roots: &mut ArrayMap<u32, Vec<[u8; 32]>, 2>,
) {
    old_account.sequence_number += 1;
    old_account.root_history.push(root);
    println!("Adding root: {:?}", root);
    // Determine batch index and state for this update
    // For both input and output updates, use the INPUT queue's batch index
    // because that's what controls root zeroing
    let (batch_idx, _) = {
        let idx = old_account.queue_batches.pending_batch_index;
        let state = old_account
            .queue_batches
            .batches
            .get(idx as usize)
            .unwrap()
            .get_state();
        (idx as u32, state)
    };
    if let Some(roots) = batch_roots.get_mut_by_key(&batch_idx) {
        roots.push(root)
    } else {
        batch_roots.insert(batch_idx, vec![root], ()).unwrap();
    }

    let input_queue_previous_batch_state =
        old_account.queue_batches.get_previous_batch().get_state();
    let input_queue_current_batch = old_account.queue_batches.get_current_batch();
    let previous_batch_index = old_account.queue_batches.get_previous_batch_index();
    let is_half_full = input_queue_current_batch.get_num_inserted_elements()
        >= input_queue_current_batch.batch_size / 2
        && input_queue_current_batch.get_state() != BatchState::Inserted;
    let root_history_len = old_account.root_history.capacity() as u64;
    let previous_batch = old_account.queue_batches.get_previous_batch();
    let no_insert_since_last_batch_root = (previous_batch
        .sequence_number
        .saturating_sub(root_history_len))
        == old_account.sequence_number;
    if is_half_full
        && input_queue_previous_batch_state == BatchState::Inserted
        && !old_account
            .queue_batches
            .get_previous_batch()
            .bloom_filter_is_zeroed()
        && !no_insert_since_last_batch_root
    {
        println!("Entering zeroing block for batch {}", previous_batch_index);
        println!(
            "Previous batch state: {:?}",
            input_queue_previous_batch_state
        );
        println!(
            "Previous batch: {:?}",
            old_account.queue_batches.get_previous_batch()
        );
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

        // Log the last unsafe root
        let last_unsafe_root_index = previous_full_batch.root_index;
        let first_safe_root_index = last_unsafe_root_index + 1;
        println!("DEBUG: Last unsafe root index: {}", last_unsafe_root_index);
        println!("DEBUG: First safe root index: {}", first_safe_root_index);
        if let Some(last_unsafe_root) = old_account
            .root_history
            .get(last_unsafe_root_index as usize)
        {
            println!(
                "DEBUG: Last unsafe root at index {}: {:?}",
                last_unsafe_root_index,
                &last_unsafe_root[0..4]
            );
        }

        let overlapping_roots_exits = sequence_number > old_account.sequence_number;
        if overlapping_roots_exits {
            let mut oldest_root_index = old_account.root_history.first_index();
            // 2.1. Get, num of remaining roots.
            //    Remaining roots have not been updated since
            //    the update of the previous batch hence enable to prove
            //    inclusion of values nullified in the previous batch.
            let num_remaining_roots = sequence_number - old_account.sequence_number;
            // 2.2. Zero out roots oldest to first safe root index.
            for _ in 0..num_remaining_roots {
                old_account.root_history[oldest_root_index] = [0u8; 32];
                oldest_root_index += 1;
                oldest_root_index %= old_account.root_history.len();
            }

            // Assert that all unsafe roots from this batch are zeroed
            let batch_key = previous_batch_index as u32;
            if let Some(unsafe_roots) = batch_roots.get_by_key(&batch_key) {
                for unsafe_root in unsafe_roots {
                    assert!(
                        !old_account
                            .root_history
                            .iter()
                            .any(|x| *x == *unsafe_root),
                        "Unsafe root from batch {} should be zeroed: {:?} root history {:?}, unsafe roots {:?}",
                        previous_batch_index,
                        unsafe_root,
                        old_account.root_history, unsafe_roots
                    );
                }
                // Clear unsafe roots after verification - batch index will be reused
                if let Some(roots) = batch_roots.get_mut_by_key(&batch_key) {
                    roots.clear();
                }
            }

            // Assert that the correct number of roots remain non-zero
            // Calculate expected non-zero roots: those created since the last zeroing
            let non_zero_roots: Vec<[u8; 32]> = old_account
                .root_history
                .iter()
                .filter(|root| **root != [0u8; 32])
                .copied()
                .collect();

            // Expected number of non-zero roots = number of updates since last zeroing
            // This is the sequence difference that wasn't zeroed
            let expected_non_zero = old_account.root_history.len() - num_remaining_roots as usize;

            assert_eq!(
                non_zero_roots.len(),
                expected_non_zero,
                "Expected {} non-zero roots after zeroing, but found {}. Root history: {:?}",
                expected_non_zero,
                non_zero_roots.len(),
                old_account.root_history
            );

            // Assert that all remaining non-zero roots are tracked in the current (non-zeroed) batch
            let current_batch_idx = old_account.queue_batches.pending_batch_index as u32;
            if let Some(current_batch_roots) = batch_roots.get_by_key(&current_batch_idx) {
                // Debug: print the entire root history
                println!("DEBUG: Root history after zeroing:");
                for (i, root) in old_account.root_history.iter().enumerate() {
                    if *root != [0u8; 32] {
                        println!("  Index {}: {:?}", i, root);
                    }
                }

                // Debug: print all tracked roots for current batch and their indices
                println!("DEBUG: Roots tracked for batch {}:", current_batch_idx);
                for (i, root) in current_batch_roots.iter().enumerate() {
                    let root_index = old_account.root_history.iter().position(|r| r == root);
                    println!("  Root {}: {:?} at index {:?}", i, root, root_index);
                }
                let next_batch_index = (current_batch_idx + 1) % 2;
                println!("DEBUG: Roots tracked for next batch {}:", next_batch_index);
                for (i, root) in batch_roots
                    .get_by_key(&next_batch_index)
                    .as_ref()
                    .unwrap()
                    .iter()
                    .enumerate()
                {
                    let root_index = old_account.root_history.iter().position(|r| r == root);
                    println!("  Root {}: {:?} at index {:?}", i, root, root_index);
                }

                for non_zero_root in &non_zero_roots {
                    // Skip the initial root (usually all zeros or a known starting value)
                    // which might not be tracked in any batch
                    if old_account.sequence_number > 0 {
                        assert!(
                            current_batch_roots.contains(non_zero_root),
                            "Non-zero root {:?} should be tracked in current batch {} but wasn't found. Current batch roots: {:?}",
                            non_zero_root,
                            current_batch_idx,
                            current_batch_roots
                        );
                    }
                }

                // Also verify the count matches
                println!("DEBUG: current_batch_idx: {}", current_batch_idx);
                println!(
                    "DEBUG: current_batch_roots.len(): {}",
                    current_batch_roots.len()
                );
                println!("DEBUG: non_zero_roots.len(): {}", non_zero_roots.len());
                println!(
                    "DEBUG: merkle_tree.sequence_number: {}",
                    old_account.sequence_number
                );
                println!("DEBUG: num_remaining_roots: {}", num_remaining_roots);
                println!("DEBUG: previous_batch.sequence_number: {}", sequence_number);
                assert_eq!(
                    current_batch_roots.len(),
                    non_zero_roots.len(),
                    "Current batch {} should have {} roots tracked, but has {}",
                    current_batch_idx,
                    non_zero_roots.len(),
                    current_batch_roots.len()
                );
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
        let zkp_batch_size = old_account.queue_batches.zkp_batch_size;
        old_account.nullifier_next_index += zkp_batch_size;

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
        let zeroed_batch = old_full_batch.get_num_inserted_elements()
            >= old_full_batch.batch_size / 2
            && old_full_batch.get_state() != BatchState::Inserted;
        println!("zeroed_batch: {:?}", zeroed_batch);

        let state = account.queue_batches.batches[previous_full_batch_index].get_state();
        let root_history_len = old_account.root_history.capacity() as u64;
        let old_account_sequence_number = old_account.sequence_number;
        let previous_batch_sequence_number = old_account
            .queue_batches
            .batches
            .get(previous_full_batch_index)
            .unwrap()
            .sequence_number;
        let no_insert_since_last_batch_root = (previous_batch_sequence_number
            .saturating_sub(root_history_len))
            == old_account_sequence_number;
        println!(
            "zeroing out values: {}",
            zeroed_batch && state == BatchState::Inserted
        );
        if zeroed_batch && state == BatchState::Inserted && !no_insert_since_last_batch_root {
            println!(
                "DEBUG: Entering OUTPUT queue zeroing block for batch {}",
                previous_full_batch_index
            );
            let previous_batch = old_account
                .queue_batches
                .batches
                .get_mut(previous_full_batch_index)
                .unwrap();
            previous_batch.set_bloom_filter_to_zeroed();
            let sequence_number = previous_batch_sequence_number;
            let overlapping_roots_exits = sequence_number > old_account_sequence_number;
            if overlapping_roots_exits {
                old_account.bloom_filter_stores[previous_full_batch_index]
                    .iter_mut()
                    .for_each(|elem| {
                        *elem = 0;
                    });

                let mut oldest_root_index = old_account.root_history.first_index();

                let num_remaining_roots = sequence_number - old_account_sequence_number;
                println!("num_remaining_roots: {}", num_remaining_roots);
                println!("sequence_number: {}", account.sequence_number);
                for _ in 0..num_remaining_roots {
                    println!("zeroing out root index: {}", oldest_root_index);
                    old_account.root_history[oldest_root_index] = [0u8; 32];
                    oldest_root_index += 1;
                    oldest_root_index %= old_account.root_history.len();
                }

                // Assert that all unsafe roots from this batch are zeroed
                let batch_key = previous_full_batch_index as u32;
                if let Some(unsafe_roots) = batch_roots.get_by_key(&batch_key) {
                    for unsafe_root in unsafe_roots {
                        assert!(
                            !old_account.root_history.iter().any(|x| *x == *unsafe_root),
                            "Unsafe root from batch {} should be zeroed: {:?}",
                            previous_full_batch_index,
                            unsafe_root
                        );
                    }
                    // Clear unsafe roots after verification - batch index will be reused
                    if let Some(roots) = batch_roots.get_mut_by_key(&batch_key) {
                        roots.clear();
                    }
                }

                // Assert that the correct number of roots remain non-zero
                let non_zero_roots: Vec<[u8; 32]> = old_account
                    .root_history
                    .iter()
                    .filter(|root| **root != [0u8; 32])
                    .copied()
                    .collect();

                // Expected number of non-zero roots = number of updates since last zeroing
                let expected_non_zero =
                    old_account.root_history.len() - num_remaining_roots as usize;
                println!("num_remaining_roots {}", num_remaining_roots);
                assert_eq!(
                    non_zero_roots.len(),
                    expected_non_zero,
                    "Expected {} non-zero roots after output queue zeroing, but found {}. Root history: {:?}",
                    expected_non_zero,
                    non_zero_roots.len(),
                    old_account.root_history
                );

                // Assert that all remaining non-zero roots are tracked in the current (non-zeroed) batch
                let current_batch_idx = old_account.queue_batches.pending_batch_index as u32;
                if let Some(current_batch_roots) = batch_roots.get_by_key(&current_batch_idx) {
                    for non_zero_root in &non_zero_roots {
                        // Skip the initial root which might not be tracked in any batch
                        if old_account.sequence_number > 0 {
                            assert!(
                                current_batch_roots.contains(non_zero_root),
                                "Non-zero root {:?} should be tracked in current batch {} but wasn't found. Current batch roots: {:?}",
                                non_zero_root,
                                current_batch_idx,
                                current_batch_roots
                            );
                        }
                    }

                    // Also verify the count matches
                    assert_eq!(
                        current_batch_roots.len(),
                        non_zero_roots.len(),
                        "Current batch {} should have {} roots tracked, but has {}",
                        current_batch_idx,
                        non_zero_roots.len(),
                        current_batch_roots.len()
                    );
                }
            }
        }
    }

    assert_eq!(account.get_metadata(), old_account.get_metadata());
    assert_eq!(account, old_account);
    assert_eq!(*account.root_history.last().unwrap(), root);
}

pub fn assert_address_merkle_tree_update(
    mut old_account: BatchedMerkleTreeAccount,
    account: BatchedMerkleTreeAccount,
    root: [u8; 32],
    batch_roots: &[(u32, Vec<[u8; 32]>)],
) {
    {
        // Input queue update
        let old_full_batch_index = old_account.queue_batches.pending_batch_index;
        let history_capacity = old_account.root_history.capacity();
        let pre_roots = old_account.root_history.to_vec();
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
        let current_seq = account.sequence_number;
        let root_history_len = account.root_history_capacity as u64;
        let state_seq = account.queue_batches.batches[previous_full_batch_index].sequence_number;
        let no_insert_since_last_batch_root =
            state_seq.saturating_sub(root_history_len) == current_seq;
        println!(
            "previous_batch_is_inserted{}",
            old_full_batch.get_state() != BatchState::Inserted
        );
        println!(
            "no_insert_since_last_batch_root {}",
            no_insert_since_last_batch_root
        );
        let zeroed_batch = old_full_batch.get_num_inserted_elements()
            >= old_full_batch.batch_size / 2
            && old_full_batch.get_state() != BatchState::Inserted
            && !no_insert_since_last_batch_root;
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
                for _ in 0..num_remaining_roots {
                    println!("zeroing out root index: {}", oldest_root_index);
                    old_account.root_history[oldest_root_index] = [0u8; 32];
                    oldest_root_index += 1;
                    oldest_root_index %= old_account.root_history.len();
                }
                println!(
                    "pre roots {:?}",
                    pre_roots
                        .iter()
                        .filter(|r| **r != [0u8; 32])
                        .cloned()
                        .collect::<Vec<[u8; 32]>>()
                );

                println!(
                    "post roots (actual account) {:?}",
                    account
                        .root_history
                        .iter()
                        .filter(|r| **r != [0u8; 32])
                        .cloned()
                        .collect::<Vec<[u8; 32]>>()
                );
                // No roots of the zeroed batch exist in the root history
                if let Some((_idx, zeroed_batch_roots)) = batch_roots
                    .iter()
                    .find(|(idx, _)| *idx == previous_full_batch_index as u32)
                {
                    for root in zeroed_batch_roots {
                        println!("checking root {:?}", root);
                        assert!(
                            !account.root_history.iter().any(|r| r == root),
                            "Zeroed batch root {:?} still exists in root_history",
                            root
                        );
                    }
                }
                // All non-zero roots in the root history belong to the current batch
                let current_batch_index = old_full_batch_index as u32;
                if let Some((_idx, current_batch_roots)) = batch_roots
                    .iter()
                    .find(|(idx, _)| *idx == current_batch_index)
                {
                    for root in account.root_history.iter() {
                        if *root != [0u8; 32] {
                            assert!(
                                current_batch_roots.contains(root),
                                "Non-zero root {:?} in root_history does not belong to current batch {}",
                                root,
                                current_batch_index
                            );
                        }
                    }
                }
            }
        }
    }

    old_account.sequence_number += 1;
    old_account.next_index += old_account.queue_batches.zkp_batch_size;
    old_account.root_history.push(root);
    println!(
        "post roots (old_account simulation) {:?}",
        old_account
            .root_history
            .iter()
            .filter(|r| **r != [0u8; 32])
            .cloned()
            .collect::<Vec<[u8; 32]>>()
    );
    assert_eq!(account.get_metadata(), old_account.get_metadata());
    assert_eq!(*account.root_history.last().unwrap(), root);
    assert_eq!(account, old_account);
}

pub fn get_rnd_bytes(rng: &mut StdRng) -> [u8; 32] {
    let mut rnd_bytes = rng.gen::<[u8; 32]>();
    rnd_bytes[0] = 0;
    rnd_bytes
}

pub async fn perform_input_update(
    mt_account_data: &mut [u8],
    mock_indexer: &mut MockBatchedForester<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>,
    enable_assert: bool,
    mt_pubkey: Pubkey,
    batch_roots: &mut ArrayMap<u32, Vec<[u8; 32]>, 2>,
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
        assert_merkle_tree_update(old_account, account, None, None, root, batch_roots);
    }
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

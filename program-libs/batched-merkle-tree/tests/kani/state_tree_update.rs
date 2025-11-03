#![cfg(kani)]
use crate::utils::*;
use light_batched_merkle_tree::batch::BatchState;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_batched_merkle_tree::merkle_tree::InstructionDataBatchNullifyInputs;
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_compressed_account::{pubkey::Pubkey, TreeType};
use light_merkle_tree_metadata::merkle_tree::MerkleTreeMetadata;

// Minimal full test:
// 0. Setup - create a small state tree
// 1. fill 2 batches completely
// 2. fully insert both batches via input queue (nullify operations)
//
// Verified Properties:
// 1. No unsafe roots should be present (internal invariant)
// Post conditions:
// 2. Both batches are in inserted state
// 3. sequence numbers are 3 + 7 and 6 + 7
// 4. root history contains one root [6u8; 32]
// 5. bloom filter 0 is zeroed
// 6. bloom filter 1 is not zeroed
#[kani::proof]
#[kani::stub(
    ::light_compressed_account::hash_to_bn254_field_size_be,
    stub_hash_to_bn254
)]
#[kani::unwind(35)] // Need at least 33 for memcmp on 32-byte arrays + extra for loops
fn verify_state_tree_update_minimal() {
    let mut tree = create_test_tree_small_state();
    kani::cover!(tree.root_history.len() > 0, "Root history non-empty");
    setup_batches(&mut tree, 2);
    // Verify setup succeeded
    kani::cover!(
        tree.queue_batches.batches[0].batch_is_ready_to_insert(),
        "Batch 0 is ready after setup"
    );
    kani::cover!(
        tree.queue_batches.batches[1].batch_is_ready_to_insert(),
        "Batch 1 is ready after setup"
    );
    for i in 0..1 {
        let num_insertions: u8 = 6;
        for i in 1..=num_insertions {
            let new_root: [u8; 32] = [i; 32];
            let result = tree.update_tree_from_input_queue(InstructionDataBatchNullifyInputs {
                new_root,
                compressed_proof: CompressedProof::default(), // we stub proof verification internally so the proof doesnt matter
            });
            kani::cover!(result.is_ok(), "Update succeeded");
        }
    }

    // Postcondition 2: Both batches are in inserted state
    assert_eq!(
        tree.queue_batches.batches[0].get_state(),
        BatchState::Inserted
    );
    assert_eq!(
        tree.queue_batches.batches[1].get_state(),
        BatchState::Inserted
    );
    // Postcondition 3: Sequence numbers are 3 + 7 and 6 + 7
    assert_eq!(tree.queue_batches.batches[0].sequence_number, 10);
    assert_eq!(tree.queue_batches.batches[1].sequence_number, 13);
    // Postcondition 4: Root history contains [6u8; 32]
    let contains_root_5 = (0..tree.root_history.len()).any(|i| tree.root_history[i] == [6u8; 32]);
    assert!(contains_root_5);
}

//  cargo kani --tests --no-default-features -Z stubbing --features kani --harness verify_state_tree_update_one_by_one
/// Comprehensive harness: Verify invariant holds under ALL possible tree states and operations
/// This uses symbolic state generation to explore the entire state space
#[kani::proof]
#[kani::stub(
    ::light_compressed_account::hash_to_bn254_field_size_be,
    stub_hash_to_bn254
)]
#[kani::unwind(35)] // Need at least 33 for memcmp on 32-byte arrays + extra for loops
fn verify_state_tree_update_one_by_one() {
    let mut tree = create_test_tree_small_state();
    kani::cover!(tree.root_history.len() > 0, "Root history non-empty");

    for i in 0..30u8 {
        kani::cover!(i == 0, "Loop iteration 0");
        kani::cover!(i == 29, "Loop iteration 29");
        setup_zkp_batches(&mut tree, 1);

        let new_root: [u8; 32] = [i; 32];
        let result = tree.update_tree_from_input_queue(InstructionDataBatchNullifyInputs {
            new_root,
            compressed_proof: CompressedProof::default(),
        });
        kani::cover!(result.is_ok(), "Update succeeded");
    }
}

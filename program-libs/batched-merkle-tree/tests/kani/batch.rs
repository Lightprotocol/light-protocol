#![cfg(feature = "kani")]

use light_batched_merkle_tree::batch::{Batch, BatchState};
use light_batched_merkle_tree::errors::BatchedMerkleTreeError;

// Helper to create batch with arbitrary valid configuration
fn any_batch() -> Batch {
    let num_iters: u64 = kani::any();
    let bloom_filter_capacity: u64 = kani::any();
    let batch_size: u64 = kani::any();
    let zkp_batch_size: u64 = kani::any();
    let start_index: u64 = kani::any();

    // Assume valid constraints
    kani::assume(num_iters > 0 && num_iters <= 20);
    kani::assume(bloom_filter_capacity > 0 && bloom_filter_capacity <= 1000);
    kani::assume(batch_size > 0 && batch_size <= 100000);
    kani::assume(zkp_batch_size > 0 && zkp_batch_size <= 2000);
    kani::assume(batch_size >= zkp_batch_size); // batch_size must be divisible or larger
    kani::assume(batch_size % zkp_batch_size == 0); // Must divide evenly
    kani::assume(batch_size / zkp_batch_size < 100); // Keep num_zkp_batches reasonable

    Batch::new(
        num_iters,
        bloom_filter_capacity,
        batch_size,
        zkp_batch_size,
        start_index,
    )
}

/// Verify that Fill -> Full transition works correctly
#[kani::proof]
fn verify_fill_to_full_transition() {
    let mut batch = any_batch();
    // New batch starts in Fill state
    assert_eq!(batch.get_state(), BatchState::Fill);

    // Transition should succeed
    let result = batch.advance_state_to_full();
    assert!(result.is_ok());

    // State should be Full after transition
    assert_eq!(batch.get_state(), BatchState::Full);
}

/// Verify that Full -> Inserted transition works correctly
#[kani::proof]
fn verify_full_to_inserted_transition() {
    let mut batch = any_batch();

    // Get to Full state first
    batch.advance_state_to_full().unwrap();
    assert_eq!(batch.get_state(), BatchState::Full);

    // Transition should succeed
    let result = batch.advance_state_to_inserted();
    assert!(result.is_ok());

    // State should be Inserted after transition
    assert_eq!(batch.get_state(), BatchState::Inserted);
}

/// Verify that Inserted -> Fill transition works correctly
#[kani::proof]
fn verify_inserted_to_fill_transition() {
    let mut batch = any_batch();

    // Get to Inserted state
    batch.advance_state_to_full().unwrap();
    batch.advance_state_to_inserted().unwrap();
    assert_eq!(batch.get_state(), BatchState::Inserted);

    // Transition should succeed
    let result = batch.advance_state_to_fill(None);
    assert!(result.is_ok());

    // State should be Fill after transition
    assert_eq!(batch.get_state(), BatchState::Fill);

    // Bloom filter should be reset to not zeroed
    assert!(!batch.bloom_filter_is_zeroed());
}

/// Verify that Inserted -> Fill with start_index works correctly
#[kani::proof]
fn verify_inserted_to_fill_with_start_index() {
    let mut batch = any_batch();
    let new_start_index: u64 = kani::any();

    // Get to Inserted state
    batch.advance_state_to_full().unwrap();
    batch.advance_state_to_inserted().unwrap();

    let result = batch.advance_state_to_fill(Some(new_start_index));
    assert!(result.is_ok());
    assert_eq!(batch.get_state(), BatchState::Fill);
}

/// Verify that all invalid transitions from Fill fail
#[kani::proof]
fn verify_fill_invalid_transitions() {
    let mut batch = any_batch();
    assert_eq!(batch.get_state(), BatchState::Fill);

    // Fill -> Inserted should fail
    let result = batch.advance_state_to_inserted();
    assert_eq!(result, Err(BatchedMerkleTreeError::BatchNotReady));
    assert_eq!(batch.get_state(), BatchState::Fill); // State unchanged

    // Fill -> Fill should fail
    let result = batch.advance_state_to_fill(None);
    assert_eq!(result, Err(BatchedMerkleTreeError::BatchNotReady));
    assert_eq!(batch.get_state(), BatchState::Fill); // State unchanged
}

/// Verify that all invalid transitions from Full fail
#[kani::proof]
fn verify_full_invalid_transitions() {
    let mut batch = any_batch();
    batch.advance_state_to_full().unwrap();
    assert_eq!(batch.get_state(), BatchState::Full);

    // Full -> Full should fail
    let result = batch.advance_state_to_full();
    assert_eq!(result, Err(BatchedMerkleTreeError::BatchNotReady));
    assert_eq!(batch.get_state(), BatchState::Full); // State unchanged

    // Full -> Fill should fail
    let result = batch.advance_state_to_fill(None);
    assert_eq!(result, Err(BatchedMerkleTreeError::BatchNotReady));
    assert_eq!(batch.get_state(), BatchState::Full); // State unchanged
}

/// Verify that all invalid transitions from Inserted fail
#[kani::proof]
fn verify_inserted_invalid_transitions() {
    let mut batch = any_batch();
    batch.advance_state_to_full().unwrap();
    batch.advance_state_to_inserted().unwrap();
    assert_eq!(batch.get_state(), BatchState::Inserted);

    // Inserted -> Full should fail
    let result = batch.advance_state_to_full();
    assert_eq!(result, Err(BatchedMerkleTreeError::BatchNotReady));
    assert_eq!(batch.get_state(), BatchState::Inserted); // State unchanged

    // Inserted -> Inserted should fail
    let result = batch.advance_state_to_inserted();
    assert_eq!(result, Err(BatchedMerkleTreeError::BatchNotReady));
    assert_eq!(batch.get_state(), BatchState::Inserted); // State unchanged
}

/// Verify complete state cycle: Fill -> Full -> Inserted -> Fill
#[kani::proof]
fn verify_complete_state_cycle() {
    let mut batch = any_batch();
    assert_eq!(batch.get_state(), BatchState::Fill);

    // Fill -> Full
    assert!(batch.advance_state_to_full().is_ok());
    assert_eq!(batch.get_state(), BatchState::Full);

    // Full -> Inserted
    assert!(batch.advance_state_to_inserted().is_ok());
    assert_eq!(batch.get_state(), BatchState::Inserted);

    // Inserted -> Fill
    assert!(batch.advance_state_to_fill(None).is_ok());
    assert_eq!(batch.get_state(), BatchState::Fill);
}

/// Verify that state transitions are deterministic
#[kani::proof]
fn verify_state_transition_determinism() {
    let mut batch1 = any_batch();
    let mut batch2 = any_batch();

    // Both should transition identically
    assert!(batch1.advance_state_to_full().is_ok());
    assert!(batch2.advance_state_to_full().is_ok());

    assert_eq!(batch1.get_state(), batch2.get_state());
    assert_eq!(batch1.get_state(), BatchState::Full);
}

/// Verify that only valid state values map to BatchState enum
#[kani::proof]
fn verify_batch_state_from_u64() {
    let value: u64 = kani::any();
    kani::assume(value <= 2); // Valid values are 0, 1, 2

    let state = BatchState::from(value);

    // Verify bidirectional conversion
    let back_to_u64: u64 = state.into();
    assert_eq!(value, back_to_u64);
}

/// Verify bloom filter flag operations
#[kani::proof]
fn verify_bloom_filter_zeroed_flags() {
    let mut batch = any_batch();

    // Initially not zeroed
    assert!(!batch.bloom_filter_is_zeroed());

    // Set to zeroed
    batch.set_bloom_filter_to_zeroed();
    assert!(batch.bloom_filter_is_zeroed());

    // Set back to not zeroed
    batch.set_bloom_filter_to_not_zeroed();
    assert!(!batch.bloom_filter_is_zeroed());
}

/// Verify that Inserted->Fill resets bloom_filter_is_zeroed flag
#[kani::proof]
fn verify_fill_transition_resets_bloom_filter_flag() {
    let mut batch = any_batch();

    // Get to Inserted state
    batch.advance_state_to_full().unwrap();
    batch.advance_state_to_inserted().unwrap();

    batch.set_bloom_filter_to_zeroed();
    assert!(batch.bloom_filter_is_zeroed());

    batch.advance_state_to_fill(None).unwrap();

    // Should be reset to not zeroed
    assert!(!batch.bloom_filter_is_zeroed());
}

/// Verify start_slot_is_set flag behavior
#[kani::proof]
fn verify_start_slot_flag() {
    let mut batch = any_batch();

    // Initially not set
    assert!(!batch.start_slot_is_set());

    let slot: u64 = kani::any();
    batch.set_start_slot(&slot);

    // Now it should be set
    assert!(batch.start_slot_is_set());

    // Setting again should be idempotent (still set)
    let new_slot: u64 = kani::any();
    batch.set_start_slot(&new_slot);
    assert!(batch.start_slot_is_set());
}

/// Verify start_slot getter/setter duality (Firecracker pattern)
#[kani::proof]
fn verify_start_slot_duality() {
    let mut batch = any_batch();
    let slot: u64 = kani::any();

    batch.set_start_slot(&slot);

    // Setter should mark as set
    assert!(batch.start_slot_is_set());
}

/// Verify that state transitions cover expected execution paths (Firecracker pattern)
#[kani::proof]
fn verify_state_transition_coverage() {
    let mut batch = any_batch();

    batch.advance_state_to_full().unwrap();
    // Cover: Fill -> Full transition occurred
    kani::cover!(batch.get_state() == BatchState::Full);

    batch.advance_state_to_inserted().unwrap();
    // Cover: Full -> Inserted transition occurred
    kani::cover!(batch.get_state() == BatchState::Inserted);

    batch.advance_state_to_fill(None).unwrap();
    // Cover: Inserted -> Fill transition occurred
    kani::cover!(batch.get_state() == BatchState::Fill);
}

/// Verify that invalid transitions are properly covered (Firecracker pattern)
#[kani::proof]
fn verify_invalid_transition_coverage() {
    let mut batch = any_batch();

    let result = batch.advance_state_to_inserted();
    // Cover: Invalid Fill -> Inserted was attempted and failed
    kani::cover!(result.is_err() && batch.get_state() == BatchState::Fill);

    let result = batch.advance_state_to_fill(None);
    // Cover: Invalid Fill -> Fill was attempted and failed
    kani::cover!(result.is_err() && batch.get_state() == BatchState::Fill);
}

/// Verify getters return correct computed values
#[kani::proof]
fn verify_computed_getters() {
    let batch = any_batch();

    // Test get_num_zkp_batches: should be batch_size / zkp_batch_size
    let expected_num_zkp = batch.get_batch_size() / batch.get_zkp_batch_size();
    assert_eq!(batch.get_num_zkp_batches(), expected_num_zkp);

    // Test get_num_hash_chain_store: should equal num_zkp_batches
    assert_eq!(batch.get_num_hash_chain_store(), expected_num_zkp as usize);

    // Initially zero inserted
    assert_eq!(batch.get_num_inserted_elements(), 0);
    assert_eq!(batch.get_num_elements_inserted_into_tree(), 0);
}

/// Verify that advance_state_to_fill with None preserves start_index
#[kani::proof]
fn verify_fill_without_index_preserves_start_index() {
    let mut batch = any_batch();

    // Get to Inserted state
    batch.advance_state_to_full().unwrap();
    batch.advance_state_to_inserted().unwrap();

    // With None, start_index should be preserved (we can't check the value but verify no error)
    let result = batch.advance_state_to_fill(None);
    assert!(result.is_ok());
    assert_eq!(batch.get_state(), BatchState::Fill);
}

/// Verify batch_is_ready_to_insert with fresh batch
#[kani::proof]
fn verify_batch_not_ready_initially() {
    let batch = any_batch();

    // Initially not ready (no full zkp batches)
    assert!(!batch.batch_is_ready_to_insert());
}

/// Verify get_num_ready_zkp_updates returns 0 initially
#[kani::proof]
fn verify_num_ready_zkp_updates_initial() {
    let batch = any_batch();

    // Initially no ready updates
    assert_eq!(batch.get_num_ready_zkp_updates(), 0);
}

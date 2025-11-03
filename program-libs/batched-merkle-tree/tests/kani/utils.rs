#![cfg(kani)]
use light_batched_merkle_tree::batch::BatchState;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_batched_merkle_tree::merkle_tree::InstructionDataBatchNullifyInputs;
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_compressed_account::{pubkey::Pubkey, TreeType};
use light_merkle_tree_metadata::merkle_tree::MerkleTreeMetadata;

// Stub for hash_to_bn254_field_size_be
pub fn stub_hash_to_bn254(_input: &[u8]) -> [u8; 32] {
    [1u8; 32]
}

// Helper to create a minimal tree for ghost state testing
pub fn create_test_tree_big() -> BatchedMerkleTreeAccount<'static> {
    let batch_size: u64 = 3; //TEST_DEFAULT_BATCH_SIZE;
    let zkp_batch_size: u64 = 1; // TEST_DEFAULT_ZKP_BATCH_SIZE;
    let root_history_capacity: u32 = 30;
    let height = 40; // Address trees require height 40
    let num_iters = 1;
    let bloom_filter_capacity = 8; // Minimum 8 bits = 1 byte

    // Calculate required size (includes ghost state when kani feature is enabled)
    let size = light_batched_merkle_tree::merkle_tree::get_merkle_tree_account_size(
        batch_size,
        bloom_filter_capacity,
        zkp_batch_size,
        root_history_capacity,
        height,
    );

    // Allocate using mem::zeroed() reduces branches in Kani
    let account_data: &'static mut [u8; 8096] = Box::leak(Box::new(unsafe { std::mem::zeroed() }));
    let account_data: &'static mut [u8] = &mut account_data[..size];
    let pubkey = Pubkey::new_from_array([1u8; 32]);

    let init_result = BatchedMerkleTreeAccount::init(
        account_data,
        &pubkey,
        MerkleTreeMetadata::default(),
        root_history_capacity,
        batch_size,
        zkp_batch_size,
        height,
        num_iters,
        bloom_filter_capacity,
        TreeType::AddressV2,
    );

    kani::assume(init_result.is_ok());
    kani::cover!(init_result.is_ok(), "init_result");
    init_result.unwrap()
}

// Helper to create a minimal tree for ghost state testing
pub fn create_test_tree_small() -> BatchedMerkleTreeAccount<'static> {
    let batch_size: u64 = 3;
    let zkp_batch_size: u64 = 1;
    let root_history_capacity: u32 = 7;
    let height = 40; // Address trees require height 40
    let num_iters = 1;
    let bloom_filter_capacity = 8; // Minimum 8 bits = 1 byte

    // Calculate required size (includes ghost state when kani feature is enabled)
    let size = light_batched_merkle_tree::merkle_tree::get_merkle_tree_account_size(
        batch_size,
        bloom_filter_capacity,
        zkp_batch_size,
        root_history_capacity,
        height,
    );

    // Allocate using mem::zeroed() which Kani understands as properly zero-initialized
    let account_data: &'static mut [u8; 2048] = Box::leak(Box::new(unsafe { std::mem::zeroed() }));
    let account_data: &'static mut [u8] = &mut account_data[..size];
    let pubkey = Pubkey::new_from_array([1u8; 32]);

    let init_result = BatchedMerkleTreeAccount::init(
        account_data,
        &pubkey,
        MerkleTreeMetadata::default(),
        root_history_capacity,
        batch_size,
        zkp_batch_size,
        height,
        num_iters,
        bloom_filter_capacity,
        TreeType::AddressV2,
    );

    kani::assume(init_result.is_ok());
    kani::cover!(init_result.is_ok(), "init_result");
    init_result.unwrap()
}

// Helper to create a minimal state tree for ghost state testing
pub fn create_test_tree_small_state() -> BatchedMerkleTreeAccount<'static> {
    let batch_size: u64 = 3;
    let zkp_batch_size: u64 = 1;
    let root_history_capacity: u32 = 7;
    let height = 32; // State trees use height 32
    let num_iters = 1;
    let bloom_filter_capacity = 8; // Minimum 8 bits = 1 byte

    // Calculate required size (includes ghost state when kani feature is enabled)
    let size = light_batched_merkle_tree::merkle_tree::get_merkle_tree_account_size(
        batch_size,
        bloom_filter_capacity,
        zkp_batch_size,
        root_history_capacity,
        height,
    );

    // Allocate using mem::zeroed() which Kani understands as properly zero-initialized
    let account_data: &'static mut [u8; 2048] = Box::leak(Box::new(unsafe { std::mem::zeroed() }));
    let account_data: &'static mut [u8] = &mut account_data[..size];
    let pubkey = Pubkey::new_from_array([1u8; 32]);

    let init_result = BatchedMerkleTreeAccount::init(
        account_data,
        &pubkey,
        MerkleTreeMetadata::default(),
        root_history_capacity,
        batch_size,
        zkp_batch_size,
        height,
        num_iters,
        bloom_filter_capacity,
        TreeType::StateV2,
    );

    kani::assume(init_result.is_ok());
    kani::cover!(init_result.is_ok(), "init_result");
    init_result.unwrap()
}

// Setup function: Fill up to two batches to make them ready for ZKP processing
// This function populates the hash chain stores and batch metadata needed for tree updates
#[cfg_attr(kani, kani::requires(num_batches > 0 && num_batches <= 2))]
#[cfg_attr(kani, kani::requires(tree.queue_batches.batch_size > 0))]
#[cfg_attr(kani, kani::requires(tree.hash_chain_stores.len() == 2))]
pub fn setup_batches(tree: &mut BatchedMerkleTreeAccount, num_batches: usize) {
    let batch_size = tree.queue_batches.batch_size;
    let value: [u8; 32] = [2u8; 32];

    // Insert following currently_processing_batch_index (mirrors real queue behavior)
    for i in 0..num_batches {
        let current_idx = tree.queue_batches.currently_processing_batch_index as usize;

        for j in 0..batch_size {
            let result = tree.kani_mock_insert(current_idx, &value);
            kani::assume(result.is_ok());
        }

        // After batch becomes Full, advance to next batch (mirrors queue.rs:590)
        tree.queue_batches
            .increment_currently_processing_batch_index_if_full();
    }
}

#[cfg_attr(kani, kani::requires(num_zkp_batches > 0 && num_zkp_batches <= tree.queue_batches.get_num_zkp_batches() as usize * 2))]
#[cfg_attr(kani, kani::requires(tree.queue_batches.batch_size > 0))]
#[cfg_attr(kani, kani::requires(tree.hash_chain_stores.len() == 2))]
pub fn setup_zkp_batches(tree: &mut BatchedMerkleTreeAccount, num_zkp_batches: usize) {
    let batch_size = tree.queue_batches.batch_size;
    let value: [u8; 32] = [2u8; 32];

    // Insert following currently_processing_batch_index (mirrors real queue behavior)
    for i in 0..num_zkp_batches {
        let current_idx = tree.queue_batches.currently_processing_batch_index as usize;

        kani::cover!(i == 0, "Entered setup batch loop");
        let result = tree.kani_mock_insert(current_idx, &value);
        kani::assume(result.is_ok());
        // After batch becomes Full, advance to next batch (mirrors queue.rs:590)
        // TODO: add increment_currently_processing_batch_index_if_full internally to kani_mock_insert
        tree.queue_batches
            .increment_currently_processing_batch_index_if_full();
    }
}

/// Calculate total number of zkp batches ready to insert across both batches
pub fn get_total_ready_zkp_batches(tree: &BatchedMerkleTreeAccount) -> usize {
    let batch_0_ready = if tree.queue_batches.batches[0].batch_is_ready_to_insert() {
        tree.queue_batches.batches[0].get_num_ready_zkp_updates()
    } else {
        0
    };
    let batch_1_ready = if tree.queue_batches.batches[1].batch_is_ready_to_insert() {
        tree.queue_batches.batches[1].get_num_ready_zkp_updates()
    } else {
        0
    };
    (batch_0_ready + batch_1_ready) as usize
}

/// Calculate available zkp batch space across both batches
pub fn get_available_zkp_space(tree: &BatchedMerkleTreeAccount) -> usize {
    let max_zkp_batches = tree.queue_batches.get_num_zkp_batches() as usize;

    let batch_0_space = if tree.queue_batches.batches[0].get_state() == BatchState::Inserted {
        max_zkp_batches
    } else {
        let num_full = tree.queue_batches.batches[0].get_num_inserted_zkps()
            + tree.queue_batches.batches[0].get_num_ready_zkp_updates();
        (max_zkp_batches as u64 - num_full) as usize
    };

    let batch_1_space = if tree.queue_batches.batches[1].get_state() == BatchState::Inserted {
        max_zkp_batches
    } else {
        let num_full = tree.queue_batches.batches[1].get_num_inserted_zkps()
            + tree.queue_batches.batches[1].get_num_ready_zkp_updates();
        (max_zkp_batches as u64 - num_full) as usize
    };

    batch_0_space + batch_1_space
}

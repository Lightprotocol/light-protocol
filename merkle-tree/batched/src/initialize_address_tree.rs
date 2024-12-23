use light_merkle_tree_metadata::{
    access::AccessMetadata,
    merkle_tree::{MerkleTreeMetadata, TreeType},
    rollover::{check_rollover_fee_sufficient, RolloverMetadata},
};
use light_utils::fee::compute_rollover_fee;
use solana_program::pubkey::Pubkey;

use crate::{
    constants::{
        DEFAULT_BATCH_SIZE, DEFAULT_ZKP_BATCH_SIZE, TEST_DEFAULT_BATCH_SIZE,
        TEST_DEFAULT_ZKP_BATCH_SIZE,
    },
    errors::BatchedMerkleTreeError,
    initialize_state_tree::match_circuit_size,
    merkle_tree::{get_merkle_tree_account_size, ZeroCopyBatchedMerkleTreeAccount},
    BorshDeserialize, BorshSerialize,
};

#[derive(Debug, Clone, Copy, BorshDeserialize, BorshSerialize, PartialEq)]
pub struct InitAddressTreeAccountsInstructionData {
    pub index: u64,
    pub program_owner: Option<Pubkey>,
    pub forester: Option<Pubkey>,
    pub input_queue_batch_size: u64,
    pub input_queue_zkp_batch_size: u64,
    pub bloom_filter_num_iters: u64,
    pub bloom_filter_capacity: u64,
    pub root_history_capacity: u32,
    pub network_fee: Option<u64>,
    pub rollover_threshold: Option<u64>,
    pub close_threshold: Option<u64>,
    pub input_queue_num_batches: u64,
    pub height: u32,
}

impl InitAddressTreeAccountsInstructionData {
    pub fn test_default() -> Self {
        Self {
            index: 0,
            program_owner: None,
            forester: None,
            bloom_filter_num_iters: 3,
            input_queue_batch_size: TEST_DEFAULT_BATCH_SIZE,
            input_queue_zkp_batch_size: TEST_DEFAULT_ZKP_BATCH_SIZE,
            input_queue_num_batches: 2,
            height: 40,
            root_history_capacity: 20,
            bloom_filter_capacity: 20_000 * 8,
            network_fee: Some(5000),
            rollover_threshold: Some(95),
            close_threshold: None,
        }
    }

    pub fn e2e_test_default() -> Self {
        Self {
            index: 0,
            program_owner: None,
            forester: None,
            bloom_filter_num_iters: 3,
            input_queue_batch_size: 500,
            input_queue_zkp_batch_size: TEST_DEFAULT_ZKP_BATCH_SIZE,
            input_queue_num_batches: 2,
            height: 40,
            root_history_capacity: 20,
            bloom_filter_capacity: 20_000 * 8,
            network_fee: Some(5000),
            rollover_threshold: Some(95),
            close_threshold: None,
        }
    }
}

impl Default for InitAddressTreeAccountsInstructionData {
    fn default() -> Self {
        Self {
            index: 0,
            program_owner: None,
            forester: None,
            bloom_filter_num_iters: 3,
            input_queue_batch_size: DEFAULT_BATCH_SIZE,
            input_queue_zkp_batch_size: DEFAULT_ZKP_BATCH_SIZE,
            input_queue_num_batches: 2,
            height: 40,
            root_history_capacity: (DEFAULT_BATCH_SIZE / DEFAULT_ZKP_BATCH_SIZE * 2) as u32,
            bloom_filter_capacity: (DEFAULT_BATCH_SIZE + 1) * 8,
            network_fee: Some(5000),
            rollover_threshold: Some(95),
            close_threshold: None,
        }
    }
}

pub fn init_batched_address_merkle_tree_account(
    owner: Pubkey,
    params: InitAddressTreeAccountsInstructionData,
    mt_account_data: &mut [u8],
    merkle_tree_rent: u64,
) -> Result<(), BatchedMerkleTreeError> {
    let num_batches_input_queue = params.input_queue_num_batches;
    let height = params.height;

    let rollover_fee = match params.rollover_threshold {
        Some(rollover_threshold) => {
            let rent = merkle_tree_rent;
            let rollover_fee = compute_rollover_fee(rollover_threshold, height, rent)?;
            check_rollover_fee_sufficient(rollover_fee, 0, rent, rollover_threshold, height)?;
            rollover_fee
        }
        None => 0,
    };

    let metadata = MerkleTreeMetadata {
        next_merkle_tree: Pubkey::default(),
        access_metadata: AccessMetadata::new(owner, params.program_owner, params.forester),
        rollover_metadata: RolloverMetadata::new(
            params.index,
            rollover_fee,
            params.rollover_threshold,
            params.network_fee.unwrap_or_default(),
            params.close_threshold,
            None,
        ),
        associated_queue: Pubkey::default(),
    };
    ZeroCopyBatchedMerkleTreeAccount::init(
        metadata,
        params.root_history_capacity,
        num_batches_input_queue,
        params.input_queue_batch_size,
        params.input_queue_zkp_batch_size,
        height,
        mt_account_data,
        params.bloom_filter_num_iters,
        params.bloom_filter_capacity,
        TreeType::BatchedAddress,
    )?;
    Ok(())
}

pub fn validate_batched_address_tree_params(params: InitAddressTreeAccountsInstructionData) {
    assert!(params.input_queue_batch_size > 0);
    assert_eq!(
        params.input_queue_batch_size % params.input_queue_zkp_batch_size,
        0,
        "Input queue batch size must divisible by input_queue_zkp_batch_size."
    );
    assert!(
        match_circuit_size(params.input_queue_zkp_batch_size),
        "Zkp batch size not supported. Supported 1, 10, 100, 500, 1000"
    );

    assert!(params.bloom_filter_num_iters > 0);
    assert!(params.bloom_filter_capacity > params.input_queue_batch_size * 8);
    assert_eq!(
        params.bloom_filter_capacity % 8,
        0,
        "Bloom filter capacity must be divisible by 8."
    );
    assert!(params.bloom_filter_capacity > 0);
    assert!(params.root_history_capacity > 0);
    assert!(params.input_queue_batch_size > 0);
    assert_eq!(params.input_queue_num_batches, 2);
    assert_eq!(params.close_threshold, None);
    assert_eq!(params.height, 40);
}

pub fn get_address_merkle_tree_account_size_from_params(
    params: InitAddressTreeAccountsInstructionData,
) -> usize {
    get_merkle_tree_account_size(
        params.input_queue_batch_size,
        params.bloom_filter_capacity,
        params.input_queue_zkp_batch_size,
        params.root_history_capacity,
        params.height,
        params.input_queue_num_batches,
    )
}

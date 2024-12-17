use crate::{
    batch_metadata::BatchMetadata,
    constants::{
        ADDRESS_TREE_INIT_ROOT_40, DEFAULT_BATCH_SIZE, DEFAULT_CPI_CONTEXT_ACCOUNT_SIZE,
        DEFAULT_ZKP_BATCH_SIZE, TEST_DEFAULT_BATCH_SIZE, TEST_DEFAULT_ZKP_BATCH_SIZE,
    },
    errors::BatchedMerkleTreeError,
    merkle_tree::{
        get_merkle_tree_account_size, BatchedMerkleTreeAccount, ZeroCopyBatchedMerkleTreeAccount,
    },
    queue::{assert_queue_inited, BatchedQueueAccount, ZeroCopyBatchedQueueAccount},
};
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::Hasher;
use light_merkle_tree_metadata::{
    access::AccessMetadata,
    merkle_tree::{MerkleTreeMetadata, TreeType},
    queue::{QueueMetadata, QueueType},
    rollover::{check_rollover_fee_sufficient, RolloverMetadata},
};
use light_utils::fee::compute_rollover_fee;
use solana_program::{msg, pubkey::Pubkey};

#[derive(Debug, Clone, Copy, BorshDeserialize, BorshSerialize, PartialEq)]
pub struct InitStateTreeAccountsInstructionData {
    pub index: u64,
    pub program_owner: Option<Pubkey>,
    pub forester: Option<Pubkey>,
    pub additional_bytes: u64,
    pub input_queue_batch_size: u64,
    pub output_queue_batch_size: u64,
    pub input_queue_zkp_batch_size: u64,
    pub output_queue_zkp_batch_size: u64,
    pub bloom_filter_num_iters: u64,
    pub bloom_filter_capacity: u64,
    pub root_history_capacity: u32,
    pub network_fee: Option<u64>,
    pub rollover_threshold: Option<u64>,
    pub close_threshold: Option<u64>,
    pub input_queue_num_batches: u64,
    pub output_queue_num_batches: u64,
    pub height: u32,
}

impl InitStateTreeAccountsInstructionData {
    pub fn test_default() -> Self {
        Self {
            index: 0,
            program_owner: None,
            forester: None,
            additional_bytes: DEFAULT_CPI_CONTEXT_ACCOUNT_SIZE,
            bloom_filter_num_iters: 3,
            input_queue_batch_size: TEST_DEFAULT_BATCH_SIZE,
            output_queue_batch_size: TEST_DEFAULT_BATCH_SIZE,
            input_queue_zkp_batch_size: TEST_DEFAULT_ZKP_BATCH_SIZE,
            output_queue_zkp_batch_size: TEST_DEFAULT_ZKP_BATCH_SIZE,
            input_queue_num_batches: 2,
            output_queue_num_batches: 2,
            height: 26,
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
            additional_bytes: DEFAULT_CPI_CONTEXT_ACCOUNT_SIZE,
            bloom_filter_num_iters: 3,
            input_queue_batch_size: 500,
            output_queue_batch_size: 500,
            input_queue_zkp_batch_size: TEST_DEFAULT_ZKP_BATCH_SIZE,
            output_queue_zkp_batch_size: TEST_DEFAULT_ZKP_BATCH_SIZE,
            input_queue_num_batches: 2,
            output_queue_num_batches: 2,
            height: 26,
            root_history_capacity: 20,
            bloom_filter_capacity: 20_000 * 8,
            network_fee: Some(5000),
            rollover_threshold: Some(95),
            close_threshold: None,
        }
    }
}

impl Default for InitStateTreeAccountsInstructionData {
    fn default() -> Self {
        Self {
            index: 0,
            program_owner: None,
            forester: None,
            additional_bytes: DEFAULT_CPI_CONTEXT_ACCOUNT_SIZE,
            bloom_filter_num_iters: 3,
            input_queue_batch_size: DEFAULT_BATCH_SIZE,
            output_queue_batch_size: DEFAULT_BATCH_SIZE,
            input_queue_zkp_batch_size: DEFAULT_ZKP_BATCH_SIZE,
            output_queue_zkp_batch_size: DEFAULT_ZKP_BATCH_SIZE,
            input_queue_num_batches: 2,
            output_queue_num_batches: 2,
            height: 26,
            root_history_capacity: (DEFAULT_BATCH_SIZE / DEFAULT_ZKP_BATCH_SIZE * 2) as u32,
            bloom_filter_capacity: (DEFAULT_BATCH_SIZE + 1) * 8,
            network_fee: Some(5000),
            rollover_threshold: Some(95),
            close_threshold: None,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn init_batched_state_merkle_tree_accounts(
    owner: Pubkey,
    params: InitStateTreeAccountsInstructionData,
    output_queue_account_data: &mut [u8],
    output_queue_pubkey: Pubkey,
    queue_rent: u64,
    mt_account_data: &mut [u8],
    mt_pubkey: Pubkey,
    merkle_tree_rent: u64,
    additional_bytes_rent: u64,
) -> Result<(), BatchedMerkleTreeError> {
    let num_batches_input_queue = params.input_queue_num_batches;
    let num_batches_output_queue = params.output_queue_num_batches;
    let height = params.height;

    // Output queue
    {
        let rollover_fee = match params.rollover_threshold {
            Some(rollover_threshold) => {
                let rent = merkle_tree_rent + additional_bytes_rent + queue_rent;
                let rollover_fee = compute_rollover_fee(rollover_threshold, height, rent)?;
                check_rollover_fee_sufficient(rollover_fee, 0, rent, rollover_threshold, height)?;
                rollover_fee
            }
            None => 0,
        };
        msg!(" Output queue rollover_fee: {}", rollover_fee);
        let metadata = QueueMetadata {
            next_queue: Pubkey::default(),
            access_metadata: AccessMetadata::new(owner, params.program_owner, params.forester),
            rollover_metadata: RolloverMetadata::new(
                params.index,
                rollover_fee,
                params.rollover_threshold,
                params.network_fee.unwrap_or_default(),
                params.close_threshold,
                Some(params.additional_bytes),
            ),
            queue_type: QueueType::Output as u64,
            associated_merkle_tree: mt_pubkey,
        };

        ZeroCopyBatchedQueueAccount::init(
            metadata,
            num_batches_output_queue,
            params.output_queue_batch_size,
            params.output_queue_zkp_batch_size,
            output_queue_account_data,
            0,
            0,
        )?;
    }
    let metadata = MerkleTreeMetadata {
        next_merkle_tree: Pubkey::default(),
        access_metadata: AccessMetadata::new(owner, params.program_owner, params.forester),
        rollover_metadata: RolloverMetadata::new(
            params.index,
            // Complete rollover fee is charged when creating an output
            // compressed account by inserting it into the output queue.
            0,
            params.rollover_threshold,
            params.network_fee.unwrap_or_default(),
            params.close_threshold,
            None,
        ),
        associated_queue: output_queue_pubkey,
    };
    msg!("initing mt_account: ");
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
        TreeType::BatchedState,
    )?;
    Ok(())
}

pub fn validate_batched_tree_params(params: InitStateTreeAccountsInstructionData) {
    assert!(params.input_queue_batch_size > 0);
    assert!(params.output_queue_batch_size > 0);
    assert_eq!(
        params.input_queue_batch_size % params.input_queue_zkp_batch_size,
        0,
        "Input queue batch size must divisible by input_queue_zkp_batch_size."
    );
    assert_eq!(
        params.output_queue_batch_size % params.output_queue_zkp_batch_size,
        0,
        "Output queue batch size must divisible by output_queue_zkp_batch_size."
    );
    assert!(
        match_circuit_size(params.input_queue_zkp_batch_size),
        "Zkp batch size not supported. Supported 1, 10, 100, 500, 1000"
    );
    assert!(
        match_circuit_size(params.output_queue_zkp_batch_size),
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
    assert_eq!(params.output_queue_num_batches, 2);
    assert_eq!(params.close_threshold, None);
    assert_eq!(params.height, 26);
}

pub fn match_circuit_size(size: u64) -> bool {
    matches!(size, 10 | 100 | 500 | 1000)
}

pub fn get_state_merkle_tree_account_size_from_params(
    params: InitStateTreeAccountsInstructionData,
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

pub fn assert_state_mt_zero_copy_inited(
    account_data: &mut [u8],
    ref_account: BatchedMerkleTreeAccount,
    num_iters: u64,
) {
    let zero_copy_account =
        ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(account_data)
            .expect("from_bytes_mut failed");
    _assert_mt_zero_copy_inited(
        zero_copy_account,
        ref_account,
        num_iters,
        TreeType::BatchedState as u64,
    );
}

pub fn assert_address_mt_zero_copy_inited(
    account_data: &mut [u8],
    ref_account: BatchedMerkleTreeAccount,
    num_iters: u64,
) {
    let zero_copy_account =
        ZeroCopyBatchedMerkleTreeAccount::address_tree_from_bytes_mut(account_data)
            .expect("from_bytes_mut failed");
    _assert_mt_zero_copy_inited(
        zero_copy_account,
        ref_account,
        num_iters,
        TreeType::Address as u64,
    );
}

fn _assert_mt_zero_copy_inited(
    mut zero_copy_account: ZeroCopyBatchedMerkleTreeAccount,
    ref_account: BatchedMerkleTreeAccount,
    num_iters: u64,
    tree_type: u64,
) {
    let queue = zero_copy_account.get_account().queue;
    let ref_queue = ref_account.queue;
    let num_batches = ref_queue.num_batches as usize;
    let mut next_index = zero_copy_account.get_account().next_index;
    assert_eq!(
        *zero_copy_account.get_account(),
        ref_account,
        "metadata mismatch"
    );

    assert_eq!(
        zero_copy_account.root_history.capacity(),
        ref_account.root_history_capacity as usize,
        "root_history_capacity mismatch"
    );
    if tree_type == TreeType::BatchedState as u64 {
        assert_eq!(
            *zero_copy_account.root_history.get(0).unwrap(),
            light_hasher::Poseidon::zero_bytes()[ref_account.height as usize],
            "root_history not initialized"
        );
    }
    if tree_type == TreeType::BatchedAddress as u64 {
        assert_eq!(
            *zero_copy_account.root_history.get(0).unwrap(),
            ADDRESS_TREE_INIT_ROOT_40,
            "root_history not initialized"
        );
    }
    assert_eq!(
        zero_copy_account.hashchain_store[0].metadata().capacity(),
        ref_account.queue.get_num_zkp_batches() as usize,
        "hashchain_store mismatch"
    );

    if tree_type == TreeType::BatchedAddress as u64 {
        next_index = 2;
    }

    let queue_type = if tree_type == TreeType::BatchedState as u64 {
        QueueType::Input as u64
    } else {
        QueueType::Address as u64
    };
    assert_queue_inited(
        queue,
        ref_queue,
        queue_type,
        &mut zero_copy_account.value_vecs,
        &mut zero_copy_account.bloom_filter_stores,
        &mut zero_copy_account.batches,
        num_batches,
        num_iters,
        next_index,
    );
}

pub struct CreateOutputQueueParams {
    pub owner: Pubkey,
    pub program_owner: Option<Pubkey>,
    pub forester: Option<Pubkey>,
    pub rollover_threshold: Option<u64>,
    pub index: u64,
    pub batch_size: u64,
    pub zkp_batch_size: u64,
    pub additional_bytes: u64,
    pub rent: u64,
    pub associated_merkle_tree: Pubkey,
    pub height: u32,
    pub num_batches: u64,
    pub network_fee: u64,
}

impl CreateOutputQueueParams {
    pub fn from(
        params: InitStateTreeAccountsInstructionData,
        owner: Pubkey,
        rent: u64,
        associated_merkle_tree: Pubkey,
    ) -> Self {
        Self {
            owner, // Default value, should be set appropriately
            program_owner: params.program_owner,
            forester: params.forester,
            rollover_threshold: params.rollover_threshold,
            index: params.index,
            batch_size: params.output_queue_batch_size,
            zkp_batch_size: params.output_queue_zkp_batch_size,
            additional_bytes: params.additional_bytes,
            rent,                   // Default value, should be set appropriately
            associated_merkle_tree, // Default value, should be set appropriately
            height: params.height,
            num_batches: params.output_queue_num_batches,
            network_fee: params.network_fee.unwrap_or_default(),
        }
    }
}

pub fn create_output_queue_account(params: CreateOutputQueueParams) -> BatchedQueueAccount {
    let rollover_fee: u64 = match params.rollover_threshold {
        Some(rollover_threshold) => {
            compute_rollover_fee(rollover_threshold, params.height, params.rent).unwrap()
        }
        None => 0,
    };
    let metadata = QueueMetadata {
        next_queue: Pubkey::default(),
        access_metadata: AccessMetadata {
            owner: params.owner,
            program_owner: params.program_owner.unwrap_or_default(),
            forester: params.forester.unwrap_or_default(),
        },
        rollover_metadata: RolloverMetadata {
            close_threshold: u64::MAX,
            index: params.index,
            rolledover_slot: u64::MAX,
            rollover_threshold: params.rollover_threshold.unwrap_or(u64::MAX),
            rollover_fee,
            network_fee: params.network_fee,
            additional_bytes: params.additional_bytes,
        },
        queue_type: QueueType::Output as u64,
        associated_merkle_tree: params.associated_merkle_tree,
    };
    let queue = BatchMetadata::get_output_queue_default(
        params.batch_size,
        params.zkp_batch_size,
        params.num_batches,
    );
    BatchedQueueAccount {
        metadata,
        queue,
        next_index: 0,
    }
}

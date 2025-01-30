use borsh::{BorshDeserialize, BorshSerialize};
use light_merkle_tree_metadata::{
    access::AccessMetadata,
    merkle_tree::{MerkleTreeMetadata, TreeType},
    queue::{QueueMetadata, QueueType},
    rollover::{check_rollover_fee_sufficient, RolloverMetadata},
};
use light_utils::{
    account::check_account_balance_is_rent_exempt, fee::compute_rollover_fee,
    hashv_to_bn254_field_size_be, pubkey::Pubkey,
};
use solana_program::{account_info::AccountInfo, msg};

use crate::{
    batch_metadata::BatchMetadata,
    constants::{
        DEFAULT_BATCH_SIZE, DEFAULT_BATCH_STATE_TREE_HEIGHT, DEFAULT_CPI_CONTEXT_ACCOUNT_SIZE,
        DEFAULT_ZKP_BATCH_SIZE, TEST_DEFAULT_BATCH_SIZE, TEST_DEFAULT_ZKP_BATCH_SIZE,
    },
    errors::BatchedMerkleTreeError,
    merkle_tree::{get_merkle_tree_account_size, BatchedMerkleTreeAccount},
    queue::{get_output_queue_account_size, BatchedQueueAccount, BatchedQueueMetadata},
};

#[repr(C)]
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
            height: DEFAULT_BATCH_STATE_TREE_HEIGHT,
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
            height: DEFAULT_BATCH_STATE_TREE_HEIGHT,
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
            height: DEFAULT_BATCH_STATE_TREE_HEIGHT,
            root_history_capacity: (DEFAULT_BATCH_SIZE / DEFAULT_ZKP_BATCH_SIZE * 2) as u32,
            bloom_filter_capacity: DEFAULT_BATCH_SIZE * 8,
            network_fee: Some(5000),
            rollover_threshold: Some(95),
            close_threshold: None,
        }
    }
}

/// Initializes the state Merkle tree and output queue accounts.
/// 1. Check rent exemption and that accounts are initialized with the correct size.
/// 2. Initialize the output queue and state Merkle tree accounts.
pub fn init_batched_state_merkle_tree_from_account_info<'a>(
    params: InitStateTreeAccountsInstructionData,
    owner: solana_program::pubkey::Pubkey,
    merkle_tree_account_info: &AccountInfo<'a>,
    queue_account_info: &AccountInfo<'a>,
    additional_bytes_rent: u64,
) -> Result<(), BatchedMerkleTreeError> {
    // 1. Check rent exemption and that accounts are initialized with the correct size.
    let queue_rent;
    let merkle_tree_rent;
    {
        let queue_account_size = get_output_queue_account_size(
            params.output_queue_batch_size,
            params.output_queue_zkp_batch_size,
        );
        let mt_account_size = get_merkle_tree_account_size(
            params.input_queue_batch_size,
            params.bloom_filter_capacity,
            params.input_queue_zkp_batch_size,
            params.root_history_capacity,
            params.height,
        );

        queue_rent = check_account_balance_is_rent_exempt(queue_account_info, queue_account_size)?;

        merkle_tree_rent =
            check_account_balance_is_rent_exempt(merkle_tree_account_info, mt_account_size)?;
    }

    // 2. Initialize the output queue and state Merkle tree accounts.
    let queue_data = &mut queue_account_info.try_borrow_mut_data()?;
    let mt_data = &mut merkle_tree_account_info.try_borrow_mut_data()?;

    init_batched_state_merkle_tree_accounts(
        owner.into(),
        params,
        queue_data,
        (*queue_account_info.key).into(),
        queue_rent,
        mt_data,
        (*merkle_tree_account_info.key).into(),
        merkle_tree_rent,
        additional_bytes_rent,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn init_batched_state_merkle_tree_accounts<'a>(
    owner: Pubkey,
    params: InitStateTreeAccountsInstructionData,
    output_queue_account_data: &mut [u8],
    output_queue_pubkey: Pubkey,
    queue_rent: u64,
    mt_account_data: &'a mut [u8],
    mt_pubkey: Pubkey,
    merkle_tree_rent: u64,
    additional_bytes_rent: u64,
) -> Result<BatchedMerkleTreeAccount<'a>, BatchedMerkleTreeError> {
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
            queue_type: QueueType::BatchedOutput as u64,
            associated_merkle_tree: mt_pubkey,
        };

        BatchedQueueAccount::init(
            output_queue_account_data,
            metadata,
            params.output_queue_batch_size,
            params.output_queue_zkp_batch_size,
            // Output queues have no bloom filter.
            0,
            0,
            output_queue_pubkey,
        )?;
    }
    let metadata = MerkleTreeMetadata {
        next_merkle_tree: Pubkey::default(),
        access_metadata: AccessMetadata::new(owner, params.program_owner, params.forester),
        rollover_metadata: RolloverMetadata::new(
            params.index,
            // The complete rollover fee is charged when creating an output
            // compressed account by inserting it into the output queue.
            0,
            params.rollover_threshold,
            params.network_fee.unwrap_or_default(),
            params.close_threshold,
            None,
        ),
        associated_queue: output_queue_pubkey,
    };

    // Note, the state Merkle tree account contains the input queue,
    // because to insert a nullifier into the input queue the
    // compressed state is spent. To spend compressed state we need
    // to prove inclusion of this state for which we need a root from the tree account.
    BatchedMerkleTreeAccount::init(
        mt_account_data,
        &mt_pubkey,
        metadata,
        params.root_history_capacity,
        params.input_queue_batch_size,
        params.input_queue_zkp_batch_size,
        height,
        params.bloom_filter_num_iters,
        params.bloom_filter_capacity,
        TreeType::BatchedState,
    )
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
    assert!(params.bloom_filter_capacity >= params.input_queue_batch_size * 8);
    assert_eq!(
        params.bloom_filter_capacity % 8,
        0,
        "Bloom filter capacity must be divisible by 8."
    );
    assert!(params.bloom_filter_capacity > 0);
    assert!(params.root_history_capacity > 0);
    assert!(params.input_queue_batch_size > 0);
    assert_eq!(params.close_threshold, None);
    assert_eq!(params.height, DEFAULT_BATCH_STATE_TREE_HEIGHT);
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
    )
}

#[cfg(not(target_os = "solana"))]
pub fn assert_state_mt_zero_copy_inited(
    account_data: &mut [u8],
    ref_account: crate::merkle_tree_metadata::BatchedMerkleTreeMetadata,
) {
    let account = BatchedMerkleTreeAccount::state_from_bytes(account_data)
        .expect("from_bytes_unchecked_mut failed");
    _assert_mt_zero_copy_inited::<{ crate::constants::BATCHED_STATE_TREE_TYPE }>(
        account,
        ref_account,
        TreeType::BatchedState as u64,
    );
}

#[cfg(not(target_os = "solana"))]
pub fn assert_address_mt_zero_copy_inited(
    account_data: &mut [u8],
    ref_account: crate::merkle_tree_metadata::BatchedMerkleTreeMetadata,
) {
    use crate::{constants::BATCHED_ADDRESS_TREE_TYPE, merkle_tree::BatchedMerkleTreeAccount};

    let account = BatchedMerkleTreeAccount::address_from_bytes(account_data)
        .expect("from_bytes_unchecked_mut failed");
    _assert_mt_zero_copy_inited::<BATCHED_ADDRESS_TREE_TYPE>(
        account,
        ref_account,
        TreeType::Address as u64,
    );
}

#[cfg(not(target_os = "solana"))]
fn _assert_mt_zero_copy_inited<const TREE_TYPE: u64>(
    account: BatchedMerkleTreeAccount,
    ref_account: crate::merkle_tree_metadata::BatchedMerkleTreeMetadata,
    tree_type: u64,
) {
    use light_hasher::Hasher;

    let queue = account.queue_metadata;
    let ref_queue = ref_account.queue_metadata;
    assert_eq!(*account, ref_account, "metadata mismatch");

    assert_eq!(
        account.root_history.capacity(),
        ref_account.root_history_capacity as usize,
        "root_history_capacity mismatch"
    );
    if tree_type == TreeType::BatchedState as u64 {
        assert_eq!(
            *account.root_history.get(0).unwrap(),
            light_hasher::Poseidon::zero_bytes()[ref_account.height as usize],
            "root_history not initialized"
        );
    }
    if tree_type == TreeType::BatchedAddress as u64 {
        assert_eq!(
            *account.root_history.get(0).unwrap(),
            crate::constants::ADDRESS_TREE_INIT_ROOT_40,
            "root_history not initialized"
        );
    }
    assert_eq!(
        account.hash_chain_stores[0].capacity(),
        ref_account.queue_metadata.get_num_zkp_batches() as usize,
        "hashchain_store mismatch"
    );

    let queue_type = if tree_type == TreeType::BatchedState as u64 {
        QueueType::BatchedInput as u64
    } else {
        QueueType::BatchedAddress as u64
    };
    crate::queue::assert_queue_inited(queue, ref_queue, queue_type, &mut []);
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
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
    pub queue_pubkey: Pubkey,
    pub height: u32,
    pub network_fee: u64,
}

impl CreateOutputQueueParams {
    pub fn from(
        params: InitStateTreeAccountsInstructionData,
        owner: Pubkey,
        rent: u64,
        associated_merkle_tree: Pubkey,
        queue_pubkey: Pubkey,
    ) -> Self {
        Self {
            owner,
            program_owner: params.program_owner,
            forester: params.forester,
            rollover_threshold: params.rollover_threshold,
            index: params.index,
            batch_size: params.output_queue_batch_size,
            zkp_batch_size: params.output_queue_zkp_batch_size,
            additional_bytes: params.additional_bytes,
            rent,
            associated_merkle_tree,
            height: params.height,
            network_fee: params.network_fee.unwrap_or_default(),
            queue_pubkey,
        }
    }
}

pub fn create_output_queue_account(params: CreateOutputQueueParams) -> BatchedQueueMetadata {
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
        queue_type: QueueType::BatchedOutput as u64,
        associated_merkle_tree: params.associated_merkle_tree,
    };
    let batch_metadata =
        BatchMetadata::new_output_queue(params.batch_size, params.zkp_batch_size).unwrap();
    BatchedQueueMetadata {
        metadata,
        batch_metadata,
        tree_capacity: 2u64.pow(params.height),
        hashed_merkle_tree_pubkey: hashv_to_bn254_field_size_be(&[&params
            .associated_merkle_tree
            .to_bytes()]),
        hashed_queue_pubkey: hashv_to_bn254_field_size_be(&[&params.queue_pubkey.to_bytes()]),
    }
}

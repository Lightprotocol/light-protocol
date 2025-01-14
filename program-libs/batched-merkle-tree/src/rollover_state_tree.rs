use light_merkle_tree_metadata::{errors::MerkleTreeMetadataError, utils::if_equals_none};
use light_utils::pubkey::Pubkey;
use solana_program::msg;

use crate::{
    errors::BatchedMerkleTreeError,
    initialize_state_tree::{
        init_batched_state_merkle_tree_accounts, InitStateTreeAccountsInstructionData,
    },
    merkle_tree::{BatchedMerkleTreeAccount, BatchedMerkleTreeMetadata},
    queue::{BatchedQueueAccount, BatchedQueueMetadata},
};

#[repr(C)]
pub struct RolloverBatchStateTreeParams<'a> {
    pub old_merkle_tree: &'a mut BatchedMerkleTreeAccount<'a>,
    pub old_mt_pubkey: Pubkey,
    pub new_mt_data: &'a mut [u8],
    pub new_mt_rent: u64,
    pub new_mt_pubkey: Pubkey,
    pub old_output_queue: &'a mut BatchedQueueAccount<'a>,
    pub old_queue_pubkey: Pubkey,
    pub new_output_queue_data: &'a mut [u8],
    pub new_output_queue_rent: u64,
    pub new_output_queue_pubkey: Pubkey,
    pub additional_bytes_rent: u64,
    pub additional_bytes: u64,
    pub network_fee: Option<u64>,
}

/// Checks:
/// 1. Merkle tree is ready to be rolled over
/// 2. Merkle tree is not already rolled over
/// 3. Rollover threshold is configured, if not tree cannot be rolled over
///
/// Actions:
/// 1. mark Merkle tree as rolled over in this slot
/// 2. initialize new Merkle tree and output queue with the same parameters
pub fn rollover_batched_state_tree(
    params: RolloverBatchStateTreeParams,
) -> Result<(), BatchedMerkleTreeError> {
    params
        .old_output_queue
        .check_is_associated(&params.old_mt_pubkey)?;

    // Check that old merkle tree is ready for rollover.
    batched_tree_is_ready_for_rollover(params.old_merkle_tree, &params.network_fee)?;
    // Rollover the old merkle tree.
    params
        .old_merkle_tree
        .metadata
        .rollover(params.old_queue_pubkey, params.new_mt_pubkey)?;
    // Rollover the old output queue.
    params
        .old_output_queue
        .metadata
        .rollover(params.old_mt_pubkey, params.new_output_queue_pubkey)?;
    let init_params = InitStateTreeAccountsInstructionData::from(&params);
    let owner = params.old_merkle_tree.metadata.access_metadata.owner;

    // Initialize the new merkle tree and output queue.
    init_batched_state_merkle_tree_accounts(
        owner,
        init_params,
        params.new_output_queue_data,
        params.new_output_queue_pubkey,
        params.new_output_queue_rent,
        params.new_mt_data,
        params.new_mt_pubkey,
        params.new_mt_rent,
        params.additional_bytes_rent,
    )?;
    Ok(())
}

impl From<&RolloverBatchStateTreeParams<'_>> for InitStateTreeAccountsInstructionData {
    #[inline(always)]
    fn from(params: &RolloverBatchStateTreeParams<'_>) -> Self {
        InitStateTreeAccountsInstructionData {
            index: params.old_merkle_tree.metadata.rollover_metadata.index,
            program_owner: if_equals_none(
                params
                    .old_merkle_tree
                    .metadata
                    .access_metadata
                    .program_owner,
                Pubkey::default(),
            ),
            forester: if_equals_none(
                params.old_merkle_tree.metadata.access_metadata.forester,
                Pubkey::default(),
            ),
            height: params.old_merkle_tree.height,
            input_queue_batch_size: params.old_merkle_tree.queue_metadata.batch_size,
            input_queue_zkp_batch_size: params.old_merkle_tree.queue_metadata.zkp_batch_size,
            bloom_filter_capacity: params.old_merkle_tree.queue_metadata.bloom_filter_capacity,
            bloom_filter_num_iters: params.old_merkle_tree.batches[0].num_iters,
            root_history_capacity: params.old_merkle_tree.root_history_capacity,
            network_fee: params.network_fee,
            rollover_threshold: if_equals_none(
                params
                    .old_merkle_tree
                    .metadata
                    .rollover_metadata
                    .rollover_threshold,
                u64::MAX,
            ),
            close_threshold: if_equals_none(
                params
                    .old_merkle_tree
                    .metadata
                    .rollover_metadata
                    .close_threshold,
                u64::MAX,
            ),
            input_queue_num_batches: params.old_merkle_tree.queue_metadata.num_batches,
            additional_bytes: params.additional_bytes,
            output_queue_batch_size: params.old_output_queue.batch_metadata.batch_size,
            output_queue_zkp_batch_size: params.old_output_queue.batch_metadata.zkp_batch_size,
            output_queue_num_batches: params.old_output_queue.batches.len() as u64,
        }
    }
}

// TODO: add unit test
pub fn batched_tree_is_ready_for_rollover(
    metadata: &BatchedMerkleTreeAccount<'_>,
    network_fee: &Option<u64>,
) -> Result<(), BatchedMerkleTreeError> {
    if metadata.metadata.rollover_metadata.rollover_threshold == u64::MAX {
        return Err(MerkleTreeMetadataError::RolloverNotConfigured.into());
    }
    if metadata.next_index
        < ((1 << metadata.height) * metadata.metadata.rollover_metadata.rollover_threshold / 100)
    {
        return Err(MerkleTreeMetadataError::NotReadyForRollover.into());
    }
    if metadata.metadata.rollover_metadata.network_fee == 0 && network_fee.is_some() {
        msg!("Network fee must be 0 for manually forested trees.");
        return Err(BatchedMerkleTreeError::InvalidNetworkFee);
    }
    Ok(())
}

#[repr(C)]
pub struct StateMtRollOverAssertParams {
    pub mt_account_data: Vec<u8>,
    pub ref_mt_account: BatchedMerkleTreeMetadata,
    pub new_mt_account_data: Vec<u8>,
    pub old_mt_pubkey: Pubkey,
    pub new_mt_pubkey: Pubkey,
    pub bloom_filter_num_iters: u64,
    pub ref_rolledover_mt: BatchedMerkleTreeMetadata,
    pub queue_account_data: Vec<u8>,
    pub ref_queue_account: BatchedQueueMetadata,
    pub new_queue_account_data: Vec<u8>,
    pub new_queue_pubkey: Pubkey,
    pub ref_rolledover_queue: BatchedQueueMetadata,
    pub old_queue_pubkey: Pubkey,
    pub slot: u64,
}

#[cfg(not(target_os = "solana"))]
pub fn assert_state_mt_roll_over(params: StateMtRollOverAssertParams) {
    let StateMtRollOverAssertParams {
        mt_account_data,
        ref_mt_account,
        new_mt_account_data,
        old_mt_pubkey,
        new_mt_pubkey,
        bloom_filter_num_iters,
        ref_rolledover_mt,
        mut queue_account_data,
        ref_queue_account,
        mut new_queue_account_data,
        new_queue_pubkey,
        mut ref_rolledover_queue,
        old_queue_pubkey,
        slot,
    } = params;

    ref_rolledover_queue
        .metadata
        .rollover(old_mt_pubkey, new_queue_pubkey)
        .unwrap();
    ref_rolledover_queue
        .metadata
        .rollover_metadata
        .rolledover_slot = slot;

    crate::queue::assert_queue_zero_copy_inited(&mut new_queue_account_data, ref_queue_account, 0);

    let zero_copy_queue =
        BatchedQueueAccount::output_queue_from_bytes_mut(&mut queue_account_data).unwrap();
    assert_eq!(zero_copy_queue.metadata, ref_rolledover_queue.metadata);
    let params = MtRollOverAssertParams {
        mt_account_data,
        ref_mt_account,
        new_mt_account_data,
        new_mt_pubkey,
        bloom_filter_num_iters,
        ref_rolledover_mt,
        old_queue_pubkey,
        slot,
    };

    assert_mt_roll_over(params);
}

// TODO: assert that the rest of the rolled over account didn't change
#[repr(C)]
pub struct MtRollOverAssertParams {
    pub mt_account_data: Vec<u8>,
    pub ref_mt_account: BatchedMerkleTreeMetadata,
    pub new_mt_account_data: Vec<u8>,
    pub new_mt_pubkey: Pubkey,
    pub bloom_filter_num_iters: u64,
    pub ref_rolledover_mt: BatchedMerkleTreeMetadata,
    pub old_queue_pubkey: Pubkey,
    pub slot: u64,
}

#[cfg(not(target_os = "solana"))]
pub fn assert_mt_roll_over(params: MtRollOverAssertParams) {
    let MtRollOverAssertParams {
        mut mt_account_data,
        ref_mt_account,
        mut new_mt_account_data,
        new_mt_pubkey,
        bloom_filter_num_iters,
        mut ref_rolledover_mt,
        old_queue_pubkey,
        slot,
    } = params;

    ref_rolledover_mt
        .metadata
        .rollover(old_queue_pubkey, new_mt_pubkey)
        .unwrap();
    ref_rolledover_mt.metadata.rollover_metadata.rolledover_slot = slot;
    let zero_copy_mt =
        BatchedMerkleTreeAccount::state_tree_from_bytes_mut(&mut mt_account_data).unwrap();
    assert_eq!(*zero_copy_mt.get_metadata(), ref_rolledover_mt);

    crate::initialize_state_tree::assert_state_mt_zero_copy_inited(
        &mut new_mt_account_data,
        ref_mt_account,
        bloom_filter_num_iters,
    );
}

use light_merkle_tree_metadata::{errors::MerkleTreeMetadataError, utils::if_equals_none};
use solana_program::{msg, pubkey::Pubkey};

use crate::{
    errors::BatchedMerkleTreeError,
    initialize_state_tree::{
        assert_state_mt_zero_copy_inited, init_batched_state_merkle_tree_accounts,
        InitStateTreeAccountsInstructionData,
    },
    merkle_tree::{BatchedMerkleTreeAccount, ZeroCopyBatchedMerkleTreeAccount},
    queue::{assert_queue_zero_copy_inited, BatchedQueueAccount, ZeroCopyBatchedQueueAccount},
};

pub struct RolloverBatchStateTreeParams<'a> {
    pub old_merkle_tree: &'a mut ZeroCopyBatchedMerkleTreeAccount,
    pub old_mt_pubkey: Pubkey,
    pub new_mt_data: &'a mut [u8],
    pub new_mt_rent: u64,
    pub new_mt_pubkey: Pubkey,
    pub old_output_queue: &'a mut ZeroCopyBatchedQueueAccount,
    pub old_queue_pubkey: Pubkey,
    pub new_output_queue_data: &'a mut [u8],
    pub new_output_queue_rent: u64,
    pub new_output_queue_pubkey: Pubkey,
    pub additional_bytes_rent: u64,
    pub additional_bytes: u64,
    pub network_fee: Option<u64>,
}

pub fn rollover_batch_state_tree(
    params: RolloverBatchStateTreeParams,
) -> Result<(), BatchedMerkleTreeError> {
    let RolloverBatchStateTreeParams {
        old_merkle_tree,
        old_mt_pubkey,
        new_mt_data,
        new_mt_rent,
        new_mt_pubkey,
        old_output_queue,
        old_queue_pubkey,
        new_output_queue_data,
        new_output_queue_rent,
        new_output_queue_pubkey,
        additional_bytes_rent,
        additional_bytes,
        network_fee,
    } = params;
    old_merkle_tree
        .get_account_mut()
        .metadata
        .rollover(old_queue_pubkey, new_mt_pubkey)?;

    old_output_queue
        .get_account_mut()
        .metadata
        .rollover(old_mt_pubkey, new_output_queue_pubkey)?;
    let old_merkle_tree_account = old_merkle_tree.get_account();

    if old_merkle_tree_account.next_index
        < ((1 << old_merkle_tree_account.height)
            * old_merkle_tree_account
                .metadata
                .rollover_metadata
                .rollover_threshold
            / 100)
    {
        return Err(MerkleTreeMetadataError::NotReadyForRollover.into());
    }
    if old_merkle_tree_account
        .metadata
        .rollover_metadata
        .network_fee
        == 0
        && network_fee.is_some()
    {
        msg!("Network fee must be 0 for manually forested trees.");
        return Err(BatchedMerkleTreeError::InvalidNetworkFee);
    }

    let params = InitStateTreeAccountsInstructionData {
        index: old_merkle_tree_account.metadata.rollover_metadata.index,
        program_owner: if_equals_none(
            old_merkle_tree_account
                .metadata
                .access_metadata
                .program_owner,
            Pubkey::default(),
        ),
        forester: if_equals_none(
            old_merkle_tree_account.metadata.access_metadata.forester,
            Pubkey::default(),
        ),
        height: old_merkle_tree_account.height,
        input_queue_batch_size: old_merkle_tree_account.queue.batch_size,
        input_queue_zkp_batch_size: old_merkle_tree_account.queue.zkp_batch_size,
        bloom_filter_capacity: old_merkle_tree_account.queue.bloom_filter_capacity,
        bloom_filter_num_iters: old_merkle_tree.batches[0].num_iters,
        root_history_capacity: old_merkle_tree_account.root_history_capacity,
        network_fee,
        rollover_threshold: if_equals_none(
            old_merkle_tree_account
                .metadata
                .rollover_metadata
                .rollover_threshold,
            u64::MAX,
        ),
        close_threshold: if_equals_none(
            old_merkle_tree_account
                .metadata
                .rollover_metadata
                .close_threshold,
            u64::MAX,
        ),
        input_queue_num_batches: old_merkle_tree_account.queue.num_batches,
        additional_bytes,
        output_queue_batch_size: old_output_queue.get_account().queue.batch_size,
        output_queue_zkp_batch_size: old_output_queue.get_account().queue.zkp_batch_size,
        output_queue_num_batches: old_output_queue.batches.len() as u64,
    };

    init_batched_state_merkle_tree_accounts(
        old_merkle_tree_account.metadata.access_metadata.owner,
        params,
        new_output_queue_data,
        new_output_queue_pubkey,
        new_output_queue_rent,
        new_mt_data,
        new_mt_pubkey,
        new_mt_rent,
        additional_bytes_rent,
    )
}

pub struct StateMtRollOverAssertParams {
    pub mt_account_data: Vec<u8>,
    pub ref_mt_account: BatchedMerkleTreeAccount,
    pub new_mt_account_data: Vec<u8>,
    pub old_mt_pubkey: Pubkey,
    pub new_mt_pubkey: Pubkey,
    pub bloom_filter_num_iters: u64,
    pub ref_rolledover_mt: BatchedMerkleTreeAccount,
    pub queue_account_data: Vec<u8>,
    pub ref_queue_account: BatchedQueueAccount,
    pub new_queue_account_data: Vec<u8>,
    pub new_queue_pubkey: Pubkey,
    pub ref_rolledover_queue: BatchedQueueAccount,
    pub old_queue_pubkey: Pubkey,
    pub slot: u64,
}

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

    println!(
        "ref_rolledover_queue
        .metadata: {:?}",
        ref_rolledover_queue.metadata
    );
    println!("old_queue_pubkey: {:?}", old_queue_pubkey);
    println!("old_mt_pubkey: {:?}", old_mt_pubkey);
    println!("new queue pubkey: {:?}", new_queue_pubkey);
    println!("new mt pubkey: {:?}", new_mt_pubkey);
    ref_rolledover_queue
        .metadata
        .rollover(old_mt_pubkey, new_queue_pubkey)
        .unwrap();
    ref_rolledover_queue
        .metadata
        .rollover_metadata
        .rolledover_slot = slot;

    assert_queue_zero_copy_inited(&mut new_queue_account_data, ref_queue_account, 0);
    println!("asserted queue roll over");

    let zero_copy_queue =
        ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut queue_account_data).unwrap();
    assert_eq!(
        zero_copy_queue.get_account().metadata,
        ref_rolledover_queue.metadata
    );
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
pub struct MtRollOverAssertParams {
    pub mt_account_data: Vec<u8>,
    pub ref_mt_account: BatchedMerkleTreeAccount,
    pub new_mt_account_data: Vec<u8>,
    pub new_mt_pubkey: Pubkey,
    pub bloom_filter_num_iters: u64,
    pub ref_rolledover_mt: BatchedMerkleTreeAccount,
    pub old_queue_pubkey: Pubkey,
    pub slot: u64,
}

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
        ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(&mut mt_account_data).unwrap();
    assert_eq!(*zero_copy_mt.get_account(), ref_rolledover_mt);

    assert_state_mt_zero_copy_inited(
        &mut new_mt_account_data,
        ref_mt_account,
        bloom_filter_num_iters,
    );
}

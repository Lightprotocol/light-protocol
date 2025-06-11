use light_account_checks::{checks::check_account_balance_is_rent_exempt, AccountInfoTrait};
use light_compressed_account::pubkey::Pubkey;
use light_merkle_tree_metadata::{errors::MerkleTreeMetadataError, utils::if_equals_none};

use crate::{
    errors::BatchedMerkleTreeError,
    initialize_state_tree::{
        init_batched_state_merkle_tree_accounts, InitStateTreeAccountsInstructionData,
    },
    merkle_tree::BatchedMerkleTreeAccount,
    queue::BatchedQueueAccount,
};

#[derive(Debug)]
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

/// Rollover an almost full batched state tree,
/// ie create a new batched Merkle tree and output queue
/// with the same parameters as the old accounts,
/// and mark the old accounts as rolled over.
/// The old tree and queue can be used until completely full.
///
/// 1. Check Merkle tree account discriminator, tree type, and program ownership.
/// 2. Check Queue account discriminator, and program ownership.
/// 3. Check that new Merkle tree account is exactly rent exempt.
/// 4. Check that new Queue account is exactly rent exempt.
/// 5. Rollover the old Merkle tree and queue to new Merkle tree and queue.
///
/// Note, reimbursed rent for additional bytes is calculated from old Merkle tree accounts
/// additional bytes since those are the basis for the old trees rollover fee.
/// If new additional_bytes is greater than old additional_bytes additional
/// rent reimbursements need to be calculated outside of this function.
pub fn rollover_batched_state_tree_from_account_info<A: AccountInfoTrait>(
    old_state_merkle_tree: &A,
    new_state_merkle_tree: &A,
    old_output_queue: &A,
    new_output_queue: &A,
    additional_bytes: u64,
    network_fee: Option<u64>,
) -> Result<u64, BatchedMerkleTreeError> {
    // 1. Check Merkle tree account discriminator, tree type, and program ownership.
    let old_merkle_tree_account =
        &mut BatchedMerkleTreeAccount::state_from_account_info(old_state_merkle_tree)?;

    // 2. Check Queue account discriminator, and program ownership.
    let old_output_queue_account =
        &mut BatchedQueueAccount::output_from_account_info(old_output_queue)?;

    // 3. Check that new Merkle tree account is exactly rent exempt.
    let merkle_tree_rent = check_account_balance_is_rent_exempt(
        new_state_merkle_tree,
        old_state_merkle_tree.data_len(),
    )?;

    // 4. Check that new Queue account is exactly rent exempt.
    let queue_rent =
        check_account_balance_is_rent_exempt(new_output_queue, old_output_queue.data_len())?;

    let additional_bytes_rent = A::get_min_rent_balance(
        old_output_queue_account
            .metadata
            .rollover_metadata
            .additional_bytes as usize,
    )?;

    let new_mt_data = &mut new_state_merkle_tree.try_borrow_mut_data()?;
    let params = RolloverBatchStateTreeParams {
        old_merkle_tree: old_merkle_tree_account,
        old_mt_pubkey: old_state_merkle_tree.key().into(),
        new_mt_data,
        new_mt_rent: merkle_tree_rent,
        new_mt_pubkey: new_state_merkle_tree.key().into(),
        old_output_queue: old_output_queue_account,
        old_queue_pubkey: old_output_queue.key().into(),
        new_output_queue_data: &mut new_output_queue.try_borrow_mut_data()?,
        new_output_queue_rent: queue_rent,
        new_output_queue_pubkey: new_output_queue.key().into(),
        additional_bytes_rent,
        additional_bytes,
        network_fee,
    };

    // 5. Rollover the old Merkle tree and queue to new Merkle tree and queue.
    rollover_batched_state_tree(params)?;
    let reimbursement_for_rent = merkle_tree_rent + queue_rent + additional_bytes_rent;
    // 6. Check that queue account is rent exempt post rollover.
    #[cfg(target_os = "solana")]
    if old_output_queue
        .lamports()
        .saturating_sub(reimbursement_for_rent)
        == 0
    {
        return Err(MerkleTreeMetadataError::NotReadyForRollover.into());
    }
    Ok(reimbursement_for_rent)
}

/// Rollover an almost full batched state tree,
/// ie create a new batched Merkle tree and output queue
/// with the same parameters, and mark the old accounts as rolled over.
/// The old tree and queue can be used until these are completely full.
///
/// Steps:
/// 1. Check that Merkle tree is ready to be rolled over:
///    1.1. rollover threshold is configured
///    1.2. next index is greater than rollover threshold
///    1.3. the network fee is not set if the current fee is zero
/// 2. Rollover Merkle tree and check:
///    2.1. Merkle tree and queue are associated.
///    2.2. Rollover is configured.
///    2.3. Tree is not already rolled over.
///    2.4. Mark as rolled over in this slot.
/// 3. Rollover output queue and check:
///    3.1. Merkle tree and queue are associated.
///    3.2. Rollover is configured.
///    3.3. Tree is not already rolled over.
///    3.4. Mark as rolled over in this slot.
/// 4. Initialize new Merkle tree and output queue
///    with the same parameters as old accounts.
pub fn rollover_batched_state_tree(
    params: RolloverBatchStateTreeParams,
) -> Result<(), BatchedMerkleTreeError> {
    // 1. Check that old merkle tree is ready for rollover.
    batched_tree_is_ready_for_rollover(params.old_merkle_tree, &params.network_fee)?;
    // 2. Rollover the old merkle tree.
    params
        .old_merkle_tree
        .metadata
        .rollover(params.old_queue_pubkey, params.new_mt_pubkey)?;
    // 3. Rollover the old output queue.
    params
        .old_output_queue
        .metadata
        .rollover(params.old_mt_pubkey, params.new_output_queue_pubkey)?;
    let init_params = InitStateTreeAccountsInstructionData::from(&params);
    let owner = params.old_merkle_tree.metadata.access_metadata.owner;

    // 4. Initialize the new merkle tree and output queue.
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
            input_queue_batch_size: params.old_merkle_tree.queue_batches.batch_size,
            input_queue_zkp_batch_size: params.old_merkle_tree.queue_batches.zkp_batch_size,
            bloom_filter_capacity: params.old_merkle_tree.queue_batches.bloom_filter_capacity,
            // All num iters are the same.
            bloom_filter_num_iters: params.old_merkle_tree.queue_batches.batches[0].num_iters,
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
            additional_bytes: params.additional_bytes,
            output_queue_batch_size: params.old_output_queue.batch_metadata.batch_size,
            output_queue_zkp_batch_size: params.old_output_queue.batch_metadata.zkp_batch_size,
        }
    }
}

/// Check that:
/// 1. rollover threshold is configured
/// 2. next index is greater than rollover threshold
/// 3. the network fee is not set if the current fee is zero
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
        #[cfg(feature = "solana")]
        solana_msg::msg!("Network fee must be 0 for manually forested trees.");
        return Err(BatchedMerkleTreeError::InvalidNetworkFee);
    }
    Ok(())
}

#[cfg(feature = "test-only")]
pub mod test_utils {
    use light_compressed_account::pubkey::Pubkey;

    use crate::{
        initialize_state_tree::test_utils::assert_state_mt_zero_copy_initialized,
        merkle_tree::BatchedMerkleTreeAccount,
        merkle_tree_metadata::BatchedMerkleTreeMetadata,
        queue::{
            test_utils::assert_queue_zero_copy_inited, BatchedQueueAccount, BatchedQueueMetadata,
        },
    };

    #[repr(C)]
    pub struct StateMtRollOverAssertParams {
        pub mt_account_data: Vec<u8>,
        pub ref_mt_account: BatchedMerkleTreeMetadata,
        pub new_mt_account_data: Vec<u8>,
        pub old_mt_pubkey: Pubkey,
        pub new_mt_pubkey: Pubkey,
        pub ref_rolledover_mt: BatchedMerkleTreeMetadata,
        pub queue_account_data: Vec<u8>,
        pub ref_queue_account: BatchedQueueMetadata,
        pub new_queue_account_data: Vec<u8>,
        pub new_queue_pubkey: Pubkey,
        pub ref_rolledover_queue: BatchedQueueMetadata,
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

        assert_queue_zero_copy_inited(&mut new_queue_account_data, ref_queue_account);

        let zero_copy_queue =
            BatchedQueueAccount::output_from_bytes(&mut queue_account_data).unwrap();
        assert_eq!(zero_copy_queue.metadata, ref_rolledover_queue.metadata);
        let params = MtRollOverAssertParams {
            mt_account_data,
            ref_mt_account,
            new_mt_account_data,
            new_mt_pubkey,
            ref_rolledover_mt,
            old_queue_pubkey,
            slot,
            old_mt_pubkey,
        };

        assert_mt_roll_over(params);
    }

    #[repr(C)]
    pub struct MtRollOverAssertParams {
        pub mt_account_data: Vec<u8>,
        pub ref_mt_account: BatchedMerkleTreeMetadata,
        pub new_mt_account_data: Vec<u8>,
        pub new_mt_pubkey: Pubkey,
        pub ref_rolledover_mt: BatchedMerkleTreeMetadata,
        pub old_queue_pubkey: Pubkey,
        pub slot: u64,
        old_mt_pubkey: Pubkey,
    }

    pub fn assert_mt_roll_over(params: MtRollOverAssertParams) {
        let MtRollOverAssertParams {
            mut mt_account_data,
            ref_mt_account,
            mut new_mt_account_data,
            new_mt_pubkey,
            mut ref_rolledover_mt,
            old_queue_pubkey,
            slot,
            old_mt_pubkey,
        } = params;

        ref_rolledover_mt
            .metadata
            .rollover(old_queue_pubkey, new_mt_pubkey)
            .unwrap();
        ref_rolledover_mt.metadata.rollover_metadata.rolledover_slot = slot;
        let zero_copy_mt =
            BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &old_mt_pubkey)
                .unwrap();
        assert_eq!(*zero_copy_mt.get_metadata(), ref_rolledover_mt);

        assert_state_mt_zero_copy_initialized(
            &mut new_mt_account_data,
            ref_mt_account,
            &new_mt_pubkey,
        );
    }
}

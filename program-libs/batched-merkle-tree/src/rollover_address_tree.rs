use light_account_checks::{checks::check_account_balance_is_rent_exempt, AccountInfoTrait};
use light_compressed_account::pubkey::Pubkey;
#[cfg(target_os = "solana")]
use light_merkle_tree_metadata::errors::MerkleTreeMetadataError;
use light_merkle_tree_metadata::utils::if_equals_none;

use crate::{
    errors::BatchedMerkleTreeError,
    initialize_address_tree::{
        init_batched_address_merkle_tree_account, InitAddressTreeAccountsInstructionData,
    },
    merkle_tree::BatchedMerkleTreeAccount,
    rollover_state_tree::batched_tree_is_ready_for_rollover,
};
pub fn rollover_batched_address_tree_from_account_info<A: AccountInfoTrait>(
    old_account: &A,
    new_account: &A,
    network_fee: Option<u64>,
) -> Result<u64, BatchedMerkleTreeError> {
    let new_mt_rent = check_account_balance_is_rent_exempt(new_account, old_account.data_len())?;
    #[cfg(target_os = "solana")]
    if old_account.lamports().checked_sub(new_mt_rent).unwrap() == 0 {
        return Err(MerkleTreeMetadataError::NotReadyForRollover.into());
    }
    let mut old_merkle_tree = BatchedMerkleTreeAccount::address_from_account_info(old_account)?;
    let mut new_mt_data = new_account.try_borrow_mut_data()?;
    rollover_batched_address_tree(
        &mut old_merkle_tree,
        &mut new_mt_data,
        new_mt_rent,
        new_account.key().into(),
        network_fee,
    )?;
    Ok(new_mt_rent)
}

/// Rollover an almost full batched address tree,
/// ie create a new batched Merkle tree account
/// with the same parameters, and mark the old account as rolled over.
/// The old tree can be used until it is completely full.
///
/// Steps:
/// 1. Check that Merkle tree is ready to be rolled over:
///    1.1. rollover threshold is configured
///    1.2. next index is greater than rollover threshold
///    1.3. the network fee is not set if the current fee is zero
/// 2. Rollover the old Merkle tree and check:
///    2.2. Rollover is configured.
///    2.3. Tree is not already rolled over.
///    2.4. Mark as rolled over in this slot.
/// 3. Initialize new batched address Merkle tree account
///    with the same parameters as the old account.
pub fn rollover_batched_address_tree<'a>(
    old_merkle_tree: &mut BatchedMerkleTreeAccount<'a>,
    new_mt_data: &'a mut [u8],
    new_mt_rent: u64,
    new_mt_pubkey: Pubkey,
    network_fee: Option<u64>,
) -> Result<BatchedMerkleTreeAccount<'a>, BatchedMerkleTreeError> {
    // 1. Check that old merkle tree is ready for rollover.
    batched_tree_is_ready_for_rollover(old_merkle_tree, &network_fee)?;

    // 2. Rollover the old merkle tree.
    // - Address don't have an associated queue account (Pubkey::default()).
    old_merkle_tree
        .metadata
        .rollover(Pubkey::default(), new_mt_pubkey)?;

    // 3. Initialize the new address merkle tree.
    let params = create_batched_address_tree_init_params(old_merkle_tree, network_fee);
    let owner = old_merkle_tree.metadata.access_metadata.owner;
    init_batched_address_merkle_tree_account(owner, params, new_mt_data, new_mt_rent, new_mt_pubkey)
}

fn create_batched_address_tree_init_params(
    old_merkle_tree: &mut BatchedMerkleTreeAccount,
    network_fee: Option<u64>,
) -> InitAddressTreeAccountsInstructionData {
    InitAddressTreeAccountsInstructionData {
        index: old_merkle_tree.metadata.rollover_metadata.index,
        program_owner: if_equals_none(
            old_merkle_tree.metadata.access_metadata.program_owner,
            Pubkey::default(),
        ),
        forester: if_equals_none(
            old_merkle_tree.metadata.access_metadata.forester,
            Pubkey::default(),
        ),
        height: old_merkle_tree.height,
        input_queue_batch_size: old_merkle_tree.queue_batches.batch_size,
        input_queue_zkp_batch_size: old_merkle_tree.queue_batches.zkp_batch_size,
        bloom_filter_capacity: old_merkle_tree.queue_batches.bloom_filter_capacity,
        bloom_filter_num_iters: old_merkle_tree.queue_batches.batches[0].num_iters,
        root_history_capacity: old_merkle_tree.root_history_capacity,
        network_fee,
        rollover_threshold: if_equals_none(
            old_merkle_tree
                .metadata
                .rollover_metadata
                .rollover_threshold,
            u64::MAX,
        ),
        close_threshold: if_equals_none(
            old_merkle_tree.metadata.rollover_metadata.close_threshold,
            u64::MAX,
        ),
    }
}

#[cfg(feature = "test-only")]
pub mod test_utils {
    use super::*;
    use crate::{
        initialize_state_tree::test_utils::assert_address_mt_zero_copy_initialized,
        merkle_tree::BatchedMerkleTreeAccount,
    };
    pub fn assert_address_mt_roll_over(
        mut old_mt_account_data: Vec<u8>,
        mut old_ref_mt_account: crate::merkle_tree_metadata::BatchedMerkleTreeMetadata,
        old_mt_pubkey: Pubkey,
        mut new_mt_account_data: Vec<u8>,
        new_ref_mt_account: crate::merkle_tree_metadata::BatchedMerkleTreeMetadata,
        new_mt_pubkey: Pubkey,
    ) {
        old_ref_mt_account
            .metadata
            .rollover(Pubkey::default(), new_mt_pubkey)
            .unwrap();

        let old_mt_account =
            BatchedMerkleTreeAccount::address_from_bytes(&mut old_mt_account_data, &old_mt_pubkey)
                .unwrap();
        assert_eq!(*old_mt_account.get_metadata(), old_ref_mt_account);
        assert_address_mt_zero_copy_initialized(
            &mut new_mt_account_data,
            new_ref_mt_account,
            &new_mt_pubkey,
        );
    }
}

use light_merkle_tree_metadata::{errors::MerkleTreeMetadataError, utils::if_equals_none};
use solana_program::{msg, pubkey::Pubkey};

use crate::{
    errors::BatchedMerkleTreeError,
    initialize_address_tree::{
        init_batched_address_merkle_tree_account, InitAddressTreeAccountsInstructionData,
    },
    initialize_state_tree::assert_address_mt_zero_copy_inited,
    merkle_tree::{BatchedMerkleTreeAccount, BatchedMerkleTreeMetadata},
};

pub fn rollover_batch_address_tree(
    old_merkle_tree: &mut BatchedMerkleTreeAccount,
    new_mt_data: &mut [u8],
    new_mt_rent: u64,
    new_mt_pubkey: Pubkey,
    network_fee: Option<u64>,
) -> Result<(), BatchedMerkleTreeError> {
    old_merkle_tree
        .get_metadata_mut()
        .metadata
        .rollover(Pubkey::default(), new_mt_pubkey)?;
    let old_merkle_tree_account = old_merkle_tree.get_metadata();

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
        return Err(crate::errors::BatchedMerkleTreeError::InvalidNetworkFee);
    }

    let params = InitAddressTreeAccountsInstructionData {
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
        input_queue_batch_size: old_merkle_tree_account.queue_metadata.batch_size,
        input_queue_zkp_batch_size: old_merkle_tree_account.queue_metadata.zkp_batch_size,
        bloom_filter_capacity: old_merkle_tree_account.queue_metadata.bloom_filter_capacity,
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
        input_queue_num_batches: old_merkle_tree_account.queue_metadata.num_batches,
    };

    init_batched_address_merkle_tree_account(
        old_merkle_tree_account.metadata.access_metadata.owner,
        params,
        new_mt_data,
        new_mt_rent,
    )
}

// TODO: assert that remainder of old_mt_account_data is not changed
pub fn assert_address_mt_roll_over(
    mut old_mt_account_data: Vec<u8>,
    mut old_ref_mt_account: BatchedMerkleTreeMetadata,
    mut new_mt_account_data: Vec<u8>,
    new_ref_mt_account: BatchedMerkleTreeMetadata,
    new_mt_pubkey: Pubkey,
    bloom_filter_num_iters: u64,
) {
    old_ref_mt_account
        .metadata
        .rollover(Pubkey::default(), new_mt_pubkey)
        .unwrap();
    let old_mt_account =
        BatchedMerkleTreeAccount::address_tree_from_bytes_mut(&mut old_mt_account_data).unwrap();
    assert_eq!(old_mt_account.get_metadata(), &old_ref_mt_account);

    assert_address_mt_zero_copy_inited(
        &mut new_mt_account_data,
        new_ref_mt_account,
        bloom_filter_num_iters,
    );
}

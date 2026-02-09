use light_compressed_account::instruction_data::zero_copy::ZPackedReadOnlyCompressedAccount;
use light_program_profiler::profile;
use pinocchio::{msg, program_error::ProgramError};

use crate::{
    accounts::remaining_account_checks::AcpAccount, errors::SystemProgramError,
    utils::get_queue_and_tree_accounts, Result,
};

/// For each read-only account
/// 1. prove inclusion by index in the output queue if leaf index should exist in the output queue.
///    1.1. if inclusion was proven by index, return Ok.
/// 2. prove non-inclusion in the bloom filters
///    2.1. skip cleared batches.
///    2.2. prove non-inclusion in the bloom filters for each batch.
#[inline(always)]
#[profile]
pub fn verify_read_only_account_inclusion_by_index(
    accounts: &mut [AcpAccount<'_>],
    read_only_accounts: &[ZPackedReadOnlyCompressedAccount],
) -> Result<usize> {
    let mut num_prove_read_only_accounts_prove_by_index = 0;
    for read_only_account in read_only_accounts.iter() {
        let queue_index = read_only_account.merkle_context.queue_pubkey_index;
        let tree_index = read_only_account.merkle_context.merkle_tree_pubkey_index;
        let (output_queue_account_info, merkle_tree_account_info) =
            get_queue_and_tree_accounts(accounts, queue_index as usize, tree_index as usize)?;

        let output_queue = if let AcpAccount::OutputQueue(queue) = output_queue_account_info {
            queue
        } else {
            msg!(format!(
                "Read only account is not an OutputQueue {:?} ",
                read_only_account
            )
            .as_str());
            return Err(SystemProgramError::InvalidAccount.into());
        };
        let merkle_tree = if let AcpAccount::BatchedStateTree(tree) = merkle_tree_account_info {
            tree
        } else {
            msg!(format!(
                "Read only account is not a BatchedStateTree {:?}",
                read_only_account
            )
            .as_str());
            return Err(SystemProgramError::InvalidAccount.into());
        };
        output_queue
            .check_is_associated(merkle_tree.pubkey())
            .map_err(ProgramError::from)?;

        // Checks inclusion by index in the output queue if leaf index should exist in the output queue.
        // Else does nothing.
        let proved_inclusion = output_queue
            .prove_inclusion_by_index(
                read_only_account.merkle_context.leaf_index.into(),
                &read_only_account.account_hash,
            )
            .map_err(|_| ProgramError::from(SystemProgramError::ReadOnlyAccountDoesNotExist))?;
        if read_only_account.merkle_context.prove_by_index() {
            num_prove_read_only_accounts_prove_by_index += 1;
        }
        // If a read-only account is marked as proven by index
        // inclusion proof by index has to be successful
        // -> proved_inclusion == true.
        if !proved_inclusion && read_only_account.merkle_context.prove_by_index() {
            pinocchio::msg!(
                "Expected read-only account in the output queue but account does not exist."
            );
            return Err(SystemProgramError::ReadOnlyAccountDoesNotExist.into());
        }
        // If we prove inclusion by index we do not need to check non-inclusion in bloom filters.
        // Since proving inclusion by index of non-read
        // only accounts overwrites the leaf in the output queue.
        if !proved_inclusion {
            merkle_tree
                .check_input_queue_non_inclusion(&read_only_account.account_hash)
                .map_err(|_| ProgramError::from(SystemProgramError::ReadOnlyAccountDoesNotExist))?;
        }
    }
    Ok(num_prove_read_only_accounts_prove_by_index)
}

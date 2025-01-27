use account_compression::{context::AcpAccount, insert_into_queues::get_queue_and_tree_accounts};
use anchor_lang::prelude::*;
use light_utils::instruction::instruction_data_zero_copy::ZPackedReadOnlyCompressedAccount;

use crate::errors::SystemProgramError;

/// For each read-only account
/// 1. prove inclusion by index in the output queue if leaf index should exist in the output queue.
///    1.1. if inclusion was proven by index, return Ok.
/// 2. prove non-inclusion in the bloom filters
///    2.1. skip cleared batches.
///    2.2. prove non-inclusion in the bloom filters for each batch.
#[inline(always)]
pub fn verify_read_only_account_inclusion_by_index<'a>(
    accounts: &mut [AcpAccount<'a, '_>],
    read_only_accounts: &'a [ZPackedReadOnlyCompressedAccount],
) -> Result<usize> {
    let mut num_prove_read_only_accounts_prove_by_index = 0;
    for read_only_account in read_only_accounts.iter() {
        let queue_index = read_only_account
            .merkle_context
            .nullifier_queue_pubkey_index;
        let tree_index = read_only_account.merkle_context.merkle_tree_pubkey_index;
        let (output_queue_account_info, merkle_tree_account_info) =
            get_queue_and_tree_accounts(accounts, queue_index as usize, tree_index as usize)?;

        let output_queue = if let AcpAccount::OutputQueue(queue) = output_queue_account_info {
            queue
        } else {
            msg!(
                "Read only account is not an OutputQueue {:?} ",
                read_only_account
            );
            return err!(SystemProgramError::InvalidAccount);
        };
        let merkle_tree = if let AcpAccount::BatchedStateTree(tree) = merkle_tree_account_info {
            tree
        } else {
            msg!(
                "Read only account is not a BatchedStateTree {:?}",
                read_only_account
            );
            return err!(SystemProgramError::InvalidAccount);
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
            .map_err(|_| SystemProgramError::ReadOnlyAccountDoesNotExist)?;
        if read_only_account.merkle_context.prove_by_index() {
            num_prove_read_only_accounts_prove_by_index += 1;
        }
        // If a read-only account is marked as proven by index
        // inclusion proof by index has to be successful
        // -> proved_inclusion == true.
        if !proved_inclusion && read_only_account.merkle_context.prove_by_index() {
            msg!("Expected read-only account in the output queue but account does not exist.");
            return err!(SystemProgramError::ReadOnlyAccountDoesNotExist);
        }
        // If we prove inclusion by index we do not need to check non-inclusion in bloom filters.
        // Since proving inclusion by index of non-read
        // only accounts overwrites the leaf in the output queue.
        if !proved_inclusion {
            merkle_tree
                .check_input_queue_non_inclusion(&read_only_account.account_hash)
                .map_err(|_| SystemProgramError::ReadOnlyAccountDoesNotExist)?;
        }
    }
    Ok(num_prove_read_only_accounts_prove_by_index)
}

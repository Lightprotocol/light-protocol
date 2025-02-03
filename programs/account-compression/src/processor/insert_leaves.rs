use anchor_lang::prelude::*;
use light_utils::instruction::insert_into_queues::AppendLeavesInput;

use crate::{context::AcpAccount, errors::AccountCompressionErrorCode};

/// Perform batch appends to Merkle trees provided as remaining accounts. Leaves
/// are assumed to be ordered by Merkle tree account.
/// 1. Iterate over all remaining accounts (Merkle tree accounts)
/// 2. get first leaves that points to current Merkle tree account
/// 3. get last leaf that points to current Merkle tree account
/// 4. append batch to Merkle tree or insert into output queue
///     based on discriminator
/// 5. transfer rollover fee
/// 6. check if all leaves are processed
///     return Ok(()) if all leaves are processed
pub fn process_append_leaves_to_merkle_trees<'a, 'b, 'c: 'info, 'info>(
    leaves: &[AppendLeavesInput],
    start_output_appends: u8,
    num_output_queues: u8,
    accounts: &mut [AcpAccount<'a, 'info>],
) -> Result<()> {
    if leaves.is_empty() {
        return Ok(());
    }
    if leaves.len() > u8::MAX as usize {
        return err!(AccountCompressionErrorCode::TooManyLeaves);
    }
    let mut leaves_processed: u8 = 0;
    // 1. Iterate over all remaining accounts (Merkle tree or output queue accounts)
    for i in start_output_appends..start_output_appends + num_output_queues {
        let account = &mut accounts[i as usize];
        // 2. get first leaves that points to current Merkle tree account
        let start = match leaves.iter().position(|x| x.index == i) {
            Some(pos) => Ok(pos),
            None => err!(AccountCompressionErrorCode::NoLeavesForMerkleTree),
        }?;
        // 3. get last leaf that points to current Merkle tree account
        let end = match leaves[start..].iter().position(|x| x.index != i) {
            Some(pos) => pos + start,
            None => leaves.len(),
        };
        let batch_size = (end - start) as u8;
        leaves_processed += batch_size;

        // 4. append batch to Merkle tree or insert into output queue
        match account {
            AcpAccount::OutputQueue(queue) => {
                for leaf in leaves[start..end].iter() {
                    queue
                        .insert_into_current_batch(&leaf.leaf)
                        .map_err(ProgramError::from)?;
                }
            }
            AcpAccount::StateTree((_, merkle_tree)) => {
                merkle_tree
                    .append_batch(
                        &leaves[start..end]
                            .iter()
                            .map(|x| &x.leaf)
                            .collect::<Vec<&[u8; 32]>>(),
                    )
                    .map_err(ProgramError::from)?;
            }

            _ => {
                return err!(
                    AccountCompressionErrorCode::StateMerkleTreeAccountDiscriminatorMismatch
                )
            }
        }
    }
    // 5. check if all leaves are processed
    if leaves_processed != leaves.len() as u8 {
        msg!("leaves processed {}", leaves_processed);
        msg!("leaves {}, ", leaves.len());
        err!(crate::errors::AccountCompressionErrorCode::NotAllLeavesProcessed)
    } else {
        Ok(())
    }
}

use anchor_lang::prelude::*;

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use crate::{context::AcpAccount, errors::AccountCompressionErrorCode};
#[repr(C)]
#[derive(
    KnownLayout,
    IntoBytes,
    Immutable,
    Copy,
    Clone,
    FromBytes,
    AnchorSerialize,
    AnchorDeserialize,
    PartialEq,
    Debug,
    Unaligned,
)]
pub struct AppendLeavesInput {
    pub index: u8,
    pub leaf: [u8; 32],
}

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
    // ctx: &Context<'a, 'b, 'c, 'info, AppendLeaves<'info>>,
    leaves: &[AppendLeavesInput],
    num_unique_appends: u8,
    accounts: &mut [AcpAccount<'a, 'info>],
) -> Result<()> {
    if leaves.is_empty() {
        return Ok(());
    }
    let mut leaves_processed: usize = 0;
    // 1. Iterate over all remaining accounts (Merkle tree or output queue accounts)
    for i in 0..num_unique_appends as usize {
        // 2. get first leaves that points to current Merkle tree account
        let start = match leaves.iter().position(|x| x.index as usize == i) {
            Some(pos) => Ok(pos),
            None => err!(AccountCompressionErrorCode::NoLeavesForMerkleTree),
        }?;
        // 3. get last leaf that points to current Merkle tree account
        let end = match leaves[start..].iter().position(|x| x.index as usize != i) {
            Some(pos) => pos + start,
            None => leaves.len(),
        };
        let batch_size = end - start;
        leaves_processed += batch_size;

        // 4. append batch to Merkle tree or insert into output queue
        match &mut accounts[i] {
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
            AcpAccount::OutputQueue(queue) => {
                for leaf in leaves[start..end].iter() {
                    queue
                        .insert_into_current_batch(&leaf.leaf)
                        .map_err(ProgramError::from)?;
                }
            }
            _ => {
                return err!(
                    AccountCompressionErrorCode::StateMerkleTreeAccountDiscriminatorMismatch
                )
            }
        }

        // 5. transfer rollover fee (rollover fee is transferred in the system program)
        // transfer_lamports_cpi(&accounts[0], merkle_tree_acc_info, rollover_fee)?;
    }
    // 6. check if all leaves are processed
    if leaves_processed != leaves.len() {
        err!(crate::errors::AccountCompressionErrorCode::NotAllLeavesProcessed)
    } else {
        Ok(())
    }
}

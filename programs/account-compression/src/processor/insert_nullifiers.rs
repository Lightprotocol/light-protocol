use anchor_lang::prelude::*;
use light_batched_merkle_tree::queue::BatchedQueueAccount;
use light_compressed_account::instruction_data::insert_into_queues::InsertNullifierInput;
use num_bigint::BigUint;

use crate::{
    context::AcpAccount, errors::AccountCompressionErrorCode,
    insert_into_queues::get_queue_and_tree_accounts, queue_from_bytes_zero_copy_mut, QueueAccount,
};

#[inline(always)]
pub fn insert_nullifiers(
    num_queues: u8,
    tx_hash: [u8; 32],
    nullifiers: &[InsertNullifierInput],
    accounts: &mut [AcpAccount<'_, '_>],
    current_slot: &u64,
) -> Result<()> {
    if nullifiers.is_empty() {
        return Ok(());
    }

    // 1. Gather unique (tree_index, queue_index) pairs in the order they appear,
    //    capped at `num_queues`.
    let mut visited = Vec::with_capacity(num_queues as usize);
    // Always push the first one
    visited.push((nullifiers[0].tree_index, nullifiers[0].queue_index));

    for nf in nullifiers.iter().skip(1) {
        // Stop once we have reached num_queues
        if visited.len() == num_queues as usize {
            break;
        }
        // Only insert if this queue_index hasn't been added yet
        if visited.iter().all(|&(_, q)| q != nf.queue_index) {
            visited.push((nf.tree_index, nf.queue_index));
        }
    }

    let mut inserted_nullifiers = 0;

    // 2. For each unique queue_index, get the corresponding accounts and process nullifiers.
    for &(tree_index, queue_index) in &visited {
        // Lookup the queue and tree accounts
        let (queue_account, merkle_tree_account) =
            get_queue_and_tree_accounts(accounts, queue_index as usize, tree_index as usize)?;

        // Dispatch to v1 / v2 / ... based on the account type
        match queue_account {
            AcpAccount::OutputQueue(queue) => {
                inserted_nullifiers += batched_nullifiers(
                    merkle_tree_account,
                    queue,
                    &tx_hash,
                    nullifiers,
                    queue_index,
                    tree_index,
                    current_slot,
                )?;
                anchor_lang::Result::Ok(())
            }
            AcpAccount::V1Queue(queue_account_info) => {
                inserted_nullifiers += process_nullifiers_v1(
                    merkle_tree_account,
                    queue_account_info,
                    nullifiers,
                    queue_index,
                    tree_index,
                )?;
                Ok(())
            }
            AcpAccount::BatchedStateTree(_) => {
                msg!("BatchedStateTree, expected output queue.");
                Err(AccountCompressionErrorCode::InvalidAccount.into())
            }
            AcpAccount::StateTree(_) => {
                msg!("StateTree, expected v1 nullifier queue.");
                Err(AccountCompressionErrorCode::InvalidAccount.into())
            }
            AcpAccount::BatchedAddressTree(_) => {
                msg!("BatchedAddressTree, expected v1 nullifier or output queue.");
                Err(AccountCompressionErrorCode::InvalidAccount.into())
            }
            _ => Err(AccountCompressionErrorCode::InvalidAccount.into()),
        }?;
    }

    // 3. Verify we inserted a nullifier for all items
    if inserted_nullifiers != nullifiers.len() {
        msg!("inserted_nullifiers {:?}", inserted_nullifiers);
        msg!("nullifiers.len() {:?}", nullifiers.len());
        return err!(AccountCompressionErrorCode::NotAllLeavesProcessed);
    }

    Ok(())
}

/// Steps:
/// 1. filter for nullifiers with the same queue and tree indices
/// 2. unpack tree account, fail if account is not a tree account
/// 3. check queue and tree are associated
/// 4. check for every value whether it exists in the queue and zero it out.
///    If checked fail if the value is not in the queue.
/// 5. Insert the nullifiers into the current input queue batch.
#[inline(always)]
fn batched_nullifiers<'info>(
    merkle_tree: &mut AcpAccount<'_, 'info>,
    output_queue: &mut BatchedQueueAccount<'info>,
    tx_hash: &[u8; 32],
    nullifiers: &[InsertNullifierInput],
    current_queue_index: u8,
    current_tree_index: u8,
    current_slot: &u64,
) -> Result<usize> {
    // 1. filter for nullifiers with the same queue and tree indices
    let nullifiers = nullifiers
        .iter()
        .filter(|x| x.queue_index == current_queue_index && x.tree_index == current_tree_index);
    let merkle_tree = if let AcpAccount::BatchedStateTree(tree) = merkle_tree {
        tree
    } else {
        return err!(AccountCompressionErrorCode::StateMerkleTreeAccountDiscriminatorMismatch);
    };
    // 3. Check queue and Merkle tree are associated.
    output_queue
        .check_is_associated(merkle_tree.pubkey())
        .map_err(ProgramError::from)?;

    let mut num_elements = 0;

    for nullifier in nullifiers {
        num_elements += 1;
        // 4. check for every account whether the value is still in the queue and zero it out.
        //      If checked fail if the value is not in the queue.
        let leaf_index = nullifier.leaf_index.into();
        output_queue
            .prove_inclusion_by_index_and_zero_out_leaf(
                leaf_index,
                &nullifier.account_hash,
                nullifier.prove_by_index(),
            )
            .map_err(ProgramError::from)?;

        // 5. Insert the nullifiers into the current input queue batch.
        merkle_tree
            .insert_nullifier_into_queue(&nullifier.account_hash, leaf_index, tx_hash, current_slot)
            .map_err(ProgramError::from)?;
    }
    Ok(num_elements)
}

/// Steps:
/// 1. filter for nullifiers with the same queue and tree indices
/// 2. unpack tree account, fail if account is not a tree account
/// 3. check queue and tree are associated
/// 4. Insert the nullifiers into the queues hash set.
fn process_nullifiers_v1<'info>(
    merkle_tree: &mut AcpAccount<'_, 'info>,
    nullifier_queue: &mut AccountInfo<'info>,
    nullifiers: &[InsertNullifierInput],
    current_queue_index: u8,
    current_tree_index: u8,
) -> Result<usize> {
    let nullifiers = nullifiers
        .iter()
        .filter(|x| x.queue_index == current_queue_index && x.tree_index == current_tree_index);
    let (merkle_pubkey, merkle_tree) = if let AcpAccount::StateTree(tree) = merkle_tree {
        tree
    } else {
        return err!(AccountCompressionErrorCode::StateMerkleTreeAccountDiscriminatorMismatch);
    };

    {
        let queue_data = nullifier_queue.try_borrow_data()?;
        // Discriminator is already checked in try_from_account_infos.
        let queue = bytemuck::from_bytes::<QueueAccount>(&queue_data[8..QueueAccount::LEN]);
        // 3. Check queue and Merkle tree are associated.
        if queue.metadata.associated_merkle_tree != *merkle_pubkey {
            msg!(
                "Queue account {:?} is not associated with Merkle tree  {:?}",
                nullifier_queue.key(),
                *merkle_pubkey
            );
            return err!(AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated);
        }
    }
    let mut num_elements = 0;
    // 4. Insert the nullifiers into the queues hash set.

    let sequence_number = merkle_tree.sequence_number();
    let mut queue = nullifier_queue.try_borrow_mut_data()?;
    let mut queue = unsafe { queue_from_bytes_zero_copy_mut(&mut queue).unwrap() };
    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_start!("acp_insert_nf_into_queue");
    for nullifier in nullifiers {
        if nullifier.prove_by_index() {
            return Err(AccountCompressionErrorCode::V1AccountMarkedAsProofByIndex.into());
        }
        num_elements += 1;
        let element = BigUint::from_bytes_be(nullifier.account_hash.as_slice());
        queue
            .insert(&element, sequence_number)
            .map_err(ProgramError::from)?;
    }
    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_end!("acp_insert_nf_into_queue");
    Ok(num_elements)
}

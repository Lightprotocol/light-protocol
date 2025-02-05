use anchor_lang::prelude::*;
use light_batched_merkle_tree::queue::BatchedQueueAccount;
use light_utils::instruction::insert_into_queues::InsertNullifierInput;
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
            get_queue_and_tree_accounts(accounts, queue_index as usize, tree_index as usize)
                .unwrap();

        // Dispatch to v1 / v2 / ... based on the account type
        match queue_account {
            AcpAccount::OutputQueue(queue) => {
                inserted_nullifiers += process_nullifier_v2(
                    merkle_tree_account,
                    queue,
                    &tx_hash,
                    nullifiers,
                    queue_index,
                    tree_index,
                )?;
            }
            AcpAccount::V1Queue(queue_account_info) => {
                inserted_nullifiers += process_nullifier_v1(
                    merkle_tree_account,
                    queue_account_info,
                    nullifiers,
                    queue_index,
                    tree_index,
                )?;
            }
            AcpAccount::BatchedStateTree(_) => {
                msg!("BatchedStateTree");
                unimplemented!();
            }
            AcpAccount::StateTree(_) => {
                msg!("StateTree");
                unimplemented!();
            }
            AcpAccount::BatchedAddressTree(_) => {
                msg!("BatchedAddressTree");
                unimplemented!();
            }
            _ => unimplemented!(),
        }
    }

    // 3. Verify we inserted a nullifier for all items
    if inserted_nullifiers != nullifiers.len() {
        msg!("inserted_nullifiers {:?}", inserted_nullifiers);
        msg!("nullifiers.len() {:?}", nullifiers.len());
        return err!(AccountCompressionErrorCode::NotAllLeavesProcessed);
    }

    Ok(())
}

#[inline(always)]
fn process_nullifier_v2<'info>(
    merkle_tree: &mut AcpAccount<'_, 'info>,
    output_queue: &mut BatchedQueueAccount<'info>,
    tx_hash: &[u8; 32],
    nullifiers: &[InsertNullifierInput],
    current_queue_index: u8,
    current_tree_index: u8,
) -> Result<usize> {
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
            .insert_nullifier_into_current_batch(&nullifier.account_hash, leaf_index, tx_hash)
            .map_err(ProgramError::from)?;
    }
    Ok(num_elements)
}

fn process_nullifier_v1<'info>(
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
        let queue_data = nullifier_queue
            .try_borrow_data()
            .map_err(ProgramError::from)?;
        let queue = bytemuck::from_bytes::<QueueAccount>(&queue_data[8..QueueAccount::LEN]);
        // 3. Check queue and Merkle tree are associated.
        if queue.metadata.associated_merkle_tree != (*merkle_pubkey).into() {
            msg!(
                "Queue account {:?} is not associated with Merkle tree  {:?}",
                nullifier_queue.key(),
                *merkle_pubkey
            );
            return err!(AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated);
        }
    }
    let mut num_elements = 0;
    // 5. Insert the nullifiers into the queues hash set.

    let sequence_number = {
        // let merkle_tree = merkle_tree.try_borrow_data()?;
        // let merkle_tree = state_merkle_tree_from_bytes_zero_copy(&merkle_tree)?;
        merkle_tree.sequence_number()
    };
    let mut queue = nullifier_queue.try_borrow_mut_data()?;
    let mut queue = unsafe { queue_from_bytes_zero_copy_mut(&mut queue).unwrap() };
    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_end!("acp_prep_insertion");
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

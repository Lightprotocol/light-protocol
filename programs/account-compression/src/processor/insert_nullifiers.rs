use anchor_lang::prelude::*;
use light_batched_merkle_tree::queue::BatchedQueueAccount;
use num_bigint::BigUint;

use zerocopy::{little_endian::U32, FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use crate::{
    context::AcpAccount, errors::AccountCompressionErrorCode,
    insert_into_queues::get_queue_and_tree_accounts, queue_from_bytes_zero_copy_mut, QueueAccount,
};

#[repr(C)]
#[derive(
    KnownLayout, IntoBytes, Immutable, Copy, Clone, FromBytes, PartialEq, Debug, Unaligned,
)]
pub struct InsertNullifierInput {
    pub account_hash: [u8; 32],
    pub leaf_index: U32,
    pub prove_by_index: u8,
    pub tree_index: u8,
    pub queue_index: u8,
}

#[inline(always)]
pub fn insert_nullifiers<'a, 'info>(
    num_queues: u8,
    tx_hash: [u8; 32],
    nullifiers: &[InsertNullifierInput],
    accounts: &mut [AcpAccount<'a, 'info>],
) -> Result<()> {
    if nullifiers.is_empty() {
        return Ok(());
    }
    let mut inserted_nullifiers = 0;
    let mut current_tree_index = nullifiers[0].tree_index;
    let mut current_queue_index = nullifiers[0].queue_index;
    let mut dedup_vec = Vec::with_capacity(num_queues as usize);
    for _ in 0..num_queues {
        let (queue_account, merkle_tree_account) = get_queue_and_tree_accounts(
            accounts,
            current_queue_index as usize,
            current_tree_index as usize,
        )
        .unwrap();

        match queue_account {
            AcpAccount::OutputQueue(queue) => {
                inserted_nullifiers += process_nullifier_v2(
                    merkle_tree_account,
                    queue,
                    &tx_hash,
                    nullifiers,
                    current_queue_index,
                )?;
            }
            AcpAccount::V1Queue(queue_account_info) => {
                inserted_nullifiers += process_nullifier_v1(
                    merkle_tree_account,
                    queue_account_info,
                    nullifiers,
                    current_queue_index,
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
            _ => {
                unimplemented!()
            }
        }

        dedup_vec.push(current_queue_index);
        if dedup_vec.len() == num_queues as usize {
            break;
        }
        // find next tree index which doesn't exist in dedup vec yet
        let input = nullifiers
            .iter()
            .find(|x| {
                !dedup_vec
                    .iter()
                    .any(|&queue_index| queue_index == x.queue_index)
            })
            .unwrap();
        current_tree_index = input.tree_index;
        current_queue_index = input.queue_index;
    }
    if inserted_nullifiers != nullifiers.len() {
        msg!("inserted_nullifiers {:?}", inserted_nullifiers);
        msg!("nullifiers.len() {:?}", nullifiers.len());
        return err!(AccountCompressionErrorCode::NotAllLeavesProcessed);
    }
    Ok(())
}

#[inline(always)]
fn process_nullifier_v2<'a, 'info>(
    merkle_tree: &mut AcpAccount<'a, 'info>,
    output_queue: &mut BatchedQueueAccount<'info>,
    tx_hash: &[u8; 32],
    nullifiers: &[InsertNullifierInput],
    current_queue_index: u8,
) -> Result<usize> {
    let nullifiers = nullifiers
        .iter()
        .filter(|x| x.queue_index == current_queue_index);
    let merkle_tree = if let AcpAccount::BatchedStateTree(tree) = merkle_tree {
        tree
    } else {
        panic!("Invalid account");
    };
    // 3. Check queue and Merkle tree are associated.
    output_queue
        .check_is_associated(merkle_tree.pubkey())
        .map_err(ProgramError::from)?;

    let mut num_elements = 0;

    for nullifier in nullifiers {
        num_elements += 1;
        #[cfg(feature = "bench-sbf")]
        light_heap::bench_sbf_start!("acp_insert_nf_into_queue_v2");
        // 4. check for every account whether the value is still in the queue and zero it out.
        //      If checked fail if the value is not in the queue.
        let proof_index = if nullifier.prove_by_index == 1 {
            true
        } else if nullifier.prove_by_index == 0 {
            false
        } else {
            panic!("invalid value");
        };
        let leaf_index = nullifier.leaf_index.into();
        output_queue
            .prove_inclusion_by_index_and_zero_out_leaf(
                leaf_index,
                &nullifier.account_hash,
                proof_index,
            )
            .map_err(ProgramError::from)?;

        // 5. Insert the nullifiers into the current input queue batch.
        merkle_tree
            .insert_nullifier_into_current_batch(&nullifier.account_hash, leaf_index, &tx_hash)
            .map_err(ProgramError::from)?;
        #[cfg(feature = "bench-sbf")]
        light_heap::bench_sbf_end!("acp_insert_nf_into_queue_v2");
    }
    Ok(num_elements)
}

fn process_nullifier_v1<'a, 'info>(
    merkle_tree: &mut AcpAccount<'a, 'info>,
    nullifier_queue: &mut AccountInfo<'info>,
    nullifiers: &[InsertNullifierInput],
    current_queue_index: u8,
) -> Result<usize> {
    let nullifiers = nullifiers
        .iter()
        .filter(|x| x.queue_index == current_queue_index);
    let (merkle_pubkey, merkle_tree) = if let AcpAccount::StateTree(tree) = merkle_tree {
        tree
    } else {
        return err!(AccountCompressionErrorCode::InvalidAccount);
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
        if nullifier.prove_by_index == 1 {
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

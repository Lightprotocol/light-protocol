use anchor_lang::prelude::*;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_compressed_account::instruction_data::insert_into_queues::InsertAddressInput;
use num_bigint::BigUint;

use crate::{
    context::AcpAccount, errors::AccountCompressionErrorCode,
    insert_into_queues::get_queue_and_tree_accounts, queue_from_bytes_zero_copy_mut, QueueAccount,
};

#[inline(always)]
pub fn insert_addresses(
    num_queues: u8,
    addresses: &[InsertAddressInput],
    accounts: &mut [AcpAccount<'_, '_>],
    current_slot: &u64,
) -> Result<()> {
    if addresses.is_empty() {
        return Ok(());
    }

    let mut inserted_addresses = 0;
    // 1. Gather unique (tree_index, queue_index) pairs in the order they appear,
    //    capped at `num_queues`.
    let mut visited = Vec::with_capacity(num_queues as usize);
    // Always push the first one
    visited.push((addresses[0].tree_index, addresses[0].queue_index));

    for nf in addresses.iter().skip(1) {
        // Stop once we have reached num_queues
        if visited.len() == num_queues as usize {
            break;
        }
        // Only insert if this queue_index hasn't been added yet
        if visited.iter().all(|&(_, q)| q != nf.queue_index) {
            visited.push((nf.tree_index, nf.queue_index));
        }
    }

    for &(tree_index, queue_index) in &visited {
        let queue_account = &mut accounts[queue_index as usize];

        match queue_account {
            AcpAccount::BatchedAddressTree(address_tree) => {
                inserted_addresses +=
                    batched_addresses(address_tree, addresses, queue_index, current_slot)?;
                anchor_lang::Result::Ok(())
            }
            AcpAccount::V1Queue(_) => {
                let (queue_account, merkle_tree_account) = get_queue_and_tree_accounts(
                    accounts,
                    queue_index as usize,
                    tree_index as usize,
                )?;
                let queue_account_info =
                    if let AcpAccount::V1Queue(queue_account_info) = queue_account {
                        queue_account_info
                    } else {
                        msg!("Queue account is not a queue account");
                        return err!(AccountCompressionErrorCode::InvalidAccount);
                    };
                inserted_addresses += process_address_v1(
                    merkle_tree_account,
                    queue_account_info,
                    addresses,
                    queue_index,
                    tree_index,
                )?;
                Ok(())
            }
            AcpAccount::AddressTree(_) => unimplemented!("AddressTree"),
            _ => Err(
                AccountCompressionErrorCode::AddressMerkleTreeAccountDiscriminatorMismatch.into(),
            ),
        }?;
    }
    if inserted_addresses != addresses.len() {
        msg!("inserted_addresses {:?}", inserted_addresses);
        msg!("addresses.len() {:?}", addresses.len());
        return err!(AccountCompressionErrorCode::NotAllLeavesProcessed);
    }
    Ok(())
}

/// Insert a batch of addresses into the address queue.
/// 1. Filter for addresses with the same queue indices.
///    (Tree index is unused for batched address trees.)
/// 2. Insert the addresses into the address queue.
fn batched_addresses(
    addresse_tree: &mut BatchedMerkleTreeAccount<'_>,
    addresses: &[InsertAddressInput],
    current_queue_index: u8,
    current_slot: &u64,
) -> Result<usize> {
    // 1. Filter for addresses with the same queue indices.
    let addresses = addresses
        .iter()
        .filter(|x| x.queue_index == current_queue_index);
    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_start!("acp_insert_address_into_queue_v2");
    let mut num_elements = 0;
    // 2.  Insert the addresses into the address queue.
    for address in addresses {
        num_elements += 1;
        addresse_tree
            .insert_address_into_queue(&address.address, current_slot)
            .map_err(ProgramError::from)?;
        #[cfg(feature = "bench-sbf")]
        light_heap::bench_sbf_end!("acp_insert_address_into_queue_v2");
    }
    Ok(num_elements)
}

/// 1. Filter for addresses with the same queue and tree indices.
/// 2. Unpack tree account, fail if account is not a tree account.
/// 3. Check queue and Merkle tree are associated.
/// 4. Insert the addresses into the queues hash set.
fn process_address_v1<'info>(
    merkle_tree: &mut AcpAccount<'_, 'info>,
    address_queue: &mut AccountInfo<'info>,
    addresses: &[InsertAddressInput],
    current_queue_index: u8,
    current_tree_index: u8,
) -> Result<usize> {
    // 1. Filter for addresses with the same queue and tree indices.
    let addresses = addresses
        .iter()
        .filter(|x| x.queue_index == current_queue_index && x.tree_index == current_tree_index);
    // 2. Unpack tree account, fail if account is not a tree account.
    let (merkle_pubkey, merkle_tree) = if let AcpAccount::AddressTree(tree) = merkle_tree {
        tree
    } else {
        return err!(AccountCompressionErrorCode::AddressMerkleTreeAccountDiscriminatorMismatch);
    };
    {
        let queue_data = address_queue.try_borrow_data()?;
        let queue = bytemuck::from_bytes::<QueueAccount>(&queue_data[8..QueueAccount::LEN]);
        // 3. Check queue and Merkle tree are associated.
        if queue.metadata.associated_merkle_tree != *merkle_pubkey {
            msg!(
                "Queue account {:?} is not associated with Merkle tree  {:?}",
                address_queue.key(),
                *merkle_pubkey
            );
            return err!(AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated);
        }
    }
    let mut num_elements = 0;

    // 4. Insert the addresses into the queues hash set.
    let sequence_number = merkle_tree.sequence_number();
    let mut queue = address_queue.try_borrow_mut_data()?;
    let mut queue =
        unsafe { queue_from_bytes_zero_copy_mut(&mut queue).map_err(ProgramError::from)? };
    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_start!("acp_insert_nf_into_queue");
    for address in addresses {
        num_elements += 1;
        let element = BigUint::from_bytes_be(address.address.as_slice());
        queue
            .insert(&element, sequence_number)
            .map_err(ProgramError::from)?;
    }
    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_end!("acp_insert_nf_into_queue");

    Ok(num_elements)
}

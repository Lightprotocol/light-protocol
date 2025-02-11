use anchor_lang::prelude::*;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_compressed_account::insert_into_queues::InsertAddressInput;
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
    let mut current_tree_index = addresses[0].tree_index;
    let mut current_queue_index = addresses[0].queue_index;
    msg!("current_tree_index {:?}", current_tree_index);
    msg!("current_queue_index {:?}", current_queue_index);
    msg!(" num queues {:?}", num_queues);
    let mut dedup_vec = Vec::with_capacity(num_queues as usize);
    for _ in 0..num_queues {
        let queue_account = &mut accounts[current_queue_index as usize];

        match queue_account {
            AcpAccount::BatchedAddressTree(address_tree) => {
                inserted_addresses +=
                    process_address_v2(address_tree, addresses, current_queue_index, current_slot)?;
            }
            AcpAccount::V1Queue(_) => {
                let (queue_account, merkle_tree_account) = get_queue_and_tree_accounts(
                    accounts,
                    current_queue_index as usize,
                    current_tree_index as usize,
                )
                .unwrap();
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
                    current_queue_index,
                )?;
            }
            AcpAccount::AddressTree(_) => unimplemented!("AddressTree"),
            _ => unimplemented!(),
        }

        dedup_vec.push(current_queue_index);
        if dedup_vec.len() == num_queues as usize {
            break;
        }
        // find next tree index which doesn't exist in dedup vec yet
        let input = addresses.iter().find(|x| {
            !dedup_vec
                .iter()
                .any(|queue_index| *queue_index == x.queue_index)
        });
        if let Some(input) = input {
            current_tree_index = input.tree_index;
            current_queue_index = input.queue_index;
        }
    }
    if inserted_addresses != addresses.len() {
        msg!("inserted_addresses {:?}", inserted_addresses);
        msg!("addresses.len() {:?}", addresses.len());
        return err!(AccountCompressionErrorCode::NotAllLeavesProcessed);
    }
    Ok(())
}

/// Insert a batch of addresses into the address queue.
fn process_address_v2(
    addresse_tree: &mut BatchedMerkleTreeAccount<'_>,
    addresses: &[InsertAddressInput],
    current_queue_index: u8,
    current_slot: &u64,
) -> Result<usize> {
    let addresses = addresses
        .iter()
        .filter(|x| x.queue_index == current_queue_index);
    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_start!("acp_insert_address_into_queue_v2");
    let mut num_elements = 0;
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

fn process_address_v1<'info>(
    merkle_tree: &mut AcpAccount<'_, 'info>,
    address_queue: &mut AccountInfo<'info>,
    addresses: &[InsertAddressInput],
    current_queue_index: u8,
) -> Result<usize> {
    let addresses = addresses
        .iter()
        .filter(|x| x.queue_index == current_queue_index);
    msg!("addresses {:?}", addresses);
    let (merkle_pubkey, merkle_tree) = if let AcpAccount::AddressTree(tree) = merkle_tree {
        tree
    } else {
        return err!(AccountCompressionErrorCode::AddressMerkleTreeAccountDiscriminatorMismatch);
    };
    {
        let queue_data = address_queue
            .try_borrow_data()
            .map_err(ProgramError::from)?;
        let queue = bytemuck::from_bytes::<QueueAccount>(&queue_data[8..QueueAccount::LEN]);
        // 1. Check queue and Merkle tree are associated.
        if queue.metadata.associated_merkle_tree != (*merkle_pubkey).into() {
            msg!(
                "Queue account {:?} is not associated with Merkle tree  {:?}",
                address_queue.key(),
                *merkle_pubkey
            );
            return err!(AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated);
        }
    }
    let mut num_elements = 0;
    // 2. Insert the addresses into the queues hash set.

    let sequence_number = merkle_tree.sequence_number();
    let mut queue = address_queue.try_borrow_mut_data()?;
    let mut queue = unsafe { queue_from_bytes_zero_copy_mut(&mut queue).unwrap() };
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

use anchor_lang::prelude::*;
use light_batched_merkle_tree::{
    merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use light_zero_copy::{errors::ZeroCopyError, slice_mut::ZeroCopySliceMut};
use num_bigint::BigUint;
use std::mem::size_of;
use std::ops::{Deref, DerefMut};
use zerocopy::{little_endian::U32, FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned};

use crate::{
    context::AcpAccount, errors::AccountCompressionErrorCode, queue_from_bytes_zero_copy_mut,
    QueueAccount,
};

use super::AppendLeavesInput;

#[repr(C)]
#[derive(
    FromBytes, IntoBytes, KnownLayout, Immutable, Copy, Clone, PartialEq, Debug, Unaligned,
)]
pub struct AppendNullifyCreateAddressInputsMeta {
    is_invoked_by_program: u8,
    pub bump: u8,
    pub num_queues: u8,
    pub num_unique_appends: u8,
    pub num_address_appends: u8,
    pub tx_hash: [u8; 32],
}

#[derive(Debug)]
pub struct AppendNullifyCreateAddressInputs<'a> {
    meta: Ref<&'a mut [u8], AppendNullifyCreateAddressInputsMeta>,
    pub leaves: ZeroCopySliceMut<'a, u8, AppendLeavesInput, false>,
    pub nullifiers: ZeroCopySliceMut<'a, u8, InsertNullifierInput, false>,
    pub addresses: ZeroCopySliceMut<'a, u8, InsertAddressInput, false>,
    // Don't add sequence numbers we don't want to deserialize these here.
}

impl<'a> AppendNullifyCreateAddressInputs<'a> {
    pub fn is_invoked_by_program(&self) -> bool {
        self.meta.is_invoked_by_program == 1
    }

    pub fn set_invoked_by_program(&mut self, value: bool) {
        self.meta.is_invoked_by_program = value as u8;
    }

    pub fn required_size_for_capacity(
        leaves_capacity: u8,
        nullifiers_capacity: u8,
        addresses_capacity: u8,
    ) -> usize {
        size_of::<AppendNullifyCreateAddressInputsMeta>()
            + ZeroCopySliceMut::<u8, AppendLeavesInput, false>::required_size_for_capacity(
                leaves_capacity,
            )
            + ZeroCopySliceMut::<u8, InsertNullifierInput, false>::required_size_for_capacity(
                nullifiers_capacity,
            )
            + ZeroCopySliceMut::<u8, InsertAddressInput, false>::required_size_for_capacity(
                addresses_capacity,
            )
    }

    pub fn new(
        bytes: &'a mut [u8],
        leaves_capacity: u8,
        nullifiers_capacity: u8,
        addresses_capacity: u8,
    ) -> std::result::Result<Self, ZeroCopyError> {
        let (meta, bytes) = bytes.split_at_mut(size_of::<AppendNullifyCreateAddressInputsMeta>());
        let meta = Ref::<&mut [u8], AppendNullifyCreateAddressInputsMeta>::from_bytes(meta)?;
        let (leaves, bytes) =
            ZeroCopySliceMut::<u8, AppendLeavesInput, false>::new_at(leaves_capacity, bytes)?;
        let (nullifiers, bytes) = ZeroCopySliceMut::<u8, InsertNullifierInput, false>::new_at(
            nullifiers_capacity,
            bytes,
        )?;
        let addresses =
            ZeroCopySliceMut::<u8, InsertAddressInput, false>::new(addresses_capacity, bytes)?;
        Ok(AppendNullifyCreateAddressInputs {
            meta,
            leaves,
            nullifiers,
            addresses,
        })
    }
}

impl Deref for AppendNullifyCreateAddressInputs<'_> {
    type Target = AppendNullifyCreateAddressInputsMeta;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl DerefMut for AppendNullifyCreateAddressInputs<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.meta
    }
}

pub fn deserialize_nullify_append_create_address_inputs<'a>(
    bytes: &'a mut [u8],
) -> std::result::Result<AppendNullifyCreateAddressInputs<'a>, ZeroCopyError> {
    let (metadata, bytes) = bytes.split_at_mut(size_of::<AppendNullifyCreateAddressInputsMeta>());
    let meta = Ref::<&mut [u8], AppendNullifyCreateAddressInputsMeta>::from_bytes(metadata)?;

    let (leaves, bytes) = ZeroCopySliceMut::<u8, AppendLeavesInput, false>::from_bytes_at(bytes)?;

    let (nullifiers, bytes) =
        ZeroCopySliceMut::<u8, InsertNullifierInput, false>::from_bytes_at(bytes)?;
    let (addresses, _bytes) =
        ZeroCopySliceMut::<u8, InsertAddressInput, false>::from_bytes_at(bytes)?;
    Ok(AppendNullifyCreateAddressInputs {
        meta,
        leaves,
        nullifiers,
        addresses,
    })
}

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
    // (tree_index, rollover_fee) no rollover fee for the input queue all rollover fees are paid in the output queue.
    // rollover_fee_vec: Vec<(u8, u64)>,
) -> Result<()> {
    if nullifiers.is_empty() {
        return Ok(());
    }
    let mut inserted_nullifiers = 0;
    let mut current_tree_index = nullifiers[0].tree_index;
    let mut current_queue_index = nullifiers[0].queue_index;
    let mut dedup_vec = Vec::with_capacity(num_queues as usize);
    // let mut queue_account_info_index = start_account_index;
    for _ in 0..num_queues {
        // TODO: extract into function and test it.
        // let (queue, accounts) = accounts.split_at_mut((current_queue_index + 1) as usize);
        // let (queue_account, merkle_tree_account) = if current_tree_index > current_queue_index {
        //     let merkle_tree =
        //         &mut accounts[(current_tree_index - (current_queue_index + 1)) as usize];
        //     let queue_account = &mut queue[current_queue_index as usize];
        //     (queue_account, merkle_tree)
        // } else {
        //     let (tree, queue) = queue.split_at_mut((current_tree_index + 1) as usize);
        //     let merkle_tree = &mut tree[current_tree_index as usize];
        //     let queue_account =
        //         &mut queue[(current_queue_index - (current_tree_index + 1)) as usize];
        //     (queue_account, merkle_tree)
        // };
        let (queue_account, merkle_tree_account) = get_queue_and_tree_accounts(
            accounts,
            current_queue_index as usize,
            current_tree_index as usize,
        )
        .unwrap();

        match queue_account {
            AcpAccount::OutputQueue(queue) => {
                inserted_nullifiers += refactored_process_nullifier_v2(
                    merkle_tree_account,
                    queue,
                    &tx_hash,
                    nullifiers,
                    current_queue_index,
                )?;
            }
            AcpAccount::V1Queue(queue_account_info) => {
                inserted_nullifiers += refactored_process_nullifier_v1(
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
fn refactored_process_nullifier_v2<'a, 'info>(
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
        light_heap::bench_sbf_end!("acp_insert_nf_into_queue_v2");
    }
    msg!("v2 num_elements {:?}", num_elements);
    Ok(num_elements)
}

fn refactored_process_nullifier_v1<'a, 'info>(
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
        panic!("Invalid account");
    };
    msg!("refactored_process_nullifier_v1");
    {
        let queue_data = nullifier_queue
            .try_borrow_data()
            .map_err(ProgramError::from)?;
        msg!("data len {:?}", queue_data.len());
        msg!("discriminator {:?}", queue_data[0..32].to_vec());
        msg!("queue pubkey {:?}", nullifier_queue.key());
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
    msg!("refactored_process_nullifier_v1 2");

    let sequence_number = {
        // let merkle_tree = merkle_tree.try_borrow_data()?;
        // let merkle_tree = state_merkle_tree_from_bytes_zero_copy(&merkle_tree)?;
        merkle_tree.sequence_number()
    };
    let mut queue = nullifier_queue.try_borrow_mut_data()?;
    let mut queue = unsafe { queue_from_bytes_zero_copy_mut(&mut queue).unwrap() };
    light_heap::bench_sbf_end!("acp_prep_insertion");
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
    light_heap::bench_sbf_end!("acp_insert_nf_into_queue");
    msg!("v1 num_elements {:?}", num_elements);

    Ok(num_elements)
}

fn get_queue_and_tree_accounts<'a, 'b, 'info>(
    accounts: &'b mut [AcpAccount<'a, 'info>],
    queue_index: usize,
    tree_index: usize,
) -> std::result::Result<
    (&'b mut AcpAccount<'a, 'info>, &'b mut AcpAccount<'a, 'info>),
    AccountCompressionErrorCode,
> {
    // if queue_index == tree_index {
    //     return Err(AccountCompressionErrorCode::SameIndex);
    // }
    let (smaller, bigger) = if queue_index < tree_index {
        (queue_index, tree_index)
    } else {
        (tree_index, queue_index)
    };
    // if bigger >= accounts.len() {
    //     return Err(AccountCompressionErrorCode::OutOfBounds);
    // }
    let (left, right) = accounts.split_at_mut(bigger);
    let smaller_ref = &mut left[smaller];
    let bigger_ref = &mut right[0];
    Ok(if queue_index < tree_index {
        (smaller_ref, bigger_ref)
    } else {
        (bigger_ref, smaller_ref)
    })
}

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
pub struct InsertAddressInput {
    pub address: [u8; 32],
    pub tree_index: u8,
    pub queue_index: u8,
}

pub trait AccountCompressionProgramAccount {
    fn append(&mut self, batch_size: usize, leaves: &[AppendLeavesInput]) -> Result<u64>;
    fn insert(&mut self) -> Result<()>;
}

impl<'a> AccountCompressionProgramAccount for BatchedQueueAccount<'a> {
    fn append(&mut self, batch_size: usize, leaves: &[AppendLeavesInput]) -> Result<u64> {
        for leaf in leaves {
            self.insert_into_current_batch(&leaf.leaf)
                .map_err(ProgramError::from)?;
        }

        let rollover_fee = self.metadata.rollover_metadata.rollover_fee * batch_size as u64;
        Ok(rollover_fee)
    }

    fn insert(&mut self) -> Result<()> {
        unimplemented!("Batched queue accounts only append.")
    }
}

#[inline(always)]
pub fn insert_addresses<'a, 'info>(
    num_queues: u8,
    addresses: &[InsertAddressInput],
    accounts: &mut [AcpAccount<'a, 'info>],
    // (tree_index, rollover_fee) no rollover fee for the input queue all rollover fees are paid in the output queue.
    // rollover_fee_vec: Vec<(u8, u64)>,
) -> Result<()> {
    if addresses.is_empty() {
        return Ok(());
    }
    msg!("num address queues {:?}", num_queues);

    let mut inserted_nullifiers = 0;
    let mut current_tree_index = addresses[0].tree_index;
    let mut current_queue_index = addresses[0].queue_index;
    let mut dedup_vec = Vec::with_capacity(num_queues as usize);
    // let mut queue_account_info_index = start_account_index;
    for _ in 0..num_queues {
        // TODO: extract into function and test it.
        // let (queue, accounts) = accounts.split_at_mut((current_queue_index + 1) as usize);
        // let (queue_account, merkle_tree_account) = if current_tree_index > current_queue_index {
        //     let merkle_tree =
        //         &mut accounts[(current_tree_index - (current_queue_index + 1)) as usize];
        //     let queue_account = &mut queue[current_queue_index as usize];
        //     (queue_account, merkle_tree)
        // } else {
        //     let (tree, queue) = queue.split_at_mut((current_tree_index + 1) as usize);
        //     let merkle_tree = &mut tree[current_tree_index as usize];
        //     let queue_account =
        //         &mut queue[(current_queue_index - (current_tree_index + 1)) as usize];
        //     (queue_account, merkle_tree)
        // };
        let (queue_account, merkle_tree_account) = get_queue_and_tree_accounts(
            accounts,
            current_queue_index as usize,
            current_tree_index as usize,
        )
        .unwrap();

        match queue_account {
            AcpAccount::BatchedAddressTree(address_tree) => {
                inserted_nullifiers +=
                    process_address_v2(address_tree, addresses, current_queue_index)?;
            }
            AcpAccount::V1Queue(queue_account_info) => {
                inserted_nullifiers += refactored_process_address_v1(
                    merkle_tree_account,
                    queue_account_info,
                    addresses,
                    current_queue_index,
                )?;
            }
            _ => unimplemented!(),
        }

        dedup_vec.push(current_queue_index);
        if dedup_vec.len() == num_queues as usize {
            break;
        }
        // find next tree index which doesn't exist in dedup vec yet
        let input = addresses
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
    if inserted_nullifiers != addresses.len() {
        msg!("inserted_nullifiers {:?}", inserted_nullifiers);
        msg!("nullifiers.len() {:?}", addresses.len());
        return err!(AccountCompressionErrorCode::NotAllLeavesProcessed);
    }
    Ok(())
}

/// Insert a batch of addresses into the address queue.
/// 1. Check discriminator and account ownership.
/// 2. Check that the signer is the authority or registered program.
/// 3. Insert the addresses into the current batch.
/// 4. Return rollover fee.
fn process_address_v2<'info>(
    addresse_tree: &mut BatchedMerkleTreeAccount<'info>,
    addresses: &[InsertAddressInput],
    current_queue_index: u8,
) -> Result<usize> {
    let addresses = addresses
        .iter()
        .filter(|x| x.queue_index == current_queue_index);
    // 3. Insert the addresses into the current batch.
    // for element in queue_bundle.elements.iter() {
    light_heap::bench_sbf_start!("acp_insert_nf_into_queue_v2");
    let mut num_elements = 0;
    for address in addresses {
        num_elements += 1;
        addresse_tree
            .insert_address_into_current_batch(&address.address)
            .map_err(ProgramError::from)?;
        light_heap::bench_sbf_end!("acp_insert_nf_into_queue_v2");
    }
    Ok(num_elements)
}

fn refactored_process_address_v1<'a, 'info>(
    merkle_tree: &mut AcpAccount<'a, 'info>,
    nullifier_queue: &mut AccountInfo<'info>,
    addresses: &[InsertAddressInput],
    current_queue_index: u8,
) -> Result<usize> {
    let addresses = addresses
        .iter()
        .filter(|x| x.queue_index == current_queue_index);
    let (merkle_pubkey, merkle_tree) = if let AcpAccount::AddressTree(tree) = merkle_tree {
        tree
    } else {
        panic!("Invalid account");
    };
    msg!("refactored_process_nullifier_v1");
    {
        let queue_data = nullifier_queue
            .try_borrow_data()
            .map_err(ProgramError::from)?;
        msg!("data len {:?}", queue_data.len());
        msg!("discriminator {:?}", queue_data[0..32].to_vec());
        msg!("queue pubkey {:?}", nullifier_queue.key());
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
    // 5. Insert the addresses into the queues hash set.
    msg!("refactored_process_nullifier_v1 2");

    let sequence_number = merkle_tree.sequence_number();
    let mut queue = nullifier_queue.try_borrow_mut_data()?;
    let mut queue = unsafe { queue_from_bytes_zero_copy_mut(&mut queue).unwrap() };
    light_heap::bench_sbf_end!("acp_prep_insertion");
    light_heap::bench_sbf_start!("acp_insert_nf_into_queue");
    for address in addresses {
        num_elements += 1;
        let element = BigUint::from_bytes_be(address.address.as_slice());
        queue
            .insert(&element, sequence_number)
            .map_err(ProgramError::from)?;
    }
    light_heap::bench_sbf_end!("acp_insert_nf_into_queue");
    msg!("v1 num_elements {:?}", num_elements);

    Ok(num_elements)
}

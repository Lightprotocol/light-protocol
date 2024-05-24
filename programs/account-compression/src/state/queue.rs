use crate::InsertIntoQueues;
use crate::{errors::AccountCompressionErrorCode, AccessMetadata, RolloverMetadata};
use crate::{
    utils::check_registered_or_signer::{GroupAccess, GroupAccounts},
    RegisteredProgram,
};
use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use light_hash_set::{zero_copy::HashSetZeroCopy, HashSet};
use std::{cell::RefMut, mem};

#[account(zero_copy)]
#[derive(AnchorDeserialize, AnchorSerialize, Debug, PartialEq)]
pub struct QueueMetadata {
    pub access_metadata: AccessMetadata,
    pub rollover_metadata: RolloverMetadata,

    // Queue associated with this Merkle tree.
    pub associated_merkle_tree: Pubkey,
    // Next queue to be used after rollover.
    pub next_queue: Pubkey,
    pub queue_type: u64,
}

#[derive(AnchorDeserialize, AnchorSerialize, Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum QueueType {
    NullifierQueue = 1, // Explicitly assign values to the enum variants
    AddressQueue = 2,
}

pub fn check_queue_type(queue_type: &u64, expected_queue_type: &QueueType) -> Result<()> {
    if *queue_type != (*expected_queue_type) as u64 {
        err!(AccountCompressionErrorCode::InvalidQueueType)
    } else {
        Ok(())
    }
}

impl QueueMetadata {
    pub fn init(
        &mut self,
        access_metadata: AccessMetadata,
        rollover_metadata: RolloverMetadata,
        associated_merkle_tree: Pubkey,
        queue_type: QueueType,
    ) {
        self.access_metadata = access_metadata;
        self.rollover_metadata = rollover_metadata;
        self.associated_merkle_tree = associated_merkle_tree;
        self.queue_type = queue_type as u64;
    }

    pub fn rollover(
        &mut self,
        old_associated_merkle_tree: Pubkey,
        next_queue: Pubkey,
    ) -> Result<()> {
        if self.associated_merkle_tree != old_associated_merkle_tree {
            return err!(AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated);
        }

        self.rollover_metadata.rollover()?;
        self.next_queue = next_queue;

        Ok(())
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Debug, PartialEq)]
#[account(zero_copy)]
#[aligned_sized(anchor)]
pub struct QueueAccount {
    pub metadata: QueueMetadata,
}

impl QueueAccount {
    pub fn init(
        &mut self,
        access_metadata: AccessMetadata,
        rollover_meta_data: RolloverMetadata,
        associated_merkle_tree: Pubkey,
        queue_type: QueueType,
    ) {
        self.metadata.init(
            access_metadata,
            rollover_meta_data,
            associated_merkle_tree,
            queue_type,
        )
    }
}

impl GroupAccess for QueueAccount {
    fn get_owner(&self) -> &Pubkey {
        &self.metadata.access_metadata.owner
    }

    fn get_delegate(&self) -> &Pubkey {
        &self.metadata.access_metadata.delegate
    }
}

impl<'info> GroupAccounts<'info> for InsertIntoQueues<'info> {
    fn get_signing_address(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

impl QueueAccount {
    pub fn size(capacity_indices: usize, capacity_values: usize) -> Result<usize> {
        Ok(8 + mem::size_of::<Self>()
            + HashSet::<u16>::size_in_account(capacity_indices, capacity_values)
                .map_err(ProgramError::from)?)
    }
}

/// Creates a copy of `IndexedArray` from the given account data.
///
/// # Safety
///
/// This operation is unsafe. It's the caller's responsibility to ensure that
/// the provided account data have correct size and alignment.
pub unsafe fn nullifier_queue_from_bytes_copy(
    mut data: RefMut<'_, &mut [u8]>,
) -> Result<HashSet<u16>> {
    let data = &mut data[8 + mem::size_of::<QueueAccount>()..];
    let queue = HashSet::<u16>::from_bytes_copy(data).map_err(ProgramError::from)?;
    Ok(queue)
}

/// Casts the given account data to an `IndexedArrayZeroCopy` instance.
///
/// # Safety
///
/// This operation is unsafe. It's the caller's responsibility to ensure that
/// the provided account data have correct size and alignment.
pub unsafe fn queue_from_bytes_zero_copy_mut(data: &mut [u8]) -> Result<HashSetZeroCopy<u16>> {
    let data = &mut data[8 + mem::size_of::<QueueAccount>()..];
    let queue =
        HashSetZeroCopy::<u16>::from_bytes_zero_copy_mut(data).map_err(ProgramError::from)?;
    Ok(queue)
}
/// Casts the given account data to an `IndexedArrayZeroCopy` instance.
///
/// # Safety
///
/// This operation is unsafe. It's the caller's responsibility to ensure that
/// the provided account data have correct size and alignment.
pub unsafe fn queue_from_bytes_zero_copy_init(
    data: &mut [u8],
    capacity_indices: usize,
    capacity_values: usize,
    sequence_threshold: usize,
) -> Result<HashSetZeroCopy<u16>> {
    let data = &mut data[8 + mem::size_of::<QueueAccount>()..];
    let queue = HashSetZeroCopy::<u16>::from_bytes_zero_copy_init(
        data,
        capacity_indices,
        capacity_values,
        sequence_threshold,
    )
    .map_err(ProgramError::from)?;
    Ok(queue)
}

use std::mem;

use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use light_hash_set::{zero_copy::HashSetZeroCopy, HashSet};
use light_merkle_tree_metadata::{
    access::AccessMetadata, queue::QueueMetadata, rollover::RolloverMetadata, QueueType,
};

use crate::utils::check_signer_is_registered_or_authority::GroupAccess;

#[account(zero_copy)]
#[derive(AnchorDeserialize, Debug, PartialEq)]
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
            associated_merkle_tree.into(),
            queue_type,
        )
    }
}

impl GroupAccess for QueueAccount {
    fn get_owner(&self) -> Pubkey {
        self.metadata.access_metadata.owner.into()
    }

    fn get_program_owner(&self) -> Pubkey {
        self.metadata
            .access_metadata
            .program_owner
            .to_bytes()
            .into()
    }
}

impl QueueAccount {
    pub fn size(capacity: usize) -> Result<usize> {
        Ok(8 + mem::size_of::<Self>() + HashSet::size_in_account(capacity))
    }
}

/// Creates a copy of `HashSet` from the given account data.
///
/// # Safety
///
/// This operation is unsafe. It's the caller's responsibility to ensure that
/// the provided account data have correct size and alignment.
pub unsafe fn queue_from_bytes_copy(data: &mut [u8]) -> Result<HashSet> {
    let data = &mut data[8 + mem::size_of::<QueueAccount>()..];
    let queue = HashSet::from_bytes_copy(data).map_err(ProgramError::from)?;
    Ok(queue)
}

/// Casts the given account data to an `HashSetZeroCopy` instance.
///
/// # Safety
///
/// This operation is unsafe. It's the caller's responsibility to ensure that
/// the provided account data have correct size and alignment.
pub unsafe fn queue_from_bytes_zero_copy_mut(data: &mut [u8]) -> Result<HashSetZeroCopy<'_>> {
    let data = &mut data[8 + mem::size_of::<QueueAccount>()..];
    let queue = HashSetZeroCopy::from_bytes_zero_copy_mut(data).map_err(ProgramError::from)?;
    Ok(queue)
}

/// Casts the given account data to an `HashSetZeroCopy` instance.
///
/// # Safety
///
/// This operation is unsafe. It's the caller's responsibility to ensure that
/// the provided account data have correct size and alignment.
pub unsafe fn queue_from_bytes_zero_copy_init(
    data: &mut [u8],
    capacity: usize,
    sequence_threshold: usize,
) -> Result<HashSetZeroCopy<'_>> {
    let data = &mut data[8 + mem::size_of::<QueueAccount>()..];
    let queue = HashSetZeroCopy::from_bytes_zero_copy_init(data, capacity, sequence_threshold)
        .map_err(ProgramError::from)?;
    Ok(queue)
}

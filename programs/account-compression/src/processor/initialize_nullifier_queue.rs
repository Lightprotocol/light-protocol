use std::{cell::RefMut, mem};

use aligned_sized::aligned_sized;
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_hash_set::{zero_copy::HashSetZeroCopy, HashSet};

use crate::{
    utils::check_registered_or_signer::{GroupAccess, GroupAccounts},
    QueueMetadata, RegisteredProgram,
};
use crate::{AccessMetadata, InsertIntoNullifierQueues, RolloverMetadata};

pub fn process_initialize_nullifier_queue<'a, 'b, 'c: 'info, 'info>(
    nullifier_queue_account_info: AccountInfo<'info>,
    nullifier_queue_account_loader: &'a AccountLoader<'info, NullifierQueueAccount>,
    index: u64,
    owner: Pubkey,
    delegate: Option<Pubkey>,
    associated_merkle_tree: Pubkey,
    capacity_indices: u16,
    capacity_values: u16,
    sequence_threshold: u64,
    rollover_threshold: Option<u64>,
    close_threshold: Option<u64>,
    network_fee: u64,
) -> Result<()> {
    {
        let mut nullifier_queue = nullifier_queue_account_loader.load_init()?;
        let rollover_meta_data = RolloverMetadata {
            index,
            rollover_threshold: rollover_threshold.unwrap_or_default(),
            close_threshold: close_threshold.unwrap_or(u64::MAX),
            rolledover_slot: u64::MAX,
            network_fee,
            rollover_fee: 0,
        };

        nullifier_queue.init(
            AccessMetadata {
                owner,
                delegate: delegate.unwrap_or_default(),
            },
            rollover_meta_data,
            associated_merkle_tree,
        );

        drop(nullifier_queue);
    }

    let nullifier_queue = nullifier_queue_account_info;
    let mut nullifier_queue = nullifier_queue.try_borrow_mut_data()?;
    let _ = unsafe {
        nullifier_queue_from_bytes_zero_copy_init(
            &mut nullifier_queue,
            capacity_indices.into(),
            capacity_values.into(),
            sequence_threshold as usize,
        )
        .unwrap()
    };
    Ok(())
}

#[derive(AnchorDeserialize, AnchorSerialize, Debug, PartialEq)]
#[account(zero_copy)]
#[aligned_sized(anchor)]
pub struct NullifierQueueAccount {
    pub metadata: QueueMetadata,
}

impl NullifierQueueAccount {
    pub fn init(
        &mut self,
        access_metadata: AccessMetadata,
        rollover_meta_data: RolloverMetadata,
        associated_merkle_tree: Pubkey,
    ) {
        self.metadata
            .init(access_metadata, rollover_meta_data, associated_merkle_tree)
    }
}

impl GroupAccess for NullifierQueueAccount {
    fn get_owner(&self) -> &Pubkey {
        &self.metadata.access_metadata.owner
    }

    fn get_delegate(&self) -> &Pubkey {
        &self.metadata.access_metadata.delegate
    }
}

impl<'info> GroupAccounts<'info> for InsertIntoNullifierQueues<'info> {
    fn get_signing_address(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

impl NullifierQueueAccount {
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
    let data = &mut data[8 + mem::size_of::<NullifierQueueAccount>()..];
    let queue = HashSet::<u16>::from_bytes_copy(data).map_err(ProgramError::from)?;
    Ok(queue)
}

/// Casts the given account data to an `IndexedArrayZeroCopy` instance.
///
/// # Safety
///
/// This operation is unsafe. It's the caller's responsibility to ensure that
/// the provided account data have correct size and alignment.
pub unsafe fn nullifier_queue_from_bytes_zero_copy_mut(
    data: &mut [u8],
) -> Result<HashSetZeroCopy<u16>> {
    let data = &mut data[8 + mem::size_of::<NullifierQueueAccount>()..];
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
pub unsafe fn nullifier_queue_from_bytes_zero_copy_init(
    data: &mut [u8],
    capacity_indices: usize,
    capacity_values: usize,
    sequence_threshold: usize,
) -> Result<HashSetZeroCopy<u16>> {
    let data = &mut data[8 + mem::size_of::<NullifierQueueAccount>()..];
    let queue = HashSetZeroCopy::<u16>::from_bytes_zero_copy_init(
        data,
        capacity_indices,
        capacity_values,
        sequence_threshold,
    )
    .map_err(ProgramError::from)?;
    Ok(queue)
}

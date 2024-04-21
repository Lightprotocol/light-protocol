use std::{cell::RefMut, mem};

use aligned_sized::aligned_sized;
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_hash_set::{zero_copy::HashSetZeroCopy, HashSet};

use crate::initialize_address_merkle_tree::compute_rollover_fee;
use crate::InsertIntoIndexedArrays;
use crate::{
    utils::check_registered_or_signer::{GroupAccess, GroupAccounts},
    RegisteredProgram,
};

pub fn process_initialize_indexed_array<'a, 'b, 'c: 'info, 'info>(
    indexed_array_account_info: AccountInfo<'info>,
    indexed_array_account_loader: &'a AccountLoader<'info, IndexedArrayAccount>,
    index: u64,
    owner: Pubkey,
    delegate: Option<Pubkey>,
    associated_merkle_tree: Pubkey,
    capacity_indices: u16,
    capacity_values: u16,
    sequence_threshold: u64,
    rollover_threshold: Option<u64>,
    tip: u64,
    height: u32,
) -> Result<()> {
    {
        let mut indexed_array_account = indexed_array_account_loader.load_init()?;
        indexed_array_account.index = index;
        indexed_array_account.owner = owner;
        indexed_array_account.delegate = delegate.unwrap_or(owner);
        indexed_array_account.associated_merkle_tree = associated_merkle_tree;
        indexed_array_account.rolledover_slot = u64::MAX;
        indexed_array_account.tip = tip;
        let queue_rent = indexed_array_account_info.lamports();
        let total_number_of_leaves = 2u64.pow(height);
        let rollover_fee = if let Some(rollover_threshold) = rollover_threshold {
            compute_rollover_fee(rollover_threshold, total_number_of_leaves, queue_rent)?
        } else {
            0
        };
        indexed_array_account.rollover_fee = rollover_fee;
        indexed_array_account.rolledover_slot = u64::MAX;
        drop(indexed_array_account);
    }

    let indexed_array = indexed_array_account_info;
    let mut indexed_array = indexed_array.try_borrow_mut_data()?;
    let _ = unsafe {
        indexed_array_from_bytes_zero_copy_init(
            &mut indexed_array,
            capacity_indices.into(),
            capacity_values.into(),
            sequence_threshold as usize,
        )
        .unwrap()
    };
    Ok(())
}

#[derive(Debug, PartialEq)]
#[account(zero_copy)]
#[aligned_sized(anchor)]
pub struct IndexedArrayAccount {
    pub index: u64,
    pub rollover_fee: u64,
    /// Tip for maintaining the account.
    pub tip: u64,
    /// The slot when the account was rolled over, a rolled over account should not be written to.
    pub rolledover_slot: u64,
    pub owner: Pubkey,
    pub delegate: Pubkey,
    pub associated_merkle_tree: Pubkey,
    pub next_queue: Pubkey,
}

impl GroupAccess for IndexedArrayAccount {
    fn get_owner(&self) -> &Pubkey {
        &self.owner
    }

    fn get_delegate(&self) -> &Pubkey {
        &self.delegate
    }
}

impl<'info> GroupAccounts<'info> for InsertIntoIndexedArrays<'info> {
    fn get_signing_address(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

impl IndexedArrayAccount {
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
pub unsafe fn indexed_array_from_bytes_copy(
    mut data: RefMut<'_, &mut [u8]>,
) -> Result<HashSet<u16>> {
    let data = &mut data[8 + mem::size_of::<IndexedArrayAccount>()..];
    let queue = HashSet::<u16>::from_bytes_copy(data).map_err(ProgramError::from)?;
    Ok(queue)
}

/// Casts the given account data to an `IndexedArrayZeroCopy` instance.
///
/// # Safety
///
/// This operation is unsafe. It's the caller's responsibility to ensure that
/// the provided account data have correct size and alignment.
pub unsafe fn indexed_array_from_bytes_zero_copy_mut(
    data: &mut [u8],
) -> Result<HashSetZeroCopy<u16>> {
    let data = &mut data[8 + mem::size_of::<IndexedArrayAccount>()..];
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
pub unsafe fn indexed_array_from_bytes_zero_copy_init(
    data: &mut [u8],
    capacity_indices: usize,
    capacity_values: usize,
    sequence_threshold: usize,
) -> Result<HashSetZeroCopy<u16>> {
    let data = &mut data[8 + mem::size_of::<IndexedArrayAccount>()..];
    let queue = HashSetZeroCopy::<u16>::from_bytes_zero_copy_init(
        data,
        capacity_indices,
        capacity_values,
        sequence_threshold,
    )
    .map_err(ProgramError::from)?;
    Ok(queue)
}

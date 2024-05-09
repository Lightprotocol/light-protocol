use std::{cell::RefMut, mem};

use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use light_bounded_vec::CyclicBoundedVec;
use light_concurrent_merkle_tree::ConcurrentMerkleTree26;
use light_hash_set::{zero_copy::HashSetZeroCopy, HashSet};
use light_hasher::Poseidon;
use light_indexed_merkle_tree::{
    copy::IndexedMerkleTreeCopy26,
    zero_copy::{IndexedMerkleTreeZeroCopy26, IndexedMerkleTreeZeroCopyMut26},
};

use crate::utils::check_registered_or_signer::GroupAccess;

#[account(zero_copy)]
#[aligned_sized(anchor)]
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct AddressQueueAccount {
    pub index: u64,
    pub rollover_fee: u64,
    pub tip: u64,
    pub rolledover_slot: u64,
    pub owner: Pubkey,
    pub delegate: Pubkey,
    pub associated_merkle_tree: Pubkey,
    pub next_queue: Pubkey,
}

impl GroupAccess for AddressQueueAccount {
    fn get_owner(&self) -> &Pubkey {
        &self.owner
    }

    fn get_delegate(&self) -> &Pubkey {
        &self.delegate
    }
}

impl AddressQueueAccount {
    pub fn size(capacity_indices: usize, capacity_values: usize) -> Result<usize> {
        Ok(8 + mem::size_of::<Self>()
            + HashSet::<u16>::size_in_account(capacity_indices, capacity_values)
                .map_err(ProgramError::from)?)
    }
}

/// Creates a copy of `AddressQueue` from the given account data.
///
/// # Safety
///
/// This operation is unsafe. It's the caller's responsibility to ensure that
/// the provided account data have correct size and alignment.
pub unsafe fn address_queue_from_bytes_copy(
    mut data: RefMut<'_, &mut [u8]>,
) -> Result<HashSet<u16>> {
    let data = &mut data[8 + mem::size_of::<AddressQueueAccount>()..];
    let queue = HashSet::<u16>::from_bytes_copy(data).map_err(ProgramError::from)?;
    Ok(queue)
}

/// Casts the given account data to an `AddressQueueZeroCopy` instance.
///
/// # Safety
///
/// This operation is unsafe. It's the caller's responsibility to ensure that
/// the provided account data have correct size and alignment.
pub unsafe fn address_queue_from_bytes_zero_copy_mut(
    data: &mut [u8],
) -> Result<HashSetZeroCopy<u16>> {
    let data = &mut data[8 + mem::size_of::<AddressQueueAccount>()..];
    let queue =
        HashSetZeroCopy::<u16>::from_bytes_zero_copy_mut(data).map_err(ProgramError::from)?;
    Ok(queue)
}

/// Casts the given account data to an `AddressQueueZeroCopy` instance.
///
/// # Safety
///
/// This operation is unsafe. It's the caller's responsibility to ensure that
/// the provided account data have correct size and alignment.
pub unsafe fn address_queue_from_bytes_zero_copy_init(
    data: &mut [u8],
    capacity_indices: usize,
    capacity_values: usize,
    sequence_threshold: usize,
) -> Result<HashSetZeroCopy<u16>> {
    let data = &mut data[8 + mem::size_of::<AddressQueueAccount>()..];
    let queue = HashSetZeroCopy::<u16>::from_bytes_zero_copy_init(
        data,
        capacity_indices,
        capacity_values,
        sequence_threshold,
    )
    .map_err(ProgramError::from)?;
    Ok(queue)
}

#[account(zero_copy)]
#[aligned_sized(anchor)]
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct AddressMerkleTreeAccount {
    /// Unique index.
    pub index: u64,
    pub rollover_fee: u64,
    pub tip: u64,
    pub rollover_threshold: u64,
    pub rolledover_slot: u64,
    pub close_threshold: u64,
    pub associated_queue: Pubkey,
    /// Public key of the next Merkle tree.
    pub next_merkle_tree: Pubkey,
    /// Owner of the Merkle tree.
    pub owner: Pubkey,
    /// Delegate of the Merkle tree. This will be used for program owned Merkle trees.
    pub delegate: Pubkey,
    pub merkle_tree_struct: [u8; 320],
    pub merkle_tree_filled_subtrees: [u8; 832],
    pub merkle_tree_changelog: [u8; 1220800],
    pub merkle_tree_roots: [u8; 76800],
    pub merkle_tree_canopy: [u8; 65472],
    pub address_changelog: [u8; 20480],
}

impl AddressMerkleTreeAccount {
    pub fn copy_merkle_tree(&self) -> Result<IndexedMerkleTreeCopy26<Poseidon, usize>> {
        let tree = unsafe {
            IndexedMerkleTreeCopy26::copy_from_bytes(
                &self.merkle_tree_struct,
                &self.merkle_tree_filled_subtrees,
                &self.merkle_tree_changelog,
                &self.merkle_tree_roots,
                &self.merkle_tree_canopy,
                &self.address_changelog,
            )
            .map_err(ProgramError::from)?
        };
        Ok(tree)
    }

    pub fn load_merkle_tree(&self) -> Result<IndexedMerkleTreeZeroCopy26<Poseidon, usize>> {
        let tree = unsafe {
            IndexedMerkleTreeZeroCopy26::from_bytes_zero_copy(
                &self.merkle_tree_struct,
                &self.merkle_tree_filled_subtrees,
                &self.merkle_tree_changelog,
                &self.merkle_tree_roots,
                &self.merkle_tree_canopy,
                &self.address_changelog,
            )
            .map_err(ProgramError::from)?
        };
        Ok(tree)
    }

    pub fn load_merkle_tree_init(
        &mut self,
        height: usize,
        changelog_size: usize,
        roots_size: usize,
        canopy_depth: usize,
        address_changelog_size: usize,
    ) -> Result<IndexedMerkleTreeZeroCopyMut26<Poseidon, usize>> {
        let tree = unsafe {
            IndexedMerkleTreeZeroCopyMut26::<Poseidon, usize>::from_bytes_zero_copy_init(
                &mut self.merkle_tree_struct,
                &mut self.merkle_tree_filled_subtrees,
                &mut self.merkle_tree_changelog,
                &mut self.merkle_tree_roots,
                &mut self.merkle_tree_canopy,
                height,
                changelog_size,
                roots_size,
                canopy_depth,
                &mut self.address_changelog,
                address_changelog_size,
            )
            .map_err(ProgramError::from)?
        };
        tree.merkle_tree.init().map_err(ProgramError::from)?;
        Ok(tree)
    }

    pub fn load_merkle_tree_mut(
        &mut self,
    ) -> Result<IndexedMerkleTreeZeroCopyMut26<Poseidon, usize>> {
        let tree = unsafe {
            IndexedMerkleTreeZeroCopyMut26::from_bytes_zero_copy_mut(
                &mut self.merkle_tree_struct,
                &mut self.merkle_tree_filled_subtrees,
                &mut self.merkle_tree_changelog,
                &mut self.merkle_tree_roots,
                &mut self.merkle_tree_canopy,
                &mut self.address_changelog,
            )
            .map_err(ProgramError::from)?
        };
        Ok(tree)
    }

    pub fn load_roots(&self) -> Result<CyclicBoundedVec<[u8; 32]>> {
        let tree = self.load_merkle_tree()?;
        let roots = unsafe {
            ConcurrentMerkleTree26::<Poseidon>::roots_from_bytes(
                &self.merkle_tree_roots,
                tree.merkle_tree.merkle_tree.roots.len(),
                tree.merkle_tree.merkle_tree.roots.capacity(),
                tree.merkle_tree.merkle_tree.roots.first_index(),
                tree.merkle_tree.merkle_tree.roots.last_index(),
            )
            .map_err(ProgramError::from)?
        };
        Ok(roots)
    }
}

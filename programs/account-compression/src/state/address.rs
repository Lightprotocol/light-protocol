use std::{cell::RefMut, mem};

use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use ark_ff::BigInteger256;
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::Poseidon;
use light_indexed_merkle_tree::IndexedMerkleTree22;

#[account(zero_copy)]
#[aligned_sized(anchor)]
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct AddressQueueAccount {
    pub queue: [u8; 112008],
}

pub type AddressMerkleTree<'a> = IndexedMerkleTree22<'a, Poseidon, usize, BigInteger256>;

#[account(zero_copy)]
#[aligned_sized(anchor)]
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct AddressMerkleTreeAccount {
    /// Unique index.
    pub index: u64,
    /// Public key of the next Merkle tree.
    pub next_merkle_tree: Pubkey,
    /// Owner of the Merkle tree.
    pub owner: Pubkey,
    /// Delegate of the Merkle tree. This will be used for program owned Merkle trees.
    pub delegate: Pubkey,
}

pub unsafe fn address_mt_from_bytes_copy<'a>(
    data: RefMut<'_, &'a mut [u8]>,
) -> Result<AddressMerkleTree<'a>> {
    let data = &data[8 + mem::size_of::<AddressMerkleTreeAccount>()..];
    let tree = AddressMerkleTree::from_bytes_copy(data).map_err(ProgramError::from)?;
    Ok(tree)
}

pub fn address_mt_from_bytes_zero_copy<'a>(
    data: RefMut<'_, &'a mut [u8]>,
) -> Result<AddressMerkleTree<'a>> {
    let data = &data[8 + mem::size_of::<AddressMerkleTreeAccount>()..];
    let tree =
        unsafe { AddressMerkleTree::from_bytes_zero_copy(data).map_err(ProgramError::from)? };
    Ok(tree)
}

pub fn address_mt_from_bytes_zero_copy_mut<'a>(
    data: RefMut<'_, &'a mut [u8]>,
) -> Result<AddressMerkleTree<'a>> {
    let data = &data[8 + mem::size_of::<AddressMerkleTreeAccount>()..];
    let tree =
        unsafe { AddressMerkleTree::from_bytes_zero_copy_mut(data).map_err(ProgramError::from)? };
    Ok(tree)
}

pub fn address_mt_from_bytes_zero_copy_init<'info>(
    data: RefMut<'_, &'a mut [u8]>,
    height: usize,
    changelog_size: usize,
    roots_size: usize,
    canopy_depth: usize,
) -> Result<AddressMerkleTree<'info>> {
    let data = &data[8 + mem::size_of::<AddressMerkleTreeAccount>()..];
    let tree = unsafe {
        AddressMerkleTree::from_bytes_zero_copy_init(
            data,
            height,
            changelog_size,
            roots_size,
            canopy_depth,
        )
        .map_err(ProgramError::from)?
    };
    Ok(tree)
}

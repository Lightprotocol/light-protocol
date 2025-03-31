use std::mem;

use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use light_hasher::Poseidon;
use light_indexed_merkle_tree::{
    zero_copy::{IndexedMerkleTreeZeroCopy, IndexedMerkleTreeZeroCopyMut},
    IndexedMerkleTree,
};
use light_merkle_tree_metadata::{
    access::AccessMetadata, merkle_tree::MerkleTreeMetadata, rollover::RolloverMetadata,
};
use pinocchio::program_error::ProgramError;

use crate::Result;

#[repr(C)]
#[aligned_sized(anchor)]
#[derive(Pod, Debug, Default, Zeroable, Clone, Copy)]
pub struct AddressMerkleTreeAccount {
    pub metadata: MerkleTreeMetadata,
}

impl AddressMerkleTreeAccount {
    pub fn size(
        height: usize,
        changelog_size: usize,
        roots_size: usize,
        canopy_depth: usize,
        indexed_changelog_size: usize,
    ) -> usize {
        8 + mem::size_of::<Self>()
            + IndexedMerkleTree::<Poseidon, usize, 26, 16>::size_in_account(
                height,
                changelog_size,
                roots_size,
                canopy_depth,
                indexed_changelog_size,
            )
    }

    pub fn init(
        &mut self,
        access_metadata: AccessMetadata,
        rollover_metadata: RolloverMetadata,
        associated_queue: Pubkey,
    ) {
        self.metadata.init(
            access_metadata,
            rollover_metadata,
            light_compressed_account::pubkey::Pubkey::new_from_array(associated_queue.to_bytes()),
        )
    }
}

pub fn address_merkle_tree_from_bytes_zero_copy(
    data: &[u8],
) -> Result<IndexedMerkleTreeZeroCopy<Poseidon, usize, 26, 16>> {
    let data = &data[8 + mem::size_of::<AddressMerkleTreeAccount>()..];
    let merkle_tree = IndexedMerkleTreeZeroCopy::from_bytes_zero_copy(data).unwrap();
    Ok(merkle_tree)
}

pub fn address_merkle_tree_from_bytes_zero_copy_init(
    data: &mut [u8],
    height: usize,
    canopy_depth: usize,
    changelog_capacity: usize,
    roots_capacity: usize,
    indexed_changelog_capacity: usize,
) -> Result<IndexedMerkleTreeZeroCopyMut<Poseidon, usize, 26, 16>> {
    let data = &mut data[8 + mem::size_of::<AddressMerkleTreeAccount>()..];
    let merkle_tree = IndexedMerkleTreeZeroCopyMut::from_bytes_zero_copy_init(
        data,
        height,
        canopy_depth,
        changelog_capacity,
        roots_capacity,
        indexed_changelog_capacity,
    )
    .unwrap();
    Ok(merkle_tree)
}

pub fn address_merkle_tree_from_bytes_zero_copy_mut(
    data: &mut [u8],
) -> Result<IndexedMerkleTreeZeroCopyMut<Poseidon, usize, 26, 16>> {
    let data = &mut data[8 + mem::size_of::<AddressMerkleTreeAccount>()..];
    let merkle_tree = IndexedMerkleTreeZeroCopyMut::from_bytes_zero_copy_mut(data).unwrap();
    Ok(merkle_tree)
}

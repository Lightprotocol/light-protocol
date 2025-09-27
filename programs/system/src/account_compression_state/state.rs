use std::mem;

use aligned_sized::aligned_sized;
use bytemuck::{Pod, Zeroable};
use light_concurrent_merkle_tree::{
    zero_copy::{ConcurrentMerkleTreeZeroCopy, ConcurrentMerkleTreeZeroCopyMut},
    ConcurrentMerkleTree,
};
use light_hasher::Poseidon;
use light_merkle_tree_metadata::{
    access::AccessMetadata, merkle_tree::MerkleTreeMetadata, rollover::RolloverMetadata,
};
use pinocchio::pubkey::Pubkey;

use crate::Result;

/// Concurrent state Merkle tree used for public compressed transactions.
#[repr(C)]
#[aligned_sized(anchor)]
#[derive(Pod, Debug, Default, Zeroable, Clone, Copy)]
pub struct StateMerkleTreeAccount {
    pub metadata: MerkleTreeMetadata,
}

impl StateMerkleTreeAccount {
    pub fn size(
        height: usize,
        changelog_size: usize,
        roots_size: usize,
        canopy_depth: usize,
    ) -> usize {
        8 + mem::size_of::<Self>()
            + ConcurrentMerkleTree::<Poseidon, 26>::size_in_account(
                height,
                changelog_size,
                roots_size,
                canopy_depth,
            )
    }

    pub fn init(
        &mut self,
        access_metadata: AccessMetadata,
        rollover_metadata: RolloverMetadata,
        associated_queue: Pubkey,
    ) {
        self.metadata
            .init(access_metadata, rollover_metadata, associated_queue.into())
    }
}

pub fn state_merkle_tree_from_bytes_zero_copy(
    data: &[u8],
) -> Result<ConcurrentMerkleTreeZeroCopy<'_, Poseidon, 26>> {
    let required_size = 8 + mem::size_of::<StateMerkleTreeAccount>();
    if data.len() < required_size {
        return Err(crate::errors::SystemProgramError::InvalidAccount.into());
    }
    let data = &data[required_size..];
    let merkle_tree = ConcurrentMerkleTreeZeroCopy::from_bytes_zero_copy(data)?;
    Ok(merkle_tree)
}

pub fn state_merkle_tree_from_bytes_zero_copy_mut(
    data: &mut [u8],
) -> Result<ConcurrentMerkleTreeZeroCopyMut<'_, Poseidon, 26>> {
    let required_size = 8 + mem::size_of::<StateMerkleTreeAccount>();
    if data.len() < required_size {
        return Err(crate::errors::SystemProgramError::InvalidAccount.into());
    }
    let data = &mut data[required_size..];
    let merkle_tree = ConcurrentMerkleTreeZeroCopyMut::from_bytes_zero_copy_mut(data)?;
    Ok(merkle_tree)
}

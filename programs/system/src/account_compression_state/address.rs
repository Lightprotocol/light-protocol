use std::mem;

use aligned_sized::aligned_sized;
use bytemuck::{Pod, Zeroable};
use light_hasher::Poseidon;
use light_indexed_merkle_tree::zero_copy::{
    IndexedMerkleTreeZeroCopy, IndexedMerkleTreeZeroCopyMut,
};
use light_merkle_tree_metadata::merkle_tree::MerkleTreeMetadata;

use crate::Result;

#[repr(C)]
#[aligned_sized(anchor)]
#[derive(Pod, Debug, Default, Zeroable, Clone, Copy)]
pub struct AddressMerkleTreeAccount {
    pub metadata: MerkleTreeMetadata,
}

pub fn address_merkle_tree_from_bytes_zero_copy(
    data: &[u8],
) -> Result<IndexedMerkleTreeZeroCopy<Poseidon, usize, 26, 16>> {
    let required_size = 8 + mem::size_of::<AddressMerkleTreeAccount>();
    if data.len() < required_size {
        return Err(crate::errors::SystemProgramError::InvalidAccount.into());
    }
    let data = &data[required_size..];
    let merkle_tree = IndexedMerkleTreeZeroCopy::from_bytes_zero_copy(data)?;
    Ok(merkle_tree)
}

pub fn address_merkle_tree_from_bytes_zero_copy_mut(
    data: &mut [u8],
) -> Result<IndexedMerkleTreeZeroCopyMut<Poseidon, usize, 26, 16>> {
    let required_size = 8 + mem::size_of::<AddressMerkleTreeAccount>();
    if data.len() < required_size {
        return Err(crate::errors::SystemProgramError::InvalidAccount.into());
    }
    let data = &mut data[required_size..];
    let merkle_tree = IndexedMerkleTreeZeroCopyMut::from_bytes_zero_copy_mut(data)?;
    Ok(merkle_tree)
}

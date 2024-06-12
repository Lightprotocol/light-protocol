use std::mem;

use crate::{
    utils::check_signer_is_registered_or_authority::GroupAccess, AccessMetadata,
    MerkleTreeMetadata, RolloverMetadata,
};
use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::{
    zero_copy::{IndexedMerkleTreeZeroCopy, IndexedMerkleTreeZeroCopyMut},
    IndexedMerkleTree,
};

#[account(zero_copy)]
#[aligned_sized(anchor)]
#[derive(AnchorDeserialize, Debug)]
pub struct AddressMerkleTreeAccount {
    pub metadata: MerkleTreeMetadata,
}

impl GroupAccess for AddressMerkleTreeAccount {
    fn get_owner(&self) -> &Pubkey {
        &self.metadata.access_metadata.owner
    }

    fn get_delegate(&self) -> &Pubkey {
        &self.metadata.access_metadata.program_owner
    }
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
            + IndexedMerkleTree::<Poseidon, usize, 26>::size_in_account(
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
        self.metadata
            .init(access_metadata, rollover_metadata, associated_queue)
    }
}

pub fn address_merkle_tree_from_bytes_zero_copy(
    data: &[u8],
) -> Result<IndexedMerkleTreeZeroCopy<Poseidon, usize, 26>> {
    let data = &data[8 + mem::size_of::<AddressMerkleTreeAccount>()..];
    let merkle_tree =
        IndexedMerkleTreeZeroCopy::from_bytes_zero_copy(data).map_err(ProgramError::from)?;
    Ok(merkle_tree)
}

pub fn address_merkle_tree_from_bytes_zero_copy_init(
    data: &mut [u8],
    height: usize,
    canopy_depth: usize,
    changelog_capacity: usize,
    roots_capacity: usize,
    indexed_changelog_capacity: usize,
) -> Result<IndexedMerkleTreeZeroCopyMut<Poseidon, usize, 26>> {
    let data = &mut data[8 + mem::size_of::<AddressMerkleTreeAccount>()..];
    let merkle_tree = IndexedMerkleTreeZeroCopyMut::from_bytes_zero_copy_init(
        data,
        height,
        canopy_depth,
        changelog_capacity,
        roots_capacity,
        indexed_changelog_capacity,
    )
    .map_err(ProgramError::from)?;
    Ok(merkle_tree)
}

pub fn address_merkle_tree_from_bytes_zero_copy_mut(
    data: &mut [u8],
) -> Result<IndexedMerkleTreeZeroCopyMut<Poseidon, usize, 26>> {
    let data = &mut data[8 + mem::size_of::<AddressMerkleTreeAccount>()..];
    let merkle_tree =
        IndexedMerkleTreeZeroCopyMut::from_bytes_zero_copy_mut(data).map_err(ProgramError::from)?;
    Ok(merkle_tree)
}

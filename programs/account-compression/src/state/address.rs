use crate::{AccessMetadata, MerkleTreeMetadata, RolloverMetadata, SequenceNumber};
use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use light_bounded_vec::CyclicBoundedVec;
use light_concurrent_merkle_tree::ConcurrentMerkleTree26;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::{
    copy::IndexedMerkleTreeCopy26,
    zero_copy::{IndexedMerkleTreeZeroCopy26, IndexedMerkleTreeZeroCopyMut26},
};

#[account(zero_copy)]
#[aligned_sized(anchor)]
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct AddressMerkleTreeAccount {
    pub metadata: MerkleTreeMetadata,
    pub merkle_tree_struct: [u8; 320],
    pub merkle_tree_filled_subtrees: [u8; 832],
    pub merkle_tree_changelog: [u8; 1220800],
    pub merkle_tree_roots: [u8; 76800],
    pub merkle_tree_canopy: [u8; 65472],
    pub address_changelog: [u8; 20480],
}

impl SequenceNumber for AddressMerkleTreeAccount {
    fn get_sequence_number(&self) -> Result<usize> {
        let tree = unsafe {
            IndexedMerkleTreeZeroCopy26::<Poseidon, usize>::from_bytes_zero_copy(
                &self.merkle_tree_struct,
                &self.merkle_tree_filled_subtrees,
                &self.merkle_tree_changelog,
                &self.merkle_tree_roots,
                &self.merkle_tree_canopy,
                &self.address_changelog,
            )
            .map_err(ProgramError::from)?
        };
        Ok(tree.merkle_tree.merkle_tree.sequence_number)
    }
}

impl AddressMerkleTreeAccount {
    pub fn init(
        &mut self,
        access_metadata: AccessMetadata,
        rollover_metadata: RolloverMetadata,
        associated_queue: Pubkey,
    ) {
        self.metadata
            .init(access_metadata, rollover_metadata, associated_queue)
    }

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

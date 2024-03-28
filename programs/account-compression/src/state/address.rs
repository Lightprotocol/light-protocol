use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use ark_ff::BigInteger256;
use borsh::{BorshDeserialize, BorshSerialize};
use light_bounded_vec::CyclicBoundedVec;
use light_concurrent_merkle_tree::ConcurrentMerkleTree22;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::IndexedMerkleTree22;

#[account(zero_copy)]
#[aligned_sized(anchor)]
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct AddressQueueAccount {
    pub queue: [u8; 112008],
}

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

    pub merkle_tree_struct: [u8; 224],
    pub merkle_tree_filled_subtrees: [u8; 704],
    pub merkle_tree_changelog: [u8; 2083200],
    pub merkle_tree_roots: [u8; 89600],
    pub merkle_tree_canopy: [u8; 0],
}

impl AddressMerkleTreeAccount {
    pub fn load_merkle_tree(&self) -> Result<&IndexedMerkleTree22<Poseidon, usize, BigInteger256>> {
        let tree = unsafe {
            IndexedMerkleTree22::from_bytes(
                &self.merkle_tree_struct,
                &self.merkle_tree_filled_subtrees,
                &self.merkle_tree_changelog,
                &self.merkle_tree_roots,
                &self.merkle_tree_canopy,
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
    ) -> Result<&mut IndexedMerkleTree22<Poseidon, usize, BigInteger256>> {
        let tree = unsafe {
            IndexedMerkleTree22::<Poseidon, usize, BigInteger256>::from_bytes_init(
                &mut self.merkle_tree_struct,
                &mut self.merkle_tree_filled_subtrees,
                &mut self.merkle_tree_changelog,
                &mut self.merkle_tree_roots,
                &mut self.merkle_tree_canopy,
                height,
                changelog_size,
                roots_size,
                canopy_depth,
            )
            .map_err(ProgramError::from)?
        };
        tree.init().map_err(ProgramError::from)?;
        Ok(tree)
    }

    pub fn load_merkle_tree_mut(
        &mut self,
    ) -> Result<&mut IndexedMerkleTree22<Poseidon, usize, BigInteger256>> {
        let tree = unsafe {
            IndexedMerkleTree22::from_bytes_mut(
                &mut self.merkle_tree_struct,
                &mut self.merkle_tree_filled_subtrees,
                &mut self.merkle_tree_changelog,
                &mut self.merkle_tree_roots,
                &mut self.merkle_tree_canopy,
            )
            .map_err(ProgramError::from)?
        };
        Ok(tree)
    }

    pub fn load_roots(&self) -> Result<CyclicBoundedVec<[u8; 32]>> {
        let tree = self.load_merkle_tree()?;
        let roots = unsafe {
            ConcurrentMerkleTree22::<Poseidon>::roots_from_bytes(
                &self.merkle_tree_roots,
                tree.merkle_tree.current_root_index + 1,
                tree.merkle_tree.roots_length,
                tree.merkle_tree.roots_capacity,
            )
            .map_err(ProgramError::from)?
        };
        Ok(roots)
    }
}

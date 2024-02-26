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
}

impl AddressMerkleTreeAccount {
    pub fn load_merkle_tree(&self) -> Result<&IndexedMerkleTree22<Poseidon, usize, BigInteger256>> {
        let tree = unsafe {
            IndexedMerkleTree22::from_bytes(
                &self.merkle_tree_struct,
                &self.merkle_tree_filled_subtrees,
                &self.merkle_tree_changelog,
                &self.merkle_tree_roots,
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
    ) -> Result<&mut IndexedMerkleTree22<Poseidon, usize, BigInteger256>> {
        let tree = unsafe {
            IndexedMerkleTree22::<Poseidon, usize, BigInteger256>::from_bytes_init(
                &mut self.merkle_tree_struct,
                &mut self.merkle_tree_filled_subtrees,
                &mut self.merkle_tree_changelog,
                &mut self.merkle_tree_roots,
                height,
                changelog_size,
                roots_size,
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
            )
            .map_err(ProgramError::from)?
        };
        Ok(tree)
    }
}

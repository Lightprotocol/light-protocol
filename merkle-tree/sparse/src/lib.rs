use std::marker::PhantomData;

use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use config::MerkleTreeConfig;
use light_hasher::{Hash, Hasher};

use crate::{
    constants::{DATA_LEN, HASH_LEN, MAX_HEIGHT, MAX_ROOTS},
    errors::MerkleTreeError,
};

pub mod config;
pub mod constants;
pub mod errors;
pub mod syscalls;

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone, Copy)]
#[repr(u64)]
pub enum HashFunction {
    Sha256,
    Poseidon,
}

// TODO(vadorovsky): Teach Anchor to accept `usize`, constants and const
// generics when generating IDL.
#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone, Copy)]
#[repr(C)]
pub struct MerkleTree<H, C>
where
    H: Hasher,
    C: MerkleTreeConfig,
{
    /// Height of the Merkle tree.
    pub height: u64,
    /// Subtree hashes.
    pub filled_subtrees: [[u8; 32]; MAX_HEIGHT],
    /// Full history of roots of the Merkle tree (the last one is the current
    /// one).
    pub roots: [[u8; 32]; MAX_ROOTS],
    /// Next index to insert a leaf.
    pub next_index: u64,
    /// Current index of the root.
    pub current_root_index: u64,

    /// Hash implementation used on the Merkle tree.
    pub hash_function: u64,

    hasher: PhantomData<H>,
    config: PhantomData<C>,
}

impl<H, C> MerkleTree<H, C>
where
    H: Hasher,
    C: MerkleTreeConfig,
{
    fn check_height(height: usize) -> Result<()> {
        if height == 0 {
            return err!(MerkleTreeError::HeightZero);
        }
        if height > MAX_HEIGHT {
            return err!(MerkleTreeError::HeightHigherThanMax);
        }
        Ok(())
    }

    fn init_filled_subtrees(&mut self, height: usize) {
        for i in 0..height {
            self.filled_subtrees[i] = C::ZERO_BYTES[i];
        }
    }

    fn init_roots(&mut self, height: usize) {
        self.roots[0] = C::ZERO_BYTES[height];
    }

    /// Initialize the Merkle tree with subtrees and roots based on the given
    /// height.
    pub fn init(&mut self, height: usize, hash_function: HashFunction) -> Result<()> {
        Self::check_height(height)?;

        self.height = height as u64;
        self.init_filled_subtrees(height);
        self.init_roots(height);
        self.hash_function = hash_function as u64;

        Ok(())
    }

    pub fn hash(&mut self, leaf1: [u8; DATA_LEN], leaf2: [u8; DATA_LEN]) -> Result<Hash> {
        H::hashv(&[&leaf1, &leaf2]).map_err(|e| e.into())
    }

    pub fn insert(&mut self, leaf1: [u8; DATA_LEN], leaf2: [u8; DATA_LEN]) -> Result<()> {
        // Check if next index doesn't exceed the Merkle tree capacity.
        assert_ne!(self.next_index, 2u64.pow(self.height as u32));

        let mut current_index = self.next_index / 2;
        let mut current_level_hash = self.hash(leaf1, leaf2)?;

        for i in 1..self.height as usize {
            let (left, right) = if current_index % 2 == 0 {
                self.filled_subtrees[i] = current_level_hash;
                (current_level_hash, C::ZERO_BYTES[i])
            } else {
                (self.filled_subtrees[i], current_level_hash)
            };

            current_index /= 2;
            current_level_hash = self.hash(left, right)?;
        }

        self.current_root_index = (self.current_root_index + 1) % MAX_ROOTS as u64;
        self.roots[self.current_root_index as usize] = current_level_hash;
        self.next_index += 2;

        Ok(())
    }

    pub fn last_root(&self) -> [u8; HASH_LEN] {
        self.roots[self.current_root_index as usize]
    }
}

/// The [`Pod`](bytemuck::Pod) trait is used under the hood by the
/// [`zero_copy`](anchor_lang::zero_copy) attribute macro and is required for
/// usage in zero-copy Solana accounts.
///
/// SAFETY: Generic parameters are used only as `PhantomData` and they don't
/// affect the layout of the struct nor its size or padding. The only reason
/// why we can't `#[derive(Pod)]` is because bytemuck is not aware of that and
/// it doesn't allow to derive `Pod` for structs with generic parameters.
/// Would be nice to fix that upstream:
/// https://github.com/Lokathor/bytemuck/issues/191
unsafe impl<H, C> Pod for MerkleTree<H, C>
where
    H: Hasher + Copy + 'static,
    C: MerkleTreeConfig + Copy + 'static,
{
}

/// The [`Zeroable`](bytemuck::Zeroable) trait is used under the hood by the
/// [`zero_copy`](anchor_lang::zero_copy) attribute macro and is required for
/// usage in zero-copy Solana accounts.
///
/// SAFETY: Generic parameters are used only as `PhantomData` and they don't
/// affect the layout of the struct nor its size or padding. The only reason
/// why we can't `#[derive(Zeroable)]` is because bytemuck is not aware of that
/// and it doesn't allow to derive `Zeroable` for structs with generic
/// parameters.
/// Would be nice to fix that upstream:
/// https://github.com/Lokathor/bytemuck/issues/191
unsafe impl<H, C> Zeroable for MerkleTree<H, C>
where
    H: Hasher,
    C: MerkleTreeConfig,
{
}

impl<H, C> Owner for MerkleTree<H, C>
where
    H: Hasher,
    C: MerkleTreeConfig,
{
    fn owner() -> Pubkey {
        C::PROGRAM_ID
    }
}

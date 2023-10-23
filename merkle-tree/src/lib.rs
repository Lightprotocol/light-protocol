use std::marker::PhantomData;

#[cfg(feature = "solana")]
use anchor_lang::prelude::*;

use bytemuck::{Pod, Zeroable};
use config::MerkleTreeConfig;
use hasher::{Hash, Hasher};

pub mod config;
pub mod constants;
pub mod hasher;

pub const DATA_LEN: usize = 32;
pub const HASH_LEN: usize = 32;
pub const MAX_HEIGHT: usize = 18;
pub const MERKLE_TREE_HISTORY_SIZE: usize = 20;

#[cfg(feature = "solana")]
#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone, Copy)]
pub enum HashFunction {
    Sha256,
    Poseidon,
}

// TODO(vadorovsky): Teach Anchor to accept `usize`, constants and const
// generics when generating IDL.
#[cfg_attr(feature = "solana", derive(AnchorSerialize, AnchorDeserialize))]
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[repr(C)]
pub struct MerkleTree<H, C>
where
    H: Hasher,
    C: MerkleTreeConfig,
{
    /// Height of the Merkle tree.
    pub height: u64,
    /// Subtree hashes.
    pub filled_subtrees: [[u8; 32]; 18],
    /// Full history of roots of the Merkle tree (the last one is the current
    /// one).
    pub roots: [[u8; 32]; 20],
    /// Next index to insert a leaf.
    pub next_index: u64,
    /// Current index of the root.
    pub current_root_index: u64,

    /// Hash implementation used on the Merkle tree.
    #[cfg(feature = "solana")]
    pub hash_function: HashFunction,

    hasher: PhantomData<H>,
    config: PhantomData<C>,
}

impl<H, C> MerkleTree<H, C>
where
    H: Hasher,
    C: MerkleTreeConfig,
{
    fn check_height(height: usize) {
        assert!(height > 0);
        assert!(height <= MAX_HEIGHT);
    }

    fn new_filled_subtrees(height: usize) -> [[u8; HASH_LEN]; MAX_HEIGHT] {
        let mut filled_subtrees = [[0; HASH_LEN]; MAX_HEIGHT];

        for i in 0..height {
            filled_subtrees[i] = C::ZERO_BYTES[i];
        }

        filled_subtrees
    }

    fn new_roots(height: usize) -> [[u8; HASH_LEN]; MERKLE_TREE_HISTORY_SIZE] {
        let mut roots = [[0; HASH_LEN]; MERKLE_TREE_HISTORY_SIZE];
        roots[0] = C::ZERO_BYTES[height - 1];

        roots
    }

    /// Create a new Merkle tree with the given height.
    #[cfg(not(feature = "solana"))]
    pub fn new(height: usize, #[cfg(feature = "solana")] hash_function: HashFunction) -> Self {
        Self::check_height(height);

        let filled_subtrees = Self::new_filled_subtrees(height);
        let roots = Self::new_roots(height);

        MerkleTree {
            height: height as u64,
            filled_subtrees,
            roots,
            next_index: 0,
            current_root_index: 0,
            #[cfg(feature = "solana")]
            hash_function,
            hasher: PhantomData,
            config: PhantomData,
        }
    }

    /// Initialize the Merkle tree with subtrees and roots based on the given
    /// height.
    #[cfg(feature = "solana")]
    pub fn init(&mut self, height: usize, hash_function: HashFunction) {
        Self::check_height(height);

        self.height = height as u64;
        self.filled_subtrees = Self::new_filled_subtrees(height);
        self.roots = Self::new_roots(height);
        self.hash_function = hash_function;
    }

    pub fn hash(&mut self, leaf1: [u8; DATA_LEN], leaf2: [u8; DATA_LEN]) -> Hash {
        H::hashv(&[&leaf1, &leaf2])
    }

    pub fn insert(&mut self, leaf1: [u8; DATA_LEN], leaf2: [u8; DATA_LEN]) {
        // Check if next index doesn't exceed the Merkle tree capacity.
        assert_ne!(self.next_index, 2u64.pow(self.height as u32));

        let mut current_index = self.next_index / 2;
        let mut current_level_hash = self.hash(leaf1, leaf2);

        for i in 1..self.height as usize {
            let (left, right) = if current_index % 2 == 0 {
                self.filled_subtrees[i] = current_level_hash;
                (current_level_hash, C::ZERO_BYTES[i])
            } else {
                (self.filled_subtrees[i], current_level_hash)
            };

            current_index /= 2;
            current_level_hash = self.hash(left, right);
        }

        self.current_root_index = (self.current_root_index + 1) % MERKLE_TREE_HISTORY_SIZE as u64;
        self.roots[self.current_root_index as usize] = current_level_hash;
        self.next_index += 2;
    }

    pub fn is_known_root(&self, root: [u8; HASH_LEN]) -> bool {
        for i in (0..(self.current_root_index as usize + 1)).rev() {
            if self.roots[i] == root {
                return true;
            }
        }
        return false;
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

#[cfg(feature = "solana")]
impl<H, C> Owner for MerkleTree<H, C>
where
    H: Hasher,
    C: MerkleTreeConfig,
{
    fn owner() -> Pubkey {
        C::PROGRAM_ID
    }
}

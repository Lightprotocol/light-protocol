use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use light_bounded_vec::CyclicBoundedVec;
use light_concurrent_merkle_tree::ConcurrentMerkleTree26;
use light_hasher::Poseidon;

pub type StateMerkleTree<'a> = ConcurrentMerkleTree26<'a, Poseidon>;

/// Concurrent state Merkle tree used for public compressed transactions.
#[account(zero_copy)]
#[aligned_sized(anchor)]
#[derive(AnchorDeserialize, AnchorSerialize, Debug)]
pub struct StateMerkleTreeAccount {
    /// Unique index.
    pub index: u64,
    /// Public key of the next Merkle tree.
    pub next_merkle_tree: Pubkey,
    /// Owner of the Merkle tree.
    pub owner: Pubkey,
    /// Delegate of the Merkle tree. This will be used for program owned Merkle trees.
    pub delegate: Pubkey,
    pub associated_queue: Pubkey,

    /// Merkle tree for the transaction state.
    pub state_merkle_tree_struct: [u8; 256],
    pub state_merkle_tree_filled_subtrees: [u8; 832],
    pub state_merkle_tree_changelog: [u8; 1220800],
    pub state_merkle_tree_roots: [u8; 76800],
    pub state_merkle_tree_canopy: [u8; 65472],
}

impl StateMerkleTreeAccount {
    pub fn copy_merkle_tree(&self) -> Result<ConcurrentMerkleTree26<Poseidon>> {
        let tree = unsafe {
            ConcurrentMerkleTree26::copy_from_bytes(
                &self.state_merkle_tree_struct,
                &self.state_merkle_tree_filled_subtrees,
                &self.state_merkle_tree_changelog,
                &self.state_merkle_tree_roots,
            )
            .map_err(ProgramError::from)?
        };
        Ok(tree)
    }

    pub fn load_merkle_tree(&self) -> Result<&ConcurrentMerkleTree26<Poseidon>> {
        let tree = unsafe {
            ConcurrentMerkleTree26::<Poseidon>::from_bytes(
                &self.state_merkle_tree_struct,
                &self.state_merkle_tree_filled_subtrees,
                &self.state_merkle_tree_changelog,
                &self.state_merkle_tree_roots,
                &self.state_merkle_tree_canopy,
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
    ) -> Result<&mut ConcurrentMerkleTree26<Poseidon>> {
        let tree = unsafe {
            ConcurrentMerkleTree26::<Poseidon>::from_bytes_init(
                &mut self.state_merkle_tree_struct,
                &mut self.state_merkle_tree_filled_subtrees,
                &mut self.state_merkle_tree_changelog,
                &mut self.state_merkle_tree_roots,
                &mut self.state_merkle_tree_canopy,
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

    pub fn load_merkle_tree_mut(&mut self) -> Result<&mut ConcurrentMerkleTree26<Poseidon>> {
        let tree = unsafe {
            ConcurrentMerkleTree26::<Poseidon>::from_bytes_mut(
                &mut self.state_merkle_tree_struct,
                &mut self.state_merkle_tree_filled_subtrees,
                &mut self.state_merkle_tree_changelog,
                &mut self.state_merkle_tree_roots,
                &mut self.state_merkle_tree_canopy,
            )
            .map_err(ProgramError::from)?
        };
        Ok(tree)
    }

    pub fn load_next_index(&self) -> Result<usize> {
        let tree = unsafe {
            ConcurrentMerkleTree26::<Poseidon>::struct_from_bytes(&self.state_merkle_tree_struct)
                .map_err(ProgramError::from)?
        };
        Ok(tree.next_index)
    }

    pub fn load_roots(&self) -> Result<CyclicBoundedVec<[u8; 32]>> {
        let tree = unsafe {
            ConcurrentMerkleTree26::<Poseidon>::struct_from_bytes(&self.state_merkle_tree_struct)
                .map_err(ProgramError::from)?
        };
        let roots = unsafe {
            ConcurrentMerkleTree26::<Poseidon>::roots_from_bytes(
                &self.state_merkle_tree_roots,
                tree.current_root_index + 1,
                tree.roots_length,
                tree.roots_capacity,
            )
            .map_err(ProgramError::from)?
        };
        Ok(roots)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::constants::{
        STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_CHANGELOG, STATE_MERKLE_TREE_HEIGHT,
        STATE_MERKLE_TREE_ROOTS,
    };

    #[test]
    fn test_load_merkle_tree() {
        let mut account = StateMerkleTreeAccount {
            index: 1,
            next_merkle_tree: Pubkey::new_from_array([0u8; 32]),
            owner: Pubkey::new_from_array([2u8; 32]),
            delegate: Pubkey::new_from_array([3u8; 32]),
            associated_queue: Pubkey::new_from_array([4u8; 32]),
            state_merkle_tree_struct: [0u8; 256],
            state_merkle_tree_filled_subtrees: [0u8; 832],
            state_merkle_tree_changelog: [0u8; 1220800],
            state_merkle_tree_roots: [0u8; 76800],
            state_merkle_tree_canopy: [0u8; 65472],
        };

        let merkle_tree = account
            .load_merkle_tree_init(
                STATE_MERKLE_TREE_HEIGHT as usize,
                STATE_MERKLE_TREE_CHANGELOG as usize,
                STATE_MERKLE_TREE_ROOTS as usize,
                STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
            )
            .unwrap();
        for _ in 0..(1 << 8) {
            merkle_tree.append(&[4u8; 32]).unwrap();
        }
        let root = merkle_tree.root().unwrap();

        let merkle_tree_2 = account.load_merkle_tree().unwrap();
        assert_eq!(root, merkle_tree_2.root().unwrap())
    }
}

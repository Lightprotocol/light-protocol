use std::{borrow::BorrowMut, mem};

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
}

pub unsafe fn state_mt_from_bytes_copy(account: AccountInfo) -> Result<StateMerkleTree> {
    let data = &account.try_borrow_mut_data()?[8 + mem::size_of::<StateMerkleTreeAccount>()..];
    let tree = StateMerkleTree::from_bytes_copy(data).map_err(ProgramError::from)?;
    Ok(tree)
}

pub fn state_mt_from_bytes_zero_copy<'info>(
    account: AccountLoader<'info, StateMerkleTreeAccount>,
) -> Result<StateMerkleTree> {
    let data = &account.to_account_info().try_borrow_mut_data()?
        [8 + mem::size_of::<StateMerkleTreeAccount>()..];
    let tree = unsafe { StateMerkleTree::from_bytes_zero_copy(data).map_err(ProgramError::from)? };
    Ok(tree)
}

pub fn state_mt_from_bytes_zero_copy_mut<'info>(
    account: AccountLoader<'info, StateMerkleTreeAccount>,
) -> Result<StateMerkleTree> {
    let data = &mut account.to_account_info().try_borrow_mut_data()?
        [8 + mem::size_of::<StateMerkleTreeAccount>()..];
    let tree =
        unsafe { StateMerkleTree::from_bytes_zero_copy_mut(data).map_err(ProgramError::from)? };
    Ok(tree)
}

pub fn state_mt_from_bytes_zero_copy_init<'info>(
    account: AccountLoader<'info, StateMerkleTreeAccount>,
    height: usize,
    changelog_size: usize,
    roots_size: usize,
    canopy_depth: usize,
) -> Result<StateMerkleTree> {
    let data = &mut account.to_account_info().try_borrow_mut_data()?
        [8 + mem::size_of::<StateMerkleTreeAccount>()..];
    let tree = unsafe {
        StateMerkleTree::from_bytes_zero_copy_init(
            data,
            height,
            changelog_size,
            roots_size,
            canopy_depth,
        )
        .map_err(ProgramError::from)?
    };
    Ok(tree)
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
        };

        let merkle_tree = account
            .load_merkle_tree_init(
                STATE_MERKLE_TREE_HEIGHT,
                STATE_MERKLE_TREE_CHANGELOG,
                STATE_MERKLE_TREE_ROOTS,
                STATE_MERKLE_TREE_CANOPY_DEPTH,
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

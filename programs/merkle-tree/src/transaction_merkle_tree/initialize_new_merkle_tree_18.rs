use std::cell::RefMut;

use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use crate::{transaction_merkle_tree::state::TransactionMerkleTree, MerkleTreeAuthority};

// keeps track of leaves which have been queued but not inserted into the merkle tree yet
#[account]
pub struct MerkleTreePdaToken {}

// keeps track of leaves which have been queued but not inserted into the merkle tree yet
#[account]
pub struct PreInsertedLeavesIndex {
    pub next_index: u64,
}

#[cfg(feature = "atomic-transactions")]
pub fn process_initialize_new_merkle_tree(
    merkle_tree: &mut RefMut<'_, TransactionMerkleTree>,
    merkle_tree_authority: &mut Account<'_, MerkleTreeAuthority>,
    height: usize,
) -> Result<()> {
    use light_sparse_merkle_tree::HashFunction;

    merkle_tree.newest = 1;
    merkle_tree
        .merkle_tree
        .init(height, HashFunction::Poseidon)?;
    merkle_tree_authority.transaction_merkle_tree_index += 1;

    Ok(())
}

#[cfg(not(feature = "atomic-transactions"))]
pub fn process_initialize_new_merkle_tree(
    merkle_tree: &mut RefMut<'_, TransactionMerkleTree>,
    merkle_tree_authority: &mut Account<'_, MerkleTreeAuthority>,
    height: usize,
) -> Result<()> {
    use light_merkle_tree::constants::poseidon::ZERO_BYTES;

    merkle_tree.newest = 1;

    merkle_tree.merkle_tree.filled_subtrees[..height].copy_from_slice(&ZERO_BYTES[..height]);

    merkle_tree.merkle_tree.height = merkle_tree
        .merkle_tree
        .filled_subtrees
        .len()
        .try_into()
        .unwrap();
    merkle_tree.merkle_tree_nr = merkle_tree_authority.transaction_merkle_tree_index;
    merkle_tree.merkle_tree.roots[0] = ZERO_BYTES[height];
    msg!(
        "merkle_tree_state_data.roots[0]: {:?}",
        merkle_tree.merkle_tree.roots[0]
    );

    merkle_tree_authority.transaction_merkle_tree_index += 1;

    Ok(())
}

#[cfg(test)]
mod test {
    use std::cell::RefCell;

    use super::*;
    use crate::{config::MERKLE_TREE_HISTORY_SIZE, ZERO_BYTES_MERKLE_TREE_18};

    #[test]
    fn test_init_merkle_tree() {
        let mt = TransactionMerkleTree {
            filled_subtrees: [[0u8; 32]; 18],
            current_root_index: 0u64,
            next_index: 0u64,
            roots: [[0u8; 32]; MERKLE_TREE_HISTORY_SIZE as usize],
            pubkey_locked: Pubkey::try_from([0u8; 32])
                .map_err(|_| ErrorCode::PubkeyTryFromFailed)?,
            time_locked: 0u64,
            height: 0u64,
            merkle_tree_nr: 0u64,
            lock_duration: 20u64,
            next_queued_index: 0u64,
        };
        let height = 18;
        let mt_index = 0;
        let binding = &mut RefCell::new(mt);
        let mut ref_mt = binding.borrow_mut();
        process_initialize_new_merkle_tree_18(
            &mut ref_mt,
            height,
            ZERO_BYTES_MERKLE_TREE_18.to_vec(),
            mt_index,
        );

        assert_eq!(ref_mt.height, 18, "height inited wrong");
        assert_eq!(ref_mt.merkle_tree_nr, 0, "merkle_tree_nr inited wrong");
        assert_eq!(
            ref_mt.pubkey_locked,
            Pubkey::try_from([0u8; 32]).map_err(|| ErrorCode::PubkeyTryFromFailed),
            "pubkey_locked inited wrong"
        );
        assert_eq!(ref_mt.next_index, 0, "next_index inited wrong");
        assert_eq!(
            ref_mt.current_root_index, 0,
            "current_root_index inited wrong"
        );
    }
}

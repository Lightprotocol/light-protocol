use solana_program::{msg, program_error::ProgramError};

use crate::poseidon_merkle_tree::state::{MerkleTree, TmpStoragePda};
use crate::utils::init_bytes18::ZERO_BYTES_MERKLE_TREE_18;
/*
pub fn insert_0(leaf: &Vec<u8>, merkle_tree_account: &mut MerkleTree, tmp_storage_account:&mut TmpStoragePda) {
    tmp_storage_account.current_index =  merkle_tree_account.next_index;
    assert!(tmp_storage_account.current_index != 2048/*2usize^merkle_tree_account.levels*/, "Merkle tree is full. No more leaves can be added");

    tmp_storage_account.current_level_hash = leaf.clone();
    merkle_tree_account.leaves = leaf.clone();


    merkle_tree_account.inserted_leaf = true;
}
*/
pub fn insert_0_double(
    merkle_tree_account: &mut MerkleTree,
    tmp_storage_account: &mut TmpStoragePda,
) -> Result<(), ProgramError> {
    tmp_storage_account.current_index = merkle_tree_account.next_index / 2;
    msg!(
        "current index hash bytes: {}",
        tmp_storage_account.current_index
    );
    msg!(
        "tmp_storage_account.leaf_left: {:?}",
        tmp_storage_account.leaf_left
    );
    msg!(
        "tmp_storage_account.leaf_right: {:?}",
        tmp_storage_account.leaf_right
    );

    //assert!(tmp_storage_account.current_index != 2048/*2usize^merkle_tree_account.levels*/, "Merkle tree is full. No more leaves can be added");
    if tmp_storage_account.current_index == 262144 {
        msg!("Merkle tree full");
        return Err(ProgramError::InvalidInstructionData);
    }
    tmp_storage_account.left = tmp_storage_account.leaf_left.clone();
    tmp_storage_account.right = tmp_storage_account.leaf_right.clone();
    tmp_storage_account.current_level = 1;
    merkle_tree_account.inserted_leaf = true;
    //zeroing out prior state since the account was used for prior computation
    tmp_storage_account.state = vec![vec![0u8; 32]; 3];
    tmp_storage_account.current_round = 0;
    tmp_storage_account.current_round_index = 0;
    tmp_storage_account.current_level_hash = vec![0u8; 32];
    Ok(())
}

pub fn insert_1_inner_loop(
    merkle_tree_account: &mut MerkleTree,
    tmp_storage_account: &mut TmpStoragePda,
) -> Result<(), ProgramError> {
    msg!(
        "insert_1_inner_loop_0 level {:?}",
        tmp_storage_account.current_level
    );
    msg!(
        "current_level_hash {:?}",
        tmp_storage_account.current_level_hash
    );
    if tmp_storage_account.current_level != 0 {
        tmp_storage_account.current_level_hash = tmp_storage_account.state[0].clone();
    }

    if tmp_storage_account.current_index % 2 == 0 {
        msg!(
            "updating subtree: {:?}",
            tmp_storage_account.current_level_hash
        );
        tmp_storage_account.left = tmp_storage_account.current_level_hash.clone();
        tmp_storage_account.right = ZERO_BYTES_MERKLE_TREE_18
            [tmp_storage_account.current_level * 32..(tmp_storage_account.current_level * 32 + 32)]
            .to_vec()
            .clone();
        merkle_tree_account.filled_subtrees[tmp_storage_account.current_level] =
            tmp_storage_account.current_level_hash.clone();
    } else {
        tmp_storage_account.left =
            merkle_tree_account.filled_subtrees[tmp_storage_account.current_level].clone();
        tmp_storage_account.right = tmp_storage_account.current_level_hash.clone();
    }
    tmp_storage_account.current_index /= 2;
    tmp_storage_account.current_level += 1;
    msg!("current_index {:?}", tmp_storage_account.current_index);

    msg!(
        "tmp_storage_account.leaf_left: {:?}",
        tmp_storage_account.left
    );
    msg!(
        "tmp_storage_account.leaf_right: {:?}",
        tmp_storage_account.right
    );
    Ok(())
}

pub fn insert_last_double(
    merkle_tree_account: &mut MerkleTree,
    tmp_storage_account: &mut TmpStoragePda,
) -> Result<(), ProgramError> {
    merkle_tree_account.current_root_index =
        (merkle_tree_account.current_root_index + 1) % merkle_tree_account.root_history_size;
    merkle_tree_account.next_index += 2;
    msg!(
        "merkle_tree_account.next_index {:?}",
        merkle_tree_account.next_index
    );

    //roots unpacks only the current root and write only this one
    merkle_tree_account.roots = tmp_storage_account.state[0].clone();
    merkle_tree_account.inserted_root = true;
    Ok(())
}

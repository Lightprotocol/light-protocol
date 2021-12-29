use solana_program::{
    msg,
    log::sol_log_compute_units,
    program_error::ProgramError,
};

use crate::poseidon_merkle_tree::mt_state::{
    MerkleTree,
    HashBytes
};
use crate::utils::init_bytes18::ZERO_BYTES_MERKLE_TREE_18;
/*
pub fn insert_0(leaf: &Vec<u8>, merkle_tree_account: &mut MerkleTree, hash_bytes_account:&mut HashBytes) {
    hash_bytes_account.current_index =  merkle_tree_account.next_index;
    assert!(hash_bytes_account.current_index != 2048/*2usize^merkle_tree_account.levels*/, "Merkle tree is full. No more leaves can be added");

    hash_bytes_account.current_level_hash = leaf.clone();
    merkle_tree_account.leaves = leaf.clone();


    merkle_tree_account.inserted_leaf = true;
}
*/
pub fn insert_0_double(leaf_l: &Vec<u8>, leaf_r: &Vec<u8>, merkle_tree_account: &mut MerkleTree, hash_bytes_account:&mut HashBytes) -> Result<(), ProgramError>{
    hash_bytes_account.current_index =  merkle_tree_account.next_index;
    //assert!(hash_bytes_account.current_index != 2048/*2usize^merkle_tree_account.levels*/, "Merkle tree is full. No more leaves can be added");
    if hash_bytes_account.current_index == 262144 {
        msg!("Merkle tree full");
        return Err(ProgramError::InvalidInstructionData);
    }
    hash_bytes_account.left = hash_bytes_account.leaf_left.clone();
    hash_bytes_account.right =  hash_bytes_account.leaf_right.clone();
    hash_bytes_account.current_level = 1;
    merkle_tree_account.inserted_leaf = true;
    //zeroing out prior state since the account was used for prior computation
    hash_bytes_account.state = vec![vec![0u8;32];3];
    hash_bytes_account.current_round  = 0;
    hash_bytes_account.current_round_index  = 0;
    hash_bytes_account.current_level_hash  = vec![0u8;32];
    Ok(())
}

pub fn insert_1_inner_loop(merkle_tree_account: &mut MerkleTree, hash_bytes_account:&mut HashBytes) {
    //msg!("insert_1_inner_loop_0 level {:?}",hash_bytes_account.current_level);
    //msg!("current_level_hash {:?}",hash_bytes_account.current_level_hash);
    if hash_bytes_account.current_level != 0 {
        hash_bytes_account.current_level_hash = hash_bytes_account.state[0].clone();
    }

    if hash_bytes_account.current_index % 2 == 0 {
        //msg!("updating subtree: {:?}", hash_bytes_account.current_level_hash);
        hash_bytes_account.left = hash_bytes_account.current_level_hash.clone();
        hash_bytes_account.right =  ZERO_BYTES_MERKLE_TREE_18[ hash_bytes_account.current_level * 32..(hash_bytes_account.current_level * 32 + 32) ].to_vec().clone();
        merkle_tree_account.filled_subtrees[ hash_bytes_account.current_level] = hash_bytes_account.current_level_hash.clone();
    } else {
        hash_bytes_account.left =  merkle_tree_account.filled_subtrees[ hash_bytes_account.current_level].clone();
        hash_bytes_account.right = hash_bytes_account.current_level_hash.clone();
    }
    hash_bytes_account.current_index /= 2;
    hash_bytes_account.current_level += 1;
    //merkle_tree_account.inserted_leaf = true;
    //msg!("insert_1_inner_loop_0 subtrees {:?}",merkle_tree_account.filled_subtrees);

}

pub fn insert_last_double(merkle_tree_account: &mut MerkleTree, hash_bytes_account:&mut HashBytes) {
    merkle_tree_account.current_root_index = ( merkle_tree_account.current_root_index + 1) %  merkle_tree_account.root_history_size;
    merkle_tree_account.next_index+= 2;

    //roots unpacks only the current root and write only this one
    merkle_tree_account.roots = hash_bytes_account.state[0].clone();
    merkle_tree_account.inserted_root = true;


}

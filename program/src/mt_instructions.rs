use ark_crypto_primitives::crh::{pedersen, poseidon, *};
use ark_ff::{fields, models::Fp256};
use ark_ec::twisted_edwards_extended::*;

use ark_ff::bytes::{FromBytes, ToBytes};
use ark_ff::fields::models::Fp256Parameters;
use ark_ff::biginteger::BigInteger256;
use solana_program::{
    msg,
    log::sol_log_compute_units,
    account_info::{next_account_info, AccountInfo},
};
use ark_ff::PrimeField;
use ark_ff::BigInteger;

use crate::mt_state::{MerkleTree,HashBytes, TwoLeavesBytesPda};
use crate::init_bytes18::ZERO_BYTES_MERKLE_TREE_18;
/*
pub fn insert_0(leaf: &Vec<u8>, merkle_tree_account: &mut MerkleTree, hash_bytes_account:&mut HashBytes) {
    hash_bytes_account.currentIndex =  merkle_tree_account.next_index;
    assert!(hash_bytes_account.currentIndex != 2048/*2usize^merkle_tree_account.levels*/, "Merkle tree is full. No more leaves can be added");

    hash_bytes_account.currentLevelHash = leaf.clone();
    merkle_tree_account.leaves = leaf.clone();


    merkle_tree_account.inserted_leaf = true;
}
*/
pub fn insert_0_double(leaf_l: &Vec<u8>, leaf_r: &Vec<u8>, merkle_tree_account: &mut MerkleTree, hash_bytes_account:&mut HashBytes) {
    hash_bytes_account.currentIndex =  merkle_tree_account.next_index;
    assert!(hash_bytes_account.currentIndex != 2048/*2usize^merkle_tree_account.levels*/, "Merkle tree is full. No more leaves can be added");

    //hash_bytes_account.currentLevelHash = leaf.clone();
    //merkle_tree_account.leaves = leaf.clone();
    //hash_bytes_account.leaf_left =  leaf_r.clone();
    //hash_bytes_account.leaf_right =  leaf_l.clone();
    hash_bytes_account.left = hash_bytes_account.leaf_left.clone();
    hash_bytes_account.right =  hash_bytes_account.leaf_right.clone();
    hash_bytes_account.currentLevel = 1;
    merkle_tree_account.inserted_leaf = true;
    //zeroing out prior state since the account was used for prior computation
    hash_bytes_account.state = vec![vec![0u8;32];3];
    hash_bytes_account.current_round  = 0;
    hash_bytes_account.current_round_index  = 0;
    hash_bytes_account.currentLevelHash  = vec![0u8;32];
}

pub fn insert_1_inner_loop(merkle_tree_account: &mut MerkleTree, hash_bytes_account:&mut HashBytes) {
    //msg!("insert_1_inner_loop_0 level {:?}",hash_bytes_account.currentLevel);
    //msg!("currentLevelHash {:?}",hash_bytes_account.currentLevelHash);
    if hash_bytes_account.currentLevel != 0 {
        hash_bytes_account.currentLevelHash = hash_bytes_account.state[0].clone();
    }

    if(hash_bytes_account.currentIndex % 2 == 0) {
        //msg!("updating subtree: {:?}", hash_bytes_account.currentLevelHash);
        hash_bytes_account.left = hash_bytes_account.currentLevelHash.clone();
        hash_bytes_account.right =  ZERO_BYTES_MERKLE_TREE_18[ hash_bytes_account.currentLevel * 32..(hash_bytes_account.currentLevel * 32 + 32) ].to_vec().clone();
        merkle_tree_account.filledSubtrees[ hash_bytes_account.currentLevel] = hash_bytes_account.currentLevelHash.clone();
    } else {
        hash_bytes_account.left =  merkle_tree_account.filledSubtrees[ hash_bytes_account.currentLevel].clone();
        hash_bytes_account.right = hash_bytes_account.currentLevelHash.clone();
    }
    hash_bytes_account.currentIndex /= 2;
    hash_bytes_account.currentLevel += 1;
    //merkle_tree_account.inserted_leaf = true;
    //msg!("insert_1_inner_loop_0 subtrees {:?}",merkle_tree_account.filledSubtrees);

}

pub fn insert_last_double(merkle_tree_account: &mut MerkleTree, hash_bytes_account:&mut HashBytes) {
    merkle_tree_account.currentRootIndex = ( merkle_tree_account.currentRootIndex + 1) %  merkle_tree_account.root_history_size;
    merkle_tree_account.next_index+= 2;

    //roots unpacks only the current root and write only this one
    merkle_tree_account.roots = hash_bytes_account.state[0].clone();
    merkle_tree_account.inserted_root = true;


}

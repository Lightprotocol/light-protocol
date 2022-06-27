
use anchor_lang::solana_program::{msg, program_error::ProgramError};

use crate::poseidon_merkle_tree::state::MerkleTree;
use crate::poseidon_merkle_tree::update_merkle_tree_lib::update_state::MerkleTreeTmpPda;
use crate::utils::config::ZERO_BYTES_MERKLE_TREE_18;
use std::cell::RefMut;

pub fn insert_0_double(
    merkle_tree_account: &mut MerkleTree,
    tmp_storage_account: &mut MerkleTreeTmpPda,
) -> Result<(), ProgramError> {
    tmp_storage_account.current_index = (merkle_tree_account.next_index as u64 + tmp_storage_account.insert_leaves_index as u64 * 2 ) / 2;
    msg!(
        "current index hash bytes: {}",
        tmp_storage_account.current_index
    );
    msg!(
        "tmp_storage_account.node_left: {:?}",
        tmp_storage_account.node_left
    );
    msg!(
        "tmp_storage_account.node_right: {:?}",
        tmp_storage_account.node_right
    );

    if tmp_storage_account.current_index == 262144 {
        msg!("Merkle tree full");
        return Err(ProgramError::InvalidInstructionData);
    }
    tmp_storage_account.node_left = tmp_storage_account.leaves[tmp_storage_account.insert_leaves_index as usize][0];
    tmp_storage_account.node_right = tmp_storage_account.leaves[tmp_storage_account.insert_leaves_index as usize][1];
    println!("tmp_storage_account.node_left {:?}", tmp_storage_account.node_left);
    println!("tmp_storage_account.node_right {:?}", tmp_storage_account.node_right);

    tmp_storage_account.current_level = 1;
    // increase insert leaves index to insert the next leaf
    tmp_storage_account.insert_leaves_index +=1;
    println!("tmp_storage_account.insert_leaves_index {}", tmp_storage_account.insert_leaves_index);
    tmp_storage_account.tmp_leaves_index += 2;

    merkle_tree_account.inserted_leaf = true;
    //zeroing out prior state since the account was used for prior computation
    tmp_storage_account.state = [0u8; 96];
    tmp_storage_account.current_round = 0;
    tmp_storage_account.current_round_index = 0;
    tmp_storage_account.current_level_hash = [0u8; 32];
    Ok(())
}



pub fn insert_1_inner_loop(
    merkle_tree_account: &mut MerkleTree,
    tmp_storage_account: &mut MerkleTreeTmpPda,
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
        tmp_storage_account.current_level_hash = tmp_storage_account.state[0..32].try_into().unwrap();
    }
    msg!("tmp_storage_account.current_index {}", tmp_storage_account.current_index);
    if tmp_storage_account.current_index % 2 == 0 {
        msg!(
            "updating subtree: {:?}",
            tmp_storage_account.current_level_hash
        );

        tmp_storage_account.node_left = tmp_storage_account.current_level_hash.clone();
        tmp_storage_account.node_right = ZERO_BYTES_MERKLE_TREE_18
            [tmp_storage_account.current_level as usize * 32..(tmp_storage_account.current_level as usize * 32 + 32)].try_into().unwrap();
        merkle_tree_account.filled_subtrees[tmp_storage_account.current_level as usize] =
            tmp_storage_account.current_level_hash.clone().to_vec();
        // check if there is another queued leaves pair
        if tmp_storage_account.insert_leaves_index < tmp_storage_account.number_of_leaves {
            msg!("\n\nresetting current_instruction_index {} < {}\n\n", tmp_storage_account.insert_leaves_index, tmp_storage_account.number_of_leaves);

            // reset current_instruction_index to 1 since the lock is already taken
            tmp_storage_account.current_instruction_index = 1;

            // increase tmp index by pair

            // insert next leaves pair
            insert_0_double(merkle_tree_account, tmp_storage_account)?;
            return Ok(());
        }


    } else {

        tmp_storage_account.node_left =
            merkle_tree_account.filled_subtrees[tmp_storage_account.current_level as usize].clone().try_into().unwrap();
        tmp_storage_account.node_right = tmp_storage_account.current_level_hash.clone();
    }
    tmp_storage_account.current_index /= 2;
    tmp_storage_account.current_level += 1;
    msg!("current_index {:?}", tmp_storage_account.current_index);

    msg!(
        "tmp_storage_account.node_left: {:?}",
        tmp_storage_account.node_left
    );
    msg!(
        "tmp_storage_account.node_right: {:?}",
        tmp_storage_account.node_right
    );
    Ok(())
}

pub fn insert_last_double(
    merkle_tree_account: &mut MerkleTree,
    tmp_storage_account: &mut RefMut<'_, MerkleTreeTmpPda>,
) -> Result<(), ProgramError> {
    merkle_tree_account.current_root_index =
        ((merkle_tree_account.current_root_index + 1) % merkle_tree_account.root_history_size).try_into().unwrap();
    //
    msg!(
        "merkle_tree_account.current_root_index {}",
        merkle_tree_account.current_root_index
    );
    merkle_tree_account.next_index = tmp_storage_account.tmp_leaves_index.try_into().unwrap();
    msg!(
        "merkle_tree_account.next_index {:?}",
        merkle_tree_account.next_index
    );
    msg!("tmp_storage_account.state[0..32].to_vec() {:?}", tmp_storage_account.state[0..32].to_vec());
    //roots unpacks only the current root and write only this one
    merkle_tree_account.roots = tmp_storage_account.state[0..32].to_vec();
    merkle_tree_account.inserted_root = true;
    Ok(())
}

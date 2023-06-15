use anchor_lang::solana_program::{msg, program_error::ProgramError};

use crate::transaction_merkle_tree::state::TransactionMerkleTree;
use crate::transaction_merkle_tree::update_merkle_tree_lib::merkle_tree_update_state::MerkleTreeUpdateState;
use crate::utils::config::ZERO_BYTES_MERKLE_TREE_18;
use std::cell::RefMut;

pub fn insert_0_double(
    merkle_tree_account: &mut RefMut<'_, TransactionMerkleTree>,
    update_state_data: &mut MerkleTreeUpdateState,
) -> Result<(), ProgramError> {
    update_state_data.current_index = (merkle_tree_account.next_index as u64
        + update_state_data.insert_leaves_index as u64 * 2)
        / 2;

    if update_state_data.current_index == 262144 {
        msg!("Merkle tree full");
        return Err(ProgramError::InvalidInstructionData);
    }
    update_state_data.node_left = update_state_data.leaves
        [usize::try_from(update_state_data.insert_leaves_index).unwrap()][0];
    update_state_data.node_right = update_state_data.leaves
        [usize::try_from(update_state_data.insert_leaves_index).unwrap()][1];

    update_state_data.current_level = 1;
    // increase insert leaves index to insert the next leaf
    update_state_data.insert_leaves_index += 1;

    update_state_data.tmp_leaves_index += 2;

    //zeroing out prior state since the account was used for prior computation
    update_state_data.state = [0u8; 96];
    update_state_data.current_round = 0;
    update_state_data.current_round_index = 0;
    update_state_data.current_level_hash = [0u8; 32];
    Ok(())
}

pub fn insert_1_inner_loop(
    merkle_tree_account: &mut RefMut<'_, TransactionMerkleTree>,
    update_state_data: &mut MerkleTreeUpdateState,
) -> Result<(), ProgramError> {
    if update_state_data.current_level != 0 {
        update_state_data.current_level_hash = update_state_data.state[0..32].try_into().unwrap();
    }
    if update_state_data.current_index % 2 == 0 {
        update_state_data.node_left = update_state_data.current_level_hash;
        update_state_data.node_right =
            ZERO_BYTES_MERKLE_TREE_18[usize::try_from(update_state_data.current_level).unwrap()];
        update_state_data.filled_subtrees
            [usize::try_from(update_state_data.current_level).unwrap()] =
            update_state_data.current_level_hash;
        // check if there is another queued leaves pair
        if update_state_data.insert_leaves_index < update_state_data.number_of_leaves {
            // reset current_instruction_index to 1 since the lock is already taken
            update_state_data.current_instruction_index = 1;

            // increase tmp index by pair

            // insert next leaves pair
            insert_0_double(merkle_tree_account, update_state_data)?;
            return Ok(());
        }
    } else {
        update_state_data.node_left = update_state_data.filled_subtrees
            [usize::try_from(update_state_data.current_level).unwrap()];
        update_state_data.node_right = update_state_data.current_level_hash;
    }
    update_state_data.current_index /= 2;
    update_state_data.current_level += 1;
    Ok(())
}

pub fn insert_last_double(
    merkle_tree_account: &mut RefMut<'_, TransactionMerkleTree>,
    update_state_data: &mut RefMut<'_, MerkleTreeUpdateState>,
) -> Result<(), ProgramError> {
    merkle_tree_account.current_root_index = (merkle_tree_account.current_root_index + 1)
        % u64::try_from(merkle_tree_account.roots.len()).unwrap();

    merkle_tree_account.next_index = update_state_data.tmp_leaves_index;
    let index: usize = merkle_tree_account.current_root_index.try_into().unwrap();

    merkle_tree_account.roots[index] = update_state_data.state[0..32].try_into().unwrap();

    merkle_tree_account.filled_subtrees = update_state_data.filled_subtrees;

    Ok(())
}

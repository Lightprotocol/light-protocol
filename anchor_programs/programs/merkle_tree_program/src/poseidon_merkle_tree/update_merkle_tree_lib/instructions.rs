use anchor_lang::solana_program::{msg, program_error::ProgramError};

use crate::poseidon_merkle_tree::state::MerkleTree;
use crate::poseidon_merkle_tree::update_merkle_tree_lib::merkle_tree_update_state::MerkleTreeUpdateState;
use crate::utils::config::ZERO_BYTES_MERKLE_TREE_18;
use std::cell::RefMut;

pub fn insert_0_double(
    merkle_tree_account: &mut MerkleTree,
    verifier_state_data: &mut MerkleTreeUpdateState,
) -> Result<(), ProgramError> {
    verifier_state_data.current_index = (merkle_tree_account.next_index as u64
        + verifier_state_data.insert_leaves_index as u64 * 2)
        / 2;
    msg!(
        "current index hash bytes: {}",
        verifier_state_data.current_index
    );
    msg!(
        "verifier_state_data.node_left: {:?}",
        verifier_state_data.node_left
    );
    msg!(
        "verifier_state_data.node_right: {:?}",
        verifier_state_data.node_right
    );

    if verifier_state_data.current_index == 262144 {
        msg!("Merkle tree full");
        return Err(ProgramError::InvalidInstructionData);
    }
    verifier_state_data.node_left =
        verifier_state_data.leaves[usize::try_from(verifier_state_data.insert_leaves_index).unwrap()][0];
    verifier_state_data.node_right =
        verifier_state_data.leaves[usize::try_from(verifier_state_data.insert_leaves_index).unwrap()][1];
    println!(
        "verifier_state_data.node_left {:?}",
        verifier_state_data.node_left
    );
    println!(
        "verifier_state_data.node_right {:?}",
        verifier_state_data.node_right
    );

    verifier_state_data.current_level = 1;
    // increase insert leaves index to insert the next leaf
    verifier_state_data.insert_leaves_index += 1;
    println!(
        "verifier_state_data.insert_leaves_index {}",
        verifier_state_data.insert_leaves_index
    );
    verifier_state_data.tmp_leaves_index += 2;

    merkle_tree_account.inserted_leaf = true;
    //zeroing out prior state since the account was used for prior computation
    verifier_state_data.state = [0u8; 96];
    verifier_state_data.current_round = 0;
    verifier_state_data.current_round_index = 0;
    verifier_state_data.current_level_hash = [0u8; 32];
    Ok(())
}

pub fn insert_1_inner_loop(
    merkle_tree_account: &mut MerkleTree,
    verifier_state_data: &mut MerkleTreeUpdateState,
) -> Result<(), ProgramError> {
    msg!(
        "insert_1_inner_loop_0 level {:?}",
        verifier_state_data.current_level
    );
    msg!(
        "current_level_hash {:?}",
        verifier_state_data.current_level_hash
    );
    if verifier_state_data.current_level != 0 {
        verifier_state_data.current_level_hash =
            verifier_state_data.state[0..32].try_into().unwrap();
    }
    msg!(
        "verifier_state_data.current_index {}",
        verifier_state_data.current_index
    );
    if verifier_state_data.current_index % 2 == 0 {
        msg!(
            "updating subtree: {:?}",
            verifier_state_data.current_level_hash
        );

        verifier_state_data.node_left = verifier_state_data.current_level_hash.clone();
        verifier_state_data.node_right =
            ZERO_BYTES_MERKLE_TREE_18[usize::try_from(verifier_state_data.current_level).unwrap() * 32
                ..(usize::try_from(verifier_state_data.current_level).unwrap() * 32 + 32)]
                .try_into()
                .unwrap();
        merkle_tree_account.filled_subtrees[usize::try_from(verifier_state_data.current_level).unwrap()] =
            verifier_state_data.current_level_hash.clone().to_vec();
        // check if there is another queued leaves pair
        if verifier_state_data.insert_leaves_index < verifier_state_data.number_of_leaves {
            msg!(
                "\n\nresetting current_instruction_index {} < {}\n\n",
                verifier_state_data.insert_leaves_index,
                verifier_state_data.number_of_leaves
            );

            // reset current_instruction_index to 1 since the lock is already taken
            verifier_state_data.current_instruction_index = 1;

            // increase tmp index by pair

            // insert next leaves pair
            insert_0_double(merkle_tree_account, verifier_state_data)?;
            return Ok(());
        }
    } else {
        verifier_state_data.node_left = merkle_tree_account.filled_subtrees
            [usize::try_from(verifier_state_data.current_level).unwrap()]
            .clone()
            .try_into()
            .unwrap();
        verifier_state_data.node_right = verifier_state_data.current_level_hash.clone();
    }
    verifier_state_data.current_index /= 2;
    verifier_state_data.current_level += 1;
    msg!("current_index {:?}", verifier_state_data.current_index);

    msg!(
        "verifier_state_data.node_left: {:?}",
        verifier_state_data.node_left
    );
    msg!(
        "verifier_state_data.node_right: {:?}",
        verifier_state_data.node_right
    );
    Ok(())
}

pub fn insert_last_double(
    merkle_tree_account: &mut MerkleTree,
    verifier_state_data: &mut RefMut<'_, MerkleTreeUpdateState>,
) -> Result<(), ProgramError> {
    merkle_tree_account.current_root_index = ((merkle_tree_account.current_root_index + 1)
        % merkle_tree_account.root_history_size)
        .try_into()
        .unwrap();
    //
    msg!(
        "merkle_tree_account.current_root_index {}",
        merkle_tree_account.current_root_index
    );
    merkle_tree_account.next_index = verifier_state_data.tmp_leaves_index.try_into().unwrap();
    msg!(
        "merkle_tree_account.next_index {:?}",
        merkle_tree_account.next_index
    );
    msg!(
        "verifier_state_data.state[0..32].to_vec() {:?}",
        verifier_state_data.state[0..32].to_vec()
    );
    //roots unpacks only the current root and write only this one
    merkle_tree_account.roots = verifier_state_data.state[0..32].to_vec();
    merkle_tree_account.inserted_root = true;
    Ok(())
}

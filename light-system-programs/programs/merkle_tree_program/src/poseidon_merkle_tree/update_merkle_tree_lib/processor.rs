use crate::poseidon_merkle_tree::instructions::*;
use crate::poseidon_merkle_tree::instructions_poseidon::{poseidon_0, poseidon_1, poseidon_2};
use crate::poseidon_merkle_tree::state::MerkleTree;
use crate::poseidon_merkle_tree::update_merkle_tree_lib::merkle_tree_update_state::MerkleTreeUpdateState;
use crate::utils::config::MERKLE_TREE_HEIGHT;
use crate::utils::constants::{HASH_0, HASH_1, HASH_2, MERKLE_TREE_UPDATE_START};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{msg, pubkey::Pubkey};

use crate::errors::ErrorCode;
use std::cell::RefMut;

pub fn compute_updated_merkle_tree(
    id: u8,
    merkle_tree_update_state_data: &mut MerkleTreeUpdateState,
    merkle_tree_pda_data: &mut RefMut<'_, MerkleTree>,
) -> Result<()> {
    msg!("executing instruction {}", id);
    // Hash computation is split into three parts which can be executed in ~2m compute units
    if id == HASH_0 {
        poseidon_0(merkle_tree_update_state_data)?;
    } else if id == HASH_1 {
        poseidon_1(merkle_tree_update_state_data)?;
    } else if id == HASH_2 {
        poseidon_2(merkle_tree_update_state_data)?;
        // Updating the current level hash after a new hash is completely computed.
        if merkle_tree_update_state_data.current_level < MERKLE_TREE_HEIGHT {
            insert_1_inner_loop(merkle_tree_pda_data, merkle_tree_update_state_data)?;
        }
    } else if id == MERKLE_TREE_UPDATE_START {
        insert_0_double(merkle_tree_pda_data, merkle_tree_update_state_data)?;
    }
    Ok(())
}

pub fn pubkey_check(account_pubkey0: Pubkey, account_pubkey1: Pubkey, msg: String) -> Result<()> {
    if account_pubkey0 != account_pubkey1 {
        msg!(&msg);
        return err!(ErrorCode::PubkeyCheckFailed);
    }

    Ok(())
}

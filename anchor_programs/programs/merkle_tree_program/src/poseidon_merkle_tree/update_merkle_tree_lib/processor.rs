use crate::poseidon_merkle_tree::instructions::*;
use crate::poseidon_merkle_tree::instructions_poseidon::{poseidon_0, poseidon_1, poseidon_2};
use crate::poseidon_merkle_tree::state::{InitMerkleTreeBytes, MerkleTree};
use crate::poseidon_merkle_tree::update_merkle_tree_lib::update_state::MerkleTreeTmpPda;
use crate::utils::config::{
    MERKLE_TREE_HEIGHT
};
use crate::utils::constants::{
    MERKLE_TREE_UPDATE_START,
    HASH_0,
    HASH_1,
    HASH_2,
    ROOT_INSERT,
    IX_ORDER
};
use anchor_lang::solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    msg,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::rent::Rent,
    sysvar::Sysvar,
};
use anchor_lang::prelude::*;

use crate::errors::ErrorCode;

pub fn compute_updated_merkle_tree(
    id: u8,
    tmp_storage_pda_data: &mut MerkleTreeTmpPda,
    merkle_tree_pda_data: &mut MerkleTree,
) -> Result<()>  {
    msg!("executing instruction {}", id);
    // Hash computation is split into three parts which can be executed in ~2m compute units
    if id == HASH_0 {
        poseidon_0(tmp_storage_pda_data)?;
    } else if id == HASH_1 {
        poseidon_1(tmp_storage_pda_data)?;
    } else if id == HASH_2 {
        poseidon_2(tmp_storage_pda_data)?;
        // Updating the current level hash after a new hash is completely computed.
        if tmp_storage_pda_data.current_level < MERKLE_TREE_HEIGHT {
            insert_1_inner_loop(merkle_tree_pda_data, tmp_storage_pda_data)?;
        }
    } else if id == MERKLE_TREE_UPDATE_START {
        insert_0_double(merkle_tree_pda_data, tmp_storage_pda_data)?;
    }
    Ok(())
}



pub fn pubkey_check(
    account_pubkey0: Pubkey,
    account_pubkey1: Pubkey,
    msg: String,
) -> Result<()>  {
    if account_pubkey0 != account_pubkey1 {
        msg!(&msg);
        return err!(ErrorCode::PubkeyCheckFailed);
    }

    Ok(())
}

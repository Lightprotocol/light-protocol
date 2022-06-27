use anchor_lang::prelude::*;
use crate::instructions::{close_account, sol_transfer};
use crate::poseidon_merkle_tree::processor::{
    compute_updated_merkle_tree,
};
use crate::utils::constants::{
    MERKLE_TREE_UPDATE_START,
    MERKLE_TREE_UPDATE_LEVEL,
    LOCK_START,
    HASH_0,
    HASH_1,
    HASH_2,
    ROOT_INSERT,
    IX_ORDER
};
use crate::utils::create_pda::create_and_check_pda;

use crate::utils::config;
use anchor_lang::solana_program::{
    account_info::{next_account_info, AccountInfo},
    msg,
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
    program_pack::Pack
};
use crate::errors::ErrorCode;
use crate::state::MerkleTree;
use crate::poseidon_merkle_tree::processor::pubkey_check;
use crate::MerkleTreeTmpPda;

#[derive(Accounts)]
pub struct UpdateMerkleTree<'info> {
    /// CHECK:` should be consistent
    #[account(mut, address=merkle_tree_tmp_storage.load()?.relayer)]
    pub authority: Signer<'info>,
    /// CHECK:` that merkle tree is locked for this account
    #[account(mut)]
    pub merkle_tree_tmp_storage: AccountLoader<'info ,MerkleTreeTmpPda>,
    /// CHECK:` that the merkle tree is whitelisted and consistent with merkle_tree_tmp_storage
    #[account(mut, constraint = merkle_tree.key() == Pubkey::new(&config::MERKLE_TREE_ACC_BYTES_ARRAY[merkle_tree_tmp_storage.load()?.merkle_tree_index as usize].0))]
    pub merkle_tree: AccountInfo<'info>,
}

#[allow(clippy::comparison_chain)]
pub fn process_update_merkle_tree(
    ctx: &mut Context<UpdateMerkleTree>,
) -> Result<()>{
    let tmp_storage_pda_data = ctx.accounts.merkle_tree_tmp_storage.load()?.clone();
    msg!("\n prior process_instruction {}\n",tmp_storage_pda_data.current_instruction_index );

    if tmp_storage_pda_data.current_instruction_index > 0
        && tmp_storage_pda_data.current_instruction_index < 56
    {
        let tmp_storage_pda_data = &mut ctx.accounts.merkle_tree_tmp_storage.load_mut()?;
        let mut merkle_tree_pda_data = MerkleTree::unpack(&ctx.accounts.merkle_tree.data.borrow())?;

        pubkey_check(
            ctx.accounts.merkle_tree_tmp_storage.key(),
            Pubkey::new(&merkle_tree_pda_data.pubkey_locked),
            String::from("Merkle tree locked by another account."),
        )?;

        msg!(
            "tmp_storage_pda_data.current_instruction_index0 {}",
            tmp_storage_pda_data.current_instruction_index
        );

        if tmp_storage_pda_data.current_instruction_index == 1 {
            compute_updated_merkle_tree(
                IX_ORDER[tmp_storage_pda_data.current_instruction_index as usize],
                tmp_storage_pda_data,
                &mut merkle_tree_pda_data,
            )?;
            tmp_storage_pda_data.current_instruction_index +=1;
        }

        msg!(
            "tmp_storage_pda_data.current_instruction_index1 {}",
            tmp_storage_pda_data.current_instruction_index
        );

        compute_updated_merkle_tree(
            IX_ORDER[tmp_storage_pda_data.current_instruction_index as usize],
            tmp_storage_pda_data,
            &mut merkle_tree_pda_data,
        )?;
        tmp_storage_pda_data.current_instruction_index +=1;
        // renews lock
        merkle_tree_pda_data.time_locked = <Clock as solana_program::sysvar::Sysvar>::get()?.slot;
        MerkleTree::pack_into_slice(
            &merkle_tree_pda_data,
            &mut ctx.accounts.merkle_tree.data.borrow_mut(),
        );
    }

    Ok(())
}

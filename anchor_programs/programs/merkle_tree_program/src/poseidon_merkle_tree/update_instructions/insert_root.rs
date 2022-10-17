use crate::errors::ErrorCode;
use crate::poseidon_merkle_tree::instructions::insert_last_double;
use crate::state::MerkleTree;
use crate::state::TwoLeavesBytesPda;
use crate::utils::config;
use crate::utils::constants::IX_ORDER;
use crate::utils::constants::ROOT_INSERT;
use crate::MerkleTreeUpdateState;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    account_info::AccountInfo, clock::Clock, msg, program_pack::Pack, pubkey::Pubkey,
    sysvar::Sysvar,
};
use std::borrow::BorrowMut;
use std::borrow::Borrow;

use crate::utils::constants::{
    STORAGE_SEED
};

#[derive(Accounts)]
pub struct InsertRoot<'info> {
    #[account(mut, address=merkle_tree_update_state.load()?.relayer)]
    pub authority: Signer<'info>,
    /// CHECK:` merkle_tree_update_state is derived correctly
    /// Merkle tree is locked by merkle_tree_update_state
    /// Is in correct instruction for root insert thus Merkle Tree update has been completed.
    /// The account is closed to the authority at the end of the instruction.
    #[account(
        mut,
        seeds = [&authority.key().to_bytes().as_ref(), STORAGE_SEED.as_ref()],
        bump,
        constraint= merkle_tree.load()?.pubkey_locked == merkle_tree_update_state.key(),
        constraint= IX_ORDER[usize::try_from(merkle_tree_update_state.load()?.current_instruction_index).unwrap()] == ROOT_INSERT @ErrorCode::MerkleTreeUpdateNotInRootInsert,
        close = authority
    )]
    pub merkle_tree_update_state: AccountLoader<'info, MerkleTreeUpdateState>,
    /// CHECK:` that the merkle tree is whitelisted and consistent with merkle_tree_update_state.
    #[account(mut)]
    pub merkle_tree: AccountLoader<'info, MerkleTree>,
}

pub fn process_insert_root<'a, 'b, 'c, 'info>(ctx: &mut Context<'a, 'b, 'c, 'info,InsertRoot<'info>>) -> Result<()> {
    let merkle_tree_update_state_data = &mut ctx.accounts.merkle_tree_update_state.load_mut()?;
    let mut merkle_tree_pda_data = &mut ctx.accounts.merkle_tree.load_mut()?;

    msg!(
        "Root insert Instruction: {}",
        IX_ORDER[usize::try_from(merkle_tree_update_state_data.current_instruction_index).unwrap()]
    );

    msg!(
        "merkle_tree_pda_data.pubkey_locked: {:?}",
        merkle_tree_pda_data.pubkey_locked
    );

    msg!(
        "ctx.accounts.merkle_tree_update_state.key(): {:?}",
        ctx.accounts.merkle_tree_update_state.key()
    );

    // insert root into merkle tree
    insert_last_double(&mut merkle_tree_pda_data, merkle_tree_update_state_data)?;

    // Release lock
    msg!("Lock set at slot: {}", merkle_tree_pda_data.time_locked);
    msg!("Lock released at slot: {}", <Clock as Sysvar>::get()?.slot);
    merkle_tree_pda_data.time_locked = 0;
    merkle_tree_pda_data.pubkey_locked = Pubkey::new(&[0; 32]);

    /*
    // not necessary since we are already checking that the index of a leaves account is greater
    // than the index of the merkle tree account which means the account is not part of the tree

    // mark leaves as inserted
    // check that leaves are the same as in first tx
    for (index, account) in ctx.remaining_accounts.iter().enumerate() {
        msg!("Checking leaves pair {}", index);
        // let mut leaves_pda_data = TwoLeavesBytesPda::deserialize(&mut &**account.to_account_info().try_borrow_mut_data().unwrap())?;
        let leaves_pda_data:  &mut Account<'info, TwoLeavesBytesPda> = &mut Account::try_from(account)?;

        if index >= merkle_tree_update_state_data.number_of_leaves.into() {
            msg!(
                "Submitted to many remaining accounts {}",
                ctx.remaining_accounts.len()
            );
            return err!(ErrorCode::WrongLeavesLastTx);
        }
        if merkle_tree_update_state_data.leaves[index][0][..] != leaves_pda_data.node_left {
            msg!("Wrong leaf in position {}", index);
            return err!(ErrorCode::WrongLeavesLastTx);
        }
        if  account.owner != ctx.program_id {
            msg!("Wrong owner {}", index);
            return err!(ErrorCode::WrongLeavesLastTx);
        }

        if leaves_pda_data.is_inserted {
            msg!(
                "Leaf pda with address {:?} is already inserted",
                *account.key
            );
            return err!(ErrorCode::LeafAlreadyInserted);
        }
        // Checking that the Merkle tree is the same as in leaves account.
        if leaves_pda_data.merkle_tree_pubkey != ctx.accounts.merkle_tree.key() {
            msg!(
                "Leaf pda state {} with address {:?} is already inserted",
                leaves_pda_data.merkle_tree_pubkey,
                ctx.accounts.merkle_tree.key()
            );
            return err!(ErrorCode::LeafAlreadyInserted);
        }
        // msg!(
        //     "account.data.borrow_mut()[1] {}",
        //     account.data.borrow_mut()[1]
        // );
        // mark leaves pda as inserted
        leaves_pda_data.is_inserted = true;
        // let data = TwoLeavesBytesPda::serialize(&leaves_pda_data, &mut account.data.get_mut())?;
        // let mut mut_leaves_pda =  account.data.borrow_mut()?;
        // mut_leaves_pda = data;
        // TwoLeavesBytesPda::pack_into_slice(
        //     &leaves_pda_data,
        //     &mut account.data.borrow_mut(),
        // );
    }
    */


    Ok(())
}

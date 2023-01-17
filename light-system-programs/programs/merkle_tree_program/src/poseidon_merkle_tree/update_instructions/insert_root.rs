use crate::errors::ErrorCode;
use crate::poseidon_merkle_tree::{instructions::insert_last_double, state::TwoLeavesBytesPda};
use crate::state::MerkleTree;
use crate::utils::constants::{IX_ORDER, ROOT_INSERT, STORAGE_SEED};
use crate::MerkleTreeUpdateState;

use std::ops::DerefMut;

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    clock::Clock, msg, program::invoke, pubkey::Pubkey, sysvar::Sysvar,
};
use spl_account_compression::Noop;

#[derive(Accounts)]
pub struct InsertRoot<'info> {
    #[account(mut, address=merkle_tree_update_state.load()?.relayer @ErrorCode::InvalidAuthority)]
    pub authority: Signer<'info>,
    /// CHECK:` merkle_tree_update_state is derived correctly
    /// Merkle tree is locked by merkle_tree_update_state
    /// Is in correct instruction for root insert thus Merkle Tree update has been completed.
    /// The account is closed to the authority at the end of the instruction.
    #[account(
        mut,
        seeds = [authority.key().to_bytes().as_ref(), STORAGE_SEED],
        bump,
        constraint= merkle_tree.load()?.pubkey_locked == merkle_tree_update_state.key() @ErrorCode::ContractStillLocked,
        constraint= IX_ORDER[usize::try_from(merkle_tree_update_state.load()?.current_instruction_index).unwrap()] == ROOT_INSERT @ErrorCode::MerkleTreeUpdateNotInRootInsert,
        close = authority
    )]
    /// CHECK: we need to check it's a right recipient account.
    // pub recipient: AccountInfo<'info>,
    pub merkle_tree_update_state: AccountLoader<'info, MerkleTreeUpdateState>,
    /// CHECK:` that the merkle tree is whitelisted and consistent with merkle_tree_update_state.
    #[account(mut, address= merkle_tree_update_state.load()?.merkle_tree_pda_pubkey @ErrorCode::InvalidMerkleTree)]
    pub merkle_tree: AccountLoader<'info, MerkleTree>,
    pub log_wrapper: Program<'info, Noop>,
    pub system_program: Program<'info, System>,
}

pub fn wrap_event<'info>(
    event: &TwoLeavesBytesPda,
    noop_program: &AccountInfo<'info>,
    signer: &AccountInfo<'info>,
) -> Result<()> {
    invoke(
        &spl_noop::instruction(event.try_to_vec()?),
        &[noop_program.to_account_info(), signer.to_account_info()],
    )?;
    Ok(())
}

pub fn close_account(account: &AccountInfo, dest_account: &AccountInfo) -> Result<()> {
    //close account by draining lamports
    let dest_starting_lamports = dest_account.lamports();
    **dest_account.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(account.lamports())
        .ok_or(ErrorCode::CloseAccountFailed)?;
    **account.lamports.borrow_mut() = 0;
    let mut data = account.try_borrow_mut_data()?;
    for byte in data.deref_mut().iter_mut() {
        *byte = 0;
    }
    Ok(())
}

pub fn process_insert_root<'a, 'b, 'c, 'info>(
    ctx: &mut Context<'a, 'b, 'c, 'info, InsertRoot<'info>>,
) -> Result<()> {
    let merkle_tree_update_state_data = &mut ctx.accounts.merkle_tree_update_state.load_mut()?;
    let merkle_tree_pda_data = &mut ctx.accounts.merkle_tree.load_mut()?;

    let id =
        IX_ORDER[usize::try_from(merkle_tree_update_state_data.current_instruction_index).unwrap()];
    msg!("Root insert Instruction: {}", id);

    msg!(
        "merkle_tree_pda_data.pubkey_locked: {:?}",
        merkle_tree_pda_data.pubkey_locked
    );

    msg!(
        "ctx.accounts.merkle_tree_update_state.key(): {:?}",
        ctx.accounts.merkle_tree_update_state.key()
    );

    let mut tmp_index = merkle_tree_pda_data.next_index;
    msg!("tmp_index: {}", tmp_index);

    // insert root into merkle tree

    // Release lock
    msg!("Lock set at slot: {}", merkle_tree_pda_data.time_locked);
    msg!("Lock released at slot: {}", <Clock as Sysvar>::get()?.slot);
    merkle_tree_pda_data.time_locked = 0;
    merkle_tree_pda_data.pubkey_locked = Pubkey::new(&[0; 32]);

    msg!("start loop");
    // Leaves are passed in as pdas in remaining accounts to allow for flexibility in their
    // number.
    // Checks are:
    //             - are not inserted yet
    //             - belong to merkle_tree
    //             - the lowest index is the next index of the merkle_tree
    //             - indices increase incrementally by 2 for subsequent leaves
    // Copying leaves to tmp account.
    for (index, account) in ctx.remaining_accounts.iter().enumerate() {
        msg!("Copying leaves pair {}", index);

        let leaves_pda_data: Account<'info, TwoLeavesBytesPda> = Account::try_from(account)?;

        // Checking that leaves are not inserted already.
        if leaves_pda_data.left_leaf_index < merkle_tree_pda_data.next_index {
            msg!(
                "Leaf pda state with address {:?} is already inserted",
                *account.key
            );
            return err!(ErrorCode::LeafAlreadyInserted);
        }

        // Checking that the Merkle tree is the same as in leaves account.
        if leaves_pda_data.merkle_tree_pubkey != ctx.accounts.merkle_tree.key() {
            msg!(
                "Leaf pda state merkle tree {} is different than passed in merkle tree {:?}",
                leaves_pda_data.merkle_tree_pubkey,
                ctx.accounts.merkle_tree.key()
            );
            return err!(ErrorCode::LeavesOfWrongTree);
        }

        // Checking that the lowest index is the next index of the merkle_tree.
        // Check that following leaves are correct and in the right order.
        if leaves_pda_data.left_leaf_index != tmp_index {
            return err!(ErrorCode::FirstLeavesPdaIncorrectIndex);
        }

        // Save pair of leaves in compressed account (noop program).
        wrap_event(
            &leaves_pda_data,
            &ctx.accounts.log_wrapper.to_account_info(),
            &ctx.accounts.merkle_tree.to_account_info(),
        )?;

        close_account(account, &ctx.accounts.authority.to_account_info())?;

        tmp_index += 2;
    }

    insert_last_double(merkle_tree_pda_data, merkle_tree_update_state_data)?;

    Ok(())
}

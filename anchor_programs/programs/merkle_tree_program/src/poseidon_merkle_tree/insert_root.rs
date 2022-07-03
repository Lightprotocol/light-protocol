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

use crate::utils::constants::{
    LEAVES_PDA_ACCOUNT_TYPE, STORAGE_SEED, UNINSERTED_LEAVES_PDA_ACCOUNT_TYPE,
};

#[derive(Accounts)]
pub struct InsertRoot<'info> {
    #[account(mut, address=merkle_tree_update_state.load()?.relayer)]
    pub authority: Signer<'info>,
    /// CHECK:`  and
    /// merkle_tree_update_state is derived correctly
    /// Merkle tree is locked by merkle_tree_update_state
    /// Is in correct instruction for root insert thus Merkle Tree update has been completed.
    /// The account is closed to the authority at the end of the instruction.
    #[account(
        mut,
        seeds = [&authority.key().to_bytes().as_ref(), STORAGE_SEED.as_ref()],
        bump,
        constraint= Pubkey::new(&merkle_tree.data.borrow()[16658-40..16658-8]) == merkle_tree_update_state.key(),
        constraint= IX_ORDER[merkle_tree_update_state.load()?.current_instruction_index as usize] == ROOT_INSERT @ErrorCode::MerkleTreeUpdateNotInRootInsert,
        close = authority
    )]
    pub merkle_tree_update_state: AccountLoader<'info, MerkleTreeUpdateState>,
    /// CHECK:` that the merkle tree is whitelisted and consistent with merkle_tree_update_state.
    #[account(mut, constraint = merkle_tree.key() == Pubkey::new(&config::MERKLE_TREE_ACC_BYTES_ARRAY[merkle_tree_update_state.load()?.merkle_tree_index as usize].0))]
    pub merkle_tree: AccountInfo<'info>,
}

pub fn process_insert_root(ctx: &mut Context<InsertRoot>) -> Result<()> {
    let merkle_tree_update_state_data = &mut ctx.accounts.merkle_tree_update_state.load_mut()?;

    msg!(
        "Root insert Instruction: {}",
        IX_ORDER[merkle_tree_update_state_data.current_instruction_index as usize]
    );
    //  moved to constraint
    //  if merkle_tree_update_state_data.current_instruction_index != 56 {
    //      msg!("Wrong state instruction index should be 56 is {}", merkle_tree_update_state_data.current_instruction_index);
    // }
    //
    // if IX_ORDER[merkle_tree_update_state_data.current_instruction_index as usize] != ROOT_INSERT {
    //     msg!("Merkle Tree update not completed yet, cannot insert root.");
    //     return err!(ErrorCode::MerkleTreeUpdateNotInRootInsert);
    // }

    let mut merkle_tree_pda_data = MerkleTree::unpack(&ctx.accounts.merkle_tree.data.borrow())?;

    msg!(
        "Pubkey::new(&merkle_tree_pda_data.pubkey_locked): {:?}",
        Pubkey::new(&merkle_tree_pda_data.pubkey_locked)
    );
    msg!(
        "ctx.accounts.merkle_tree_update_state.key(): {:?}",
        ctx.accounts.merkle_tree_update_state.key()
    );

    // Checking if signer locked moved to constraints
    // pubkey_check(
    //     ctx.accounts.merkle_tree_update_state.key(),
    //     Pubkey::new(&merkle_tree_pda_data.pubkey_locked),
    //     String::from("Merkle tree locked by another account."),
    // )?;

    // insert root into merkle tree
    insert_last_double(&mut merkle_tree_pda_data, merkle_tree_update_state_data)?;

    // Release lock
    msg!("Lock set at slot: {}", merkle_tree_pda_data.time_locked);
    msg!("Lock released at slot: {}", <Clock as Sysvar>::get()?.slot);
    merkle_tree_pda_data.time_locked = 0;
    merkle_tree_pda_data.pubkey_locked = vec![0; 32];

    // mark leaves as inserted
    // check that leaves are the same as in first tx
    for (index, account) in ctx.remaining_accounts.iter().enumerate() {
        msg!("Checking leaves pair {}", index);
        let leaves_pda_data = TwoLeavesBytesPda::unpack(&account.data.borrow())?;
        if index >= merkle_tree_update_state_data.number_of_leaves.into() {
            msg!(
                "Submitted to many remaining accounts {}",
                ctx.remaining_accounts.len()
            );
            return err!(ErrorCode::WrongLeavesLastTx);
        }
        if merkle_tree_update_state_data.leaves[index][0][..] != account.data.borrow()[10..42] {
            msg!("Wrong leaf in position {}", index);
            return err!(ErrorCode::WrongLeavesLastTx);
        }
        if account.data.borrow()[1] != UNINSERTED_LEAVES_PDA_ACCOUNT_TYPE {
            msg!(
                "Leaf pda with address {:?} is already inserted",
                *account.key
            );
            return err!(ErrorCode::LeafAlreadyInserted);
        }
        // Checking that the Merkle tree is the same as in leaves account.
        if Pubkey::new(&leaves_pda_data.merkle_tree_pubkey) != ctx.accounts.merkle_tree.key() {
            msg!(
                "Leaf pda state {} with address {:?} is already inserted",
                Pubkey::new(&leaves_pda_data.merkle_tree_pubkey),
                ctx.accounts.merkle_tree.key()
            );
            return err!(ErrorCode::LeafAlreadyInserted);
        }
        msg!(
            "account.data.borrow_mut()[1] {}",
            account.data.borrow_mut()[1]
        );
        // mark leaves pda as inserted
        account.data.borrow_mut()[1] = LEAVES_PDA_ACCOUNT_TYPE;
    }

    MerkleTree::pack_into_slice(
        &merkle_tree_pda_data,
        &mut ctx.accounts.merkle_tree.data.borrow_mut(),
    );

    Ok(())
}

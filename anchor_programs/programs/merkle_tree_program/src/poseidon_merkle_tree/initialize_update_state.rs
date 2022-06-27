use anchor_lang::prelude::*;
use crate::state::MerkleTree;
use crate::utils::constants::UNINSERTED_LEAVES_PDA_ACCOUNT_TYPE;
use crate::state::TwoLeavesBytesPda;
use crate::utils::config::MERKLE_TREE_TMP_PDA_SIZE;
use crate::MerkleTreeTmpPda;
use crate::utils::constants::STORAGE_SEED;
use crate::utils::config;
use crate::errors::ErrorCode;

use anchor_lang::solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};
#[derive(Accounts)]
#[instruction(merkle_tree_index: u64)]
pub struct InitializeUpdateState<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK:`
    #[account(
        init,
        seeds = [&authority.key().to_bytes().as_ref(), STORAGE_SEED.as_ref()],
        bump,
        payer = authority,
        space = MERKLE_TREE_TMP_PDA_SIZE + 64 * 20,
    )]
    pub merkle_tree_tmp_storage: AccountLoader<'info ,MerkleTreeTmpPda>,
    /// CHECK: that the merkle tree is whitelisted
    #[account(mut, constraint = merkle_tree.key() == Pubkey::new(&config::MERKLE_TREE_ACC_BYTES_ARRAY[merkle_tree_index as usize].0))]
    pub merkle_tree: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}


pub fn process_initialize_update_state(
        ctx: Context<InitializeUpdateState>,
        merkle_tree_index: u64
    ) -> Result<()> {
    msg!("InitializeUpdateState");

    // TODO check merkle tree index if not already done in contraints

    let tmp_storage_pda = &mut ctx.accounts.merkle_tree_tmp_storage.load_init()?;
    //increased by 2 because we're inserting 2 leaves at once
    tmp_storage_pda.merkle_tree_index = merkle_tree_index.try_into().unwrap();
    tmp_storage_pda.relayer = ctx.accounts.authority.key();
    tmp_storage_pda.merkle_tree_pda_pubkey = ctx.accounts.merkle_tree.key();

    tmp_storage_pda.current_instruction_index = 1;


    // Checking that the number of remaining accounts is non zero and smaller than 16.
    if ctx.remaining_accounts.len() == 0 || ctx.remaining_accounts.len() > 16 {
        msg!("Submitted number of leaves: {}", ctx.remaining_accounts.len());
        return err!(ErrorCode::InvalidNumberOfLeaves);
    }

    let mut merkle_tree_pda_data = MerkleTree::unpack(&ctx.accounts.merkle_tree.data.borrow())?;
    tmp_storage_pda.tmp_leaves_index = merkle_tree_pda_data.next_index.try_into().unwrap();


    let mut tmp_index = merkle_tree_pda_data.next_index;

    // Copying leaves to tmp account.
    for (index, account) in ctx.remaining_accounts.iter().enumerate() {
        msg!("Copying leaves pair {}", index);
        let leaves_pda_data = TwoLeavesBytesPda::unpack(&account.data.borrow())?;

        // Checking that leaves are not inserted already.
        if leaves_pda_data.account_type != UNINSERTED_LEAVES_PDA_ACCOUNT_TYPE {
            msg!("Leaf pda state {} with address {:?} is already inserted",account.data.borrow()[1], *account.key);
            return err!(ErrorCode::LeafAlreadyInserted);
        }

        // Checking that index is correct.
        if index == 0 &&
            leaves_pda_data.left_leaf_index != merkle_tree_pda_data.next_index

         {
             msg!("Leaves pda at index {} has index {} but should have {}", index,
                leaves_pda_data.left_leaf_index,
                merkle_tree_pda_data.next_index
            );
             return err!(ErrorCode::FirstLeavesPdaIncorrectIndex);
        }

        // Check that following leaves are correct and in the right order.
        else if leaves_pda_data.left_leaf_index != tmp_index {
            return err!(ErrorCode::FirstLeavesPdaIncorrectIndex);
        }
        // Copy leaves to tmp account.
        tmp_storage_pda.leaves[index][0] = leaves_pda_data.node_left.try_into().unwrap();
        tmp_storage_pda.leaves[index][1] = leaves_pda_data.node_right.try_into().unwrap();
        msg!("tmp_storage.leaves[index][0] {:?}", tmp_storage_pda.leaves[index][0]);
        msg!("tmp_storage.leaves[index][1] {:?}", tmp_storage_pda.leaves[index][1]);
        tmp_storage_pda.number_of_leaves = (index + 1).try_into().unwrap();
        tmp_index +=2;
    }

    // let ctx = get_lock(ctx)?;

    let current_slot = <Clock as  solana_program::sysvar::Sysvar>::get()?.slot;
    msg!("Current slot: {:?}", current_slot);

    msg!("Locked at slot: {}", merkle_tree_pda_data.time_locked);
    msg!(
        "Lock ends at slot: {}",
        merkle_tree_pda_data.time_locked + config::LOCK_DURATION
    );

    //lock
    if merkle_tree_pda_data.time_locked == 0
        || merkle_tree_pda_data.time_locked + config::LOCK_DURATION < current_slot
    {
        merkle_tree_pda_data.time_locked = <Clock as solana_program::sysvar::Sysvar>::get()?.slot;
        merkle_tree_pda_data.pubkey_locked = ctx.accounts.merkle_tree_tmp_storage.key().to_bytes().to_vec();
        msg!("Locked at slot: {}", merkle_tree_pda_data.time_locked);
        msg!(
            "Locked by: {:?}",
            Pubkey::new(&merkle_tree_pda_data.pubkey_locked)
        );
    } else if merkle_tree_pda_data.time_locked + config::LOCK_DURATION > current_slot {
        msg!("Contract is still locked.");
        return err!(ErrorCode::ContractStillLocked);
    } else {
        merkle_tree_pda_data.time_locked = <Clock as solana_program::sysvar::Sysvar>::get()?.slot;
        merkle_tree_pda_data.pubkey_locked = ctx.accounts.merkle_tree_tmp_storage.key().to_bytes().to_vec();
    }

    MerkleTree::pack_into_slice(
        &merkle_tree_pda_data,
        &mut ctx.accounts.merkle_tree.data.borrow_mut(),
    );
    Ok(())
}

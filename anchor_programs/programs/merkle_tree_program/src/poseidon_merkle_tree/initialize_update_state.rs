use crate::errors::ErrorCode;
use crate::state::MerkleTree;
use crate::state::TwoLeavesBytesPda;
use crate::utils::config;
use crate::utils::config::MERKLE_TREE_TMP_PDA_SIZE;
use crate::utils::constants::STORAGE_SEED;
use crate::utils::constants::UNINSERTED_LEAVES_PDA_ACCOUNT_TYPE;
use crate::MerkleTreeUpdateState;
use anchor_lang::prelude::*;

use anchor_lang::solana_program::{msg, program_pack::Pack};

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
    pub merkle_tree_update_state: AccountLoader<'info, MerkleTreeUpdateState>,
    /// CHECK: that the merkle tree is registered.
    #[account(mut, constraint = merkle_tree.key() == Pubkey::new(&config::MERKLE_TREE_ACC_BYTES_ARRAY[merkle_tree_index as usize].0))]
    pub merkle_tree: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn process_initialize_update_state(
    ctx: Context<InitializeUpdateState>,
    merkle_tree_index: u64,
) -> Result<()> {
    msg!("InitializeUpdateState");

    let verifier_state_data = &mut ctx.accounts.merkle_tree_update_state.load_init()?;
    verifier_state_data.merkle_tree_index = merkle_tree_index.try_into().unwrap();
    verifier_state_data.relayer = ctx.accounts.authority.key();
    verifier_state_data.merkle_tree_pda_pubkey = ctx.accounts.merkle_tree.key();

    verifier_state_data.current_instruction_index = 1;

    // Checking that the number of remaining accounts is non zero and smaller than 16.
    if ctx.remaining_accounts.len() == 0 || ctx.remaining_accounts.len() > 16 {
        msg!(
            "Submitted number of leaves: {}",
            ctx.remaining_accounts.len()
        );
        return err!(ErrorCode::InvalidNumberOfLeaves);
    }

    let mut merkle_tree_pda_data = MerkleTree::unpack(&ctx.accounts.merkle_tree.data.borrow())?;
    verifier_state_data.tmp_leaves_index = merkle_tree_pda_data.next_index.try_into().unwrap();

    let mut tmp_index = merkle_tree_pda_data.next_index;
    // Leaves are passed in as pdas in remaining accounts to allow for flexibility in their
    // number.
    // Checks are:
    //             - are not inserted yet
    //             - belong to merkle_tree
    //             - the lowest index is the next index of the merkle_tree
    //             - indices increases incrementally by 2 for subsequent leaves
    // Copying leaves to tmp account.
    for (index, account) in ctx.remaining_accounts.iter().enumerate() {
        msg!("Copying leaves pair {}", index);
        let leaves_pda_data = TwoLeavesBytesPda::unpack(&account.data.borrow())?;

        // Checking that leaves are not inserted already.
        if leaves_pda_data.account_type != UNINSERTED_LEAVES_PDA_ACCOUNT_TYPE {
            msg!(
                "Leaf pda state {} with address {:?} is already inserted",
                leaves_pda_data.account_type,
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

        // Checking that index is correct.
        if index == 0 && leaves_pda_data.left_leaf_index != merkle_tree_pda_data.next_index {
            msg!(
                "Leaves pda at index {} has index {} but should have {}",
                index,
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
        verifier_state_data.leaves[index][0] = leaves_pda_data.node_left.try_into().unwrap();
        verifier_state_data.leaves[index][1] = leaves_pda_data.node_right.try_into().unwrap();
        verifier_state_data.number_of_leaves = (index + 1).try_into().unwrap();
        tmp_index += 2;
    }

    // Get Merkle tree lock with update state account.
    // The lock lasts config::LOCK_DURATION and is renewed every transaction.

    let current_slot = <Clock as solana_program::sysvar::Sysvar>::get()?.slot;
    msg!("Current slot: {:?}", current_slot);

    msg!("Locked at slot: {}", merkle_tree_pda_data.time_locked);
    msg!(
        "Lock ends at slot: {}",
        merkle_tree_pda_data.time_locked + config::LOCK_DURATION
    );

    if merkle_tree_pda_data.time_locked == 0
        || merkle_tree_pda_data.time_locked + config::LOCK_DURATION < current_slot
    {
        merkle_tree_pda_data.time_locked = <Clock as solana_program::sysvar::Sysvar>::get()?.slot;
        merkle_tree_pda_data.pubkey_locked = ctx
            .accounts
            .merkle_tree_update_state
            .key()
            .to_bytes()
            .to_vec();
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
        merkle_tree_pda_data.pubkey_locked = ctx
            .accounts
            .merkle_tree_update_state
            .key()
            .to_bytes()
            .to_vec();
    }

    MerkleTree::pack_into_slice(
        &merkle_tree_pda_data,
        &mut ctx.accounts.merkle_tree.data.borrow_mut(),
    );
    Ok(())
}

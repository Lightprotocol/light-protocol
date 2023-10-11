use anchor_lang::{
    prelude::*,
    solana_program::{msg, sysvar},
};

use crate::{
    errors::ErrorCode,
    transaction_merkle_tree::state::{TransactionMerkleTree, TwoLeavesBytesPda},
    utils::{config::MERKLE_TREE_TMP_PDA_SIZE, constants::STORAGE_SEED},
    MerkleTreeUpdateState,
};

#[derive(Accounts)]
pub struct InitializeUpdateState<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK:`
    #[account(
        init,
        seeds = [authority.key().to_bytes().as_ref(), STORAGE_SEED],
        bump,
        payer = authority,
        space = MERKLE_TREE_TMP_PDA_SIZE,
    )]
    pub merkle_tree_update_state: AccountLoader<'info, MerkleTreeUpdateState>,
    #[account(mut)]
    pub transaction_merkle_tree: AccountLoader<'info, TransactionMerkleTree>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn process_initialize_update_state<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, InitializeUpdateState<'info>>,
) -> Result<()> {
    msg!("InitializeUpdateState");
    let update_state_data = &mut ctx.accounts.merkle_tree_update_state.load_init()?;
    update_state_data.relayer = ctx.accounts.authority.key();
    update_state_data.merkle_tree_pda_pubkey = ctx.accounts.transaction_merkle_tree.key();

    update_state_data.current_instruction_index = 1;

    // Checking that the number of remaining accounts is non zero and smaller than 16.
    if ctx.remaining_accounts.is_empty() || ctx.remaining_accounts.len() > 16 {
        msg!(
            "Submitted number of leaves: {}",
            ctx.remaining_accounts.len()
        );
        return err!(ErrorCode::InvalidNumberOfLeaves);
    }

    let mut merkle_tree_pda_data = ctx.accounts.transaction_merkle_tree.load_mut()?;

    let mut tmp_index = merkle_tree_pda_data.next_index;
    msg!("tmp_index: {}", tmp_index);
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
        if leaves_pda_data.merkle_tree_pubkey != ctx.accounts.transaction_merkle_tree.key() {
            msg!(
                "Leaf pda state merkle tree {} is different than passed in merkle tree {:?}",
                leaves_pda_data.merkle_tree_pubkey,
                ctx.accounts.transaction_merkle_tree.key()
            );
            return err!(ErrorCode::LeavesOfWrongTree);
        }

        // Checking that the lowest index is the next index of the merkle_tree.
        // Check that following leaves are correct and in the right order.
        if leaves_pda_data.left_leaf_index != tmp_index {
            return err!(ErrorCode::FirstLeavesPdaIncorrectIndex);
        }
        // Copy leaves to tmp account.
        update_state_data.leaves[index][0] = leaves_pda_data.node_left;
        update_state_data.leaves[index][1] = leaves_pda_data.node_right;
        update_state_data.number_of_leaves = (index + 1).try_into().unwrap();
        tmp_index += 2;
    }

    // Get Merkle tree lock with update state account.
    // The lock lasts merkle_tree_pda_data.lock_duration and is renewed every transaction.

    let current_slot = <Clock as sysvar::Sysvar>::get()?.slot;
    msg!("Current slot: {:?}", current_slot);

    msg!("Locked at slot: {}", merkle_tree_pda_data.time_locked);
    msg!(
        "Lock ends at slot: {}",
        merkle_tree_pda_data.time_locked + merkle_tree_pda_data.lock_duration
    );

    if merkle_tree_pda_data.time_locked == 0
        || merkle_tree_pda_data.time_locked + merkle_tree_pda_data.lock_duration < current_slot
    {
        merkle_tree_pda_data.time_locked = current_slot;
        merkle_tree_pda_data.pubkey_locked = ctx.accounts.merkle_tree_update_state.key();
        msg!("Locked at slot: {}", merkle_tree_pda_data.time_locked);
        msg!("Locked by: {:?}", merkle_tree_pda_data.pubkey_locked);
    } else {
        msg!("Contract is still locked.");
        return err!(ErrorCode::ContractStillLocked);
    }

    // Copying Subtrees into update state.
    update_state_data.filled_subtrees = merkle_tree_pda_data.filled_subtrees;
    update_state_data.tmp_leaves_index = merkle_tree_pda_data.next_index;

    Ok(())
}

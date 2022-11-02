use crate::errors::ErrorCode;
use crate::poseidon_merkle_tree::instructions::insert_last_double;
use crate::state::MerkleTree;
use crate::utils::constants::{IX_ORDER, ROOT_INSERT, STORAGE_SEED};

use crate::MerkleTreeUpdateState;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{clock::Clock, msg, pubkey::Pubkey, sysvar::Sysvar};

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
    pub merkle_tree_update_state: AccountLoader<'info, MerkleTreeUpdateState>,
    /// CHECK:` that the merkle tree is whitelisted and consistent with merkle_tree_update_state.
    #[account(mut, address= merkle_tree_update_state.load()?.merkle_tree_pda_pubkey @ErrorCode::InvalidMerkleTree)]
    pub merkle_tree: AccountLoader<'info, MerkleTree>,
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

    // insert root into merkle tree
    insert_last_double(merkle_tree_pda_data, merkle_tree_update_state_data)?;

    // Release lock
    msg!("Lock set at slot: {}", merkle_tree_pda_data.time_locked);
    msg!("Lock released at slot: {}", <Clock as Sysvar>::get()?.slot);
    merkle_tree_pda_data.time_locked = 0;
    merkle_tree_pda_data.pubkey_locked = Pubkey::new(&[0; 32]);

    Ok(())
}

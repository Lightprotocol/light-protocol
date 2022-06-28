use crate::poseidon_merkle_tree::processor::compute_updated_merkle_tree;
use crate::poseidon_merkle_tree::processor::pubkey_check;
use crate::state::MerkleTree;
use crate::utils::config;
use crate::utils::constants::{IX_ORDER, STORAGE_SEED};
use crate::MerkleTreeUpdateState;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    account_info::AccountInfo, msg, program_pack::Pack, pubkey::Pubkey,
};

#[derive(Accounts)]
pub struct UpdateMerkleTree<'info> {
    /// CHECK:` should be consistent
    #[account(mut, address=merkle_tree_update_state.load()?.relayer)]
    pub authority: Signer<'info>,
    /// CHECK:` that merkle tree is locked for this account
    #[account(mut, seeds = [&authority.key().to_bytes().as_ref(), STORAGE_SEED.as_ref()], bump,
        constraint= Pubkey::new(&merkle_tree.data.borrow()[16658-40..16658-8]) == merkle_tree_update_state.key()
    )]
    pub merkle_tree_update_state: AccountLoader<'info, MerkleTreeUpdateState>,
    /// CHECK:` that the merkle tree is whitelisted and consistent with merkle_tree_update_state
    #[account(mut, constraint = merkle_tree.key() == Pubkey::new(&config::MERKLE_TREE_ACC_BYTES_ARRAY[merkle_tree_update_state.load()?.merkle_tree_index as usize].0))]
    pub merkle_tree: AccountInfo<'info>,
}

#[allow(clippy::comparison_chain)]
pub fn process_update_merkle_tree(ctx: &mut Context<UpdateMerkleTree>) -> Result<()> {
    let merkle_tree_update_state_data = ctx.accounts.merkle_tree_update_state.load()?.clone();
    msg!(
        "\n prior process_instruction {}\n",
        merkle_tree_update_state_data.current_instruction_index
    );

    if merkle_tree_update_state_data.current_instruction_index > 0
        && merkle_tree_update_state_data.current_instruction_index < 56
    {
        let merkle_tree_update_state_data =
            &mut ctx.accounts.merkle_tree_update_state.load_mut()?;
        let mut merkle_tree_pda_data = MerkleTree::unpack(&ctx.accounts.merkle_tree.data.borrow())?;

        pubkey_check(
            ctx.accounts.merkle_tree_update_state.key(),
            Pubkey::new(&merkle_tree_pda_data.pubkey_locked),
            String::from("Merkle tree locked by another account."),
        )?;

        msg!(
            "merkle_tree_update_state_data.current_instruction_index0 {}",
            merkle_tree_update_state_data.current_instruction_index
        );

        if merkle_tree_update_state_data.current_instruction_index == 1 {
            compute_updated_merkle_tree(
                IX_ORDER[merkle_tree_update_state_data.current_instruction_index as usize],
                merkle_tree_update_state_data,
                &mut merkle_tree_pda_data,
            )?;
            merkle_tree_update_state_data.current_instruction_index += 1;
        }

        msg!(
            "merkle_tree_update_state_data.current_instruction_index1 {}",
            merkle_tree_update_state_data.current_instruction_index
        );

        compute_updated_merkle_tree(
            IX_ORDER[merkle_tree_update_state_data.current_instruction_index as usize],
            merkle_tree_update_state_data,
            &mut merkle_tree_pda_data,
        )?;
        merkle_tree_update_state_data.current_instruction_index += 1;
        // renews lock
        merkle_tree_pda_data.time_locked = <Clock as solana_program::sysvar::Sysvar>::get()?.slot;
        MerkleTree::pack_into_slice(
            &merkle_tree_pda_data,
            &mut ctx.accounts.merkle_tree.data.borrow_mut(),
        );
    }

    Ok(())
}

use anchor_lang::{
    prelude::*,
    solana_program::{msg, pubkey::Pubkey, sysvar},
};

use crate::{
    errors::ErrorCode,
    transaction_merkle_tree::{
        processor::{compute_updated_merkle_tree, pubkey_check},
        state::TransactionMerkleTree,
    },
    utils::constants::{IX_ORDER, STORAGE_SEED},
    MerkleTreeUpdateState,
};

#[derive(Accounts)]
pub struct UpdateTransactionMerkleTree<'info> {
    /// CHECK:` should be consistent
    #[account(mut, address=merkle_tree_update_state.load()?.relayer @ErrorCode::InvalidAuthority)]
    pub authority: Signer<'info>,
    /// CHECK:` that merkle tree is locked for this account
    #[account(mut, seeds = [authority.key().to_bytes().as_ref(), STORAGE_SEED], bump,
        constraint=transaction_merkle_tree.load()?.pubkey_locked == merkle_tree_update_state.key() @ErrorCode::ContractStillLocked
    )]
    pub merkle_tree_update_state: AccountLoader<'info, MerkleTreeUpdateState>,
    /// CHECK:` that the merkle tree is whitelisted and consistent with merkle_tree_update_state
    #[account(mut, address= merkle_tree_update_state.load()?.merkle_tree_pda_pubkey @ErrorCode::InvalidMerkleTree)]
    pub transaction_merkle_tree: AccountLoader<'info, TransactionMerkleTree>,
}

#[allow(clippy::comparison_chain)]
pub fn process_update_merkle_tree(ctx: &mut Context<UpdateTransactionMerkleTree>) -> Result<()> {
    let mut merkle_tree_update_state_data = ctx.accounts.merkle_tree_update_state.load_mut()?;
    let mut merkle_tree_pda_data = ctx.accounts.transaction_merkle_tree.load_mut()?;

    msg!(
        "\n prior process_instruction {}\n",
        merkle_tree_update_state_data.current_instruction_index
    );

    if merkle_tree_update_state_data.current_instruction_index > 0
        && merkle_tree_update_state_data.current_instruction_index < 56
    {
        pubkey_check(
            ctx.accounts.merkle_tree_update_state.key(),
            merkle_tree_pda_data.pubkey_locked,
            String::from("Merkle tree locked by another account."),
        )?;

        msg!(
            "merkle_tree_update_state_data.current_instruction_index0 {}",
            merkle_tree_update_state_data.current_instruction_index
        );
        let id = IX_ORDER
            [usize::try_from(merkle_tree_update_state_data.current_instruction_index).unwrap()];
        if merkle_tree_update_state_data.current_instruction_index == 1 {
            compute_updated_merkle_tree(
                id,
                &mut merkle_tree_update_state_data,
                &mut merkle_tree_pda_data,
            )?;
            merkle_tree_update_state_data.current_instruction_index += 1;
        }

        msg!(
            "merkle_tree_update_state_data.current_instruction_index1 {}",
            merkle_tree_update_state_data.current_instruction_index
        );
        let id = IX_ORDER
            [usize::try_from(merkle_tree_update_state_data.current_instruction_index).unwrap()];
        compute_updated_merkle_tree(
            id,
            &mut merkle_tree_update_state_data,
            &mut merkle_tree_pda_data,
        )?;

        merkle_tree_update_state_data.current_instruction_index += 1;
        // renews lock
        // Accepted potential for starvation could be solved with relayer whitelist if it becomes a problem.
        merkle_tree_pda_data.time_locked = <Clock as sysvar::Sysvar>::get()?.slot;
    }

    Ok(())
}

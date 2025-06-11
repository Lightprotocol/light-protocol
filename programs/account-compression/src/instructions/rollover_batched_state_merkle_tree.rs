use anchor_lang::prelude::*;
use light_batched_merkle_tree::{
    merkle_tree::BatchedMerkleTreeAccount,
    rollover_state_tree::rollover_batched_state_tree_from_account_info,
};
use light_merkle_tree_metadata::errors::MerkleTreeMetadataError;

use crate::{
    utils::{
        check_signer_is_registered_or_authority::{
            check_signer_is_registered_or_authority, GroupAccounts,
        },
        transfer_lamports::transfer_lamports,
    },
    RegisteredProgram,
};

#[derive(Accounts)]
pub struct RolloverBatchedStateMerkleTree<'info> {
    #[account(mut)]
    /// Signer used to receive rollover accounts rent exemption reimbursement.
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    /// CHECK: is initialized in this instruction.
    #[account(mut)]
    pub new_state_merkle_tree: AccountInfo<'info>,
    /// CHECK: in state_from_account_info.
    #[account(mut)]
    pub old_state_merkle_tree: AccountInfo<'info>,
    /// CHECK: is initialized in this instruction.
    #[account(mut)]
    pub new_output_queue: AccountInfo<'info>,
    /// CHECK: in output_from_account_info.
    #[account(mut)]
    pub old_output_queue: AccountInfo<'info>,
}

impl<'info> GroupAccounts<'info> for RolloverBatchedStateMerkleTree<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

/// Rollover the old state Merkle tree and output queue to
/// new state Merkle tree and output queue.
/// 1. Check Merkle tree account discriminator, tree type, and program ownership.
/// 2. Check that signer is registered or authority.
/// 3. Rollover the old Merkle tree and queue to new Merkle tree and queue.
/// 4. Transfer rent exemption for new accounts
///    from old output queue to fee payer.
pub fn process_rollover_batched_state_merkle_tree<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, RolloverBatchedStateMerkleTree<'info>>,
    additional_bytes: u64,
    network_fee: Option<u64>,
) -> Result<()> {
    msg!(
        "old state Merkle tree {:?}",
        ctx.accounts.old_state_merkle_tree.key()
    );
    // 1. Check Merkle tree account discriminator, tree type, and program ownership.
    let old_merkle_tree_account =
        &mut BatchedMerkleTreeAccount::state_from_account_info(&ctx.accounts.old_state_merkle_tree)
            .map_err(ProgramError::from)?;

    // 2. Check that signer is registered or authority.
    check_signer_is_registered_or_authority::<
        RolloverBatchedStateMerkleTree,
        BatchedMerkleTreeAccount,
    >(&ctx, old_merkle_tree_account)?;

    // 3. Rollover the old Merkle tree and queue to new Merkle tree and queue.
    let rent = rollover_batched_state_tree_from_account_info(
        &ctx.accounts.old_state_merkle_tree,
        &ctx.accounts.new_state_merkle_tree,
        &ctx.accounts.old_output_queue,
        &ctx.accounts.new_output_queue,
        additional_bytes,
        network_fee,
    )
    .map_err(ProgramError::from)?;

    // 4. Transfer rent exemption for new accounts
    //     from old output queue to fee payer.
    transfer_lamports(
        &ctx.accounts.old_output_queue.to_account_info(),
        &ctx.accounts.fee_payer.to_account_info(),
        rent,
    )?;
    if ctx.accounts.old_output_queue.to_account_info().lamports() == 0 {
        return Err(ProgramError::from(MerkleTreeMetadataError::NotReadyForRollover).into());
    }
    Ok(())
}

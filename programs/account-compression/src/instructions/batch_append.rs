use anchor_lang::prelude::*;
use light_batched_merkle_tree::merkle_tree::{
    BatchedMerkleTreeAccount, InstructionDataBatchAppendInputs,
};

use crate::{
    emit_indexer_event,
    utils::{
        check_signer_is_registered_or_authority::{
            check_signer_is_registered_or_authority, GroupAccounts,
        },
        transfer_lamports::transfer_lamports,
    },
    RegisteredProgram,
};

#[derive(Accounts)]
pub struct BatchAppend<'info> {
    /// CHECK: should only be accessed by a registered program or owner.
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    /// CHECK: when emitting event.
    pub log_wrapper: UncheckedAccount<'info>,
    /// CHECK: in state_from_account_info.
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
    /// CHECK: in update_tree_from_output_queue_account_info.
    #[account(mut)]
    pub output_queue: AccountInfo<'info>,
    /// CHECK: receives network fee reimbursement.
    #[account(mut)]
    pub fee_payer: UncheckedAccount<'info>,
}

impl<'info> GroupAccounts<'info> for BatchAppend<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

/// Append a batch of leaves from the output queue
/// to the state Merkle tree.
/// 1. Check Merkle tree account discriminator, tree type, and program ownership.
/// 2. Check that signer is registered or authority.
/// 3. Append leaves from the output queue to the state Merkle tree.
///    3.1 Checks that output queue is associated with the Merkle tree.
///    3.2 Checks output queue discriminator, program ownership.
///    3.3 Verifies batch zkp and updates root.
/// 4. Emit indexer event.
pub fn process_batch_append_leaves<'a, 'b, 'c: 'info, 'info>(
    ctx: &'a Context<'info, BatchAppend<'info>>,
    instruction_data: InstructionDataBatchAppendInputs,
) -> Result<()> {
    // 1. Check Merkle tree account discriminator, tree type, and program ownership.
    let merkle_tree =
        &mut BatchedMerkleTreeAccount::state_from_account_info(&ctx.accounts.merkle_tree)
            .map_err(ProgramError::from)?;
    // 2. Check that signer is registered or authority.
    check_signer_is_registered_or_authority::<BatchAppend, BatchedMerkleTreeAccount>(
        ctx,
        merkle_tree,
    )?;

    // 3. Append leaves and check output queue account.
    let event = merkle_tree
        .update_tree_from_output_queue_account_info(&ctx.accounts.output_queue, instruction_data)
        .map_err(ProgramError::from)?;
    // 4. Transfer network fee reimbursement to fee payer.
    let network_fee = merkle_tree
        .get_metadata()
        .metadata
        .rollover_metadata
        .network_fee;
    if network_fee >= 5_000 {
        transfer_lamports(
            &ctx.accounts.output_queue.to_account_info(),
            &ctx.accounts.fee_payer.to_account_info(),
            network_fee * 2,
        )?;
    }
    // 5. Emit indexer event.
    emit_indexer_event(borsh::to_vec(&event)?, &ctx.accounts.log_wrapper)
}

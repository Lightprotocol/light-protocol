use anchor_lang::prelude::*;
use light_batched_merkle_tree::merkle_tree::{
    BatchedMerkleTreeAccount, InstructionDataBatchNullifyInputs,
};

use crate::{
    emit_indexer_event,
    utils::check_signer_is_registered_or_authority::{
        check_signer_is_registered_or_authority, GroupAccounts,
    },
    RegisteredProgram,
};

#[derive(Accounts)]
pub struct BatchNullify<'info> {
    /// CHECK: should only be accessed by a registered program or owner.
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    /// CHECK: when emitting event.
    pub log_wrapper: UncheckedAccount<'info>,
    /// CHECK: in state_from_account_info.
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
}

impl<'info> GroupAccounts<'info> for BatchNullify<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

/// Nullify a batch of leaves from the input queue
/// to the state Merkle tree.
/// Nullify means updating the leaf index with a nullifier.
/// The input queue is part of the state Merkle tree account.
/// 1. Check Merkle tree account discriminator, tree type, and program ownership.
/// 2. Check that signer is registered or authority.
/// 3. Nullify leaves from the input queue to the state Merkle tree.
///    3.1 Verifies batch zkp and updates root.
/// 4. Emit indexer event.
pub fn process_batch_nullify<'a, 'b, 'c: 'info, 'info>(
    ctx: &'a Context<'a, 'b, 'c, 'info, BatchNullify<'info>>,
    instruction_data: InstructionDataBatchNullifyInputs,
) -> Result<()> {
    // 1. Check Merkle tree account discriminator, tree type, and program ownership.
    let merkle_tree =
        &mut BatchedMerkleTreeAccount::state_from_account_info(&ctx.accounts.merkle_tree)
            .map_err(ProgramError::from)?;
    // 2. Check that signer is registered or authority.
    check_signer_is_registered_or_authority::<BatchNullify, BatchedMerkleTreeAccount>(
        ctx,
        merkle_tree,
    )?;
    // 3. Nullify leaves from the input queue to the state Merkle tree.
    let event = merkle_tree
        .update_tree_from_input_queue(instruction_data)
        .map_err(ProgramError::from)?;
    // 4. Emit indexer event.
    emit_indexer_event(event.try_to_vec()?, &ctx.accounts.log_wrapper)
}

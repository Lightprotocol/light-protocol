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
pub struct BatchUpdateAddressTree<'info> {
    /// CHECK: should only be accessed by a registered program or owner.
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    /// CHECK: when emitting event.
    pub log_wrapper: UncheckedAccount<'info>,
    /// CHECK: in from_account_info.
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
}

impl<'info> GroupAccounts<'info> for BatchUpdateAddressTree<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

/// Insert a batch of addresses into a batched address Merkle tree.
/// 1. Check tree account discriminator, tree type, and program ownership.
/// 2. Check that signer is registered or authority.
/// 3. Update the address tree with the batch of addresses.
///    3.1 Verifies batch update zkp and updates root.
/// 4. Emit indexer event.
pub fn process_batch_update_address_tree<'a, 'b, 'c: 'info, 'info>(
    ctx: &'a Context<'a, 'b, 'c, 'info, BatchUpdateAddressTree<'info>>,
    instruction_data: InstructionDataBatchNullifyInputs,
) -> Result<()> {
    // 1. Check tree account discriminator, tree type, and program ownership.
    let merkle_tree =
        &mut BatchedMerkleTreeAccount::address_from_account_info(&ctx.accounts.merkle_tree)
            .map_err(ProgramError::from)?;
    // 2. Check that signer is registered or authority.
    check_signer_is_registered_or_authority::<BatchUpdateAddressTree, BatchedMerkleTreeAccount>(
        ctx,
        merkle_tree,
    )?;
    // 3. Update the address tree with the batch of addresses.
    let event = merkle_tree
        .update_tree_from_address_queue(instruction_data)
        .map_err(ProgramError::from)?;
    // 4. Emit indexer event.
    emit_indexer_event(event.try_to_vec()?, &ctx.accounts.log_wrapper)
}

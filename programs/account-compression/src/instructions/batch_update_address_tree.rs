use anchor_lang::prelude::*;
use light_batched_merkle_tree::merkle_tree::{
    InstructionDataBatchNullifyInputs, ZeroCopyBatchedMerkleTreeAccount,
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

pub fn process_batch_update_address_tree<'a, 'b, 'c: 'info, 'info>(
    ctx: &'a Context<'a, 'b, 'c, 'info, BatchUpdateAddressTree<'info>>,
    instruction_data: InstructionDataBatchNullifyInputs,
) -> Result<()> {
    let merkle_tree = &mut ZeroCopyBatchedMerkleTreeAccount::address_tree_from_account_info_mut(
        &ctx.accounts.merkle_tree,
    )
    .map_err(ProgramError::from)?;
    check_signer_is_registered_or_authority::<
        BatchUpdateAddressTree,
        ZeroCopyBatchedMerkleTreeAccount,
    >(ctx, merkle_tree)?;
    let event = merkle_tree
        .update_address_queue(instruction_data, ctx.accounts.merkle_tree.key().to_bytes())
        .map_err(ProgramError::from)?;
    emit_indexer_event(event.try_to_vec()?, &ctx.accounts.log_wrapper)
}

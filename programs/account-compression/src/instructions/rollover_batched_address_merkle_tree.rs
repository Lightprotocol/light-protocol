use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_batched_merkle_tree::{
    merkle_tree::BatchedMerkleTreeAccount, rollover_address_tree::rollover_batched_address_tree,
};
use light_utils::account::check_account_balance_is_rent_exempt;

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
pub struct RolloverBatchAddressMerkleTree<'info> {
    #[account(mut)]
    /// Signer used to receive rollover accounts rentexemption reimbursement.
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    /// CHECK:  in account compression program.
    #[account(zero)]
    pub new_address_merkle_tree: AccountInfo<'info>,
    /// CHECK: cecked in manual deserialization.
    #[account(mut)]
    pub old_address_merkle_tree: AccountInfo<'info>,
}

impl<'info> GroupAccounts<'info> for RolloverBatchAddressMerkleTree<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

/// Checks:
/// 1. Merkle tree is ready to be rolled over
/// 2. Merkle tree is not already rolled over
/// 3. Rollover threshold is configured, if not tree cannot be rolled over
///
/// Actions:
/// 1. mark Merkle tree as rolled over in this slot
/// 2. initialize new Merkle tree and nullifier queue with the same parameters
pub fn process_rollover_batch_address_merkle_tree<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, RolloverBatchAddressMerkleTree<'info>>,
    network_fee: Option<u64>,
) -> Result<()> {
    let old_merkle_tree_account =
        &mut BatchedMerkleTreeAccount::address_tree_from_account_info_mut(
            &ctx.accounts.old_address_merkle_tree,
        )
        .map_err(ProgramError::from)?;
    check_signer_is_registered_or_authority::<
        RolloverBatchAddressMerkleTree,
        BatchedMerkleTreeAccount,
    >(&ctx, old_merkle_tree_account)?;

    let merkle_tree_rent = check_account_balance_is_rent_exempt(
        &ctx.accounts.new_address_merkle_tree.to_account_info(),
        ctx.accounts
            .old_address_merkle_tree
            .to_account_info()
            .data_len(),
    )
    .map_err(ProgramError::from)?;
    let new_mt_data = &mut ctx.accounts.new_address_merkle_tree.try_borrow_mut_data()?;
    rollover_batched_address_tree(
        old_merkle_tree_account,
        new_mt_data,
        merkle_tree_rent,
        ctx.accounts.new_address_merkle_tree.key(),
        network_fee,
    )
    .map_err(ProgramError::from)?;

    transfer_lamports(
        &ctx.accounts.old_address_merkle_tree.to_account_info(),
        &ctx.accounts.fee_payer.to_account_info(),
        merkle_tree_rent,
    )?;

    Ok(())
}

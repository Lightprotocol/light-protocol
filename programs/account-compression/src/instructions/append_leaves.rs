use crate::{
    errors::AccountCompressionErrorCode,
    state::StateMerkleTreeAccount,
    state_merkle_tree_from_bytes_zero_copy_mut,
    utils::{
        check_signer_is_registered_or_authority::{
            check_signer_is_registered_or_authority, GroupAccess, GroupAccounts,
        },
        transfer_lamports::transfer_lamports_cpi,
    },
    RegisteredProgram,
};
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

#[derive(Accounts)]
pub struct AppendLeaves<'info> {
    #[account(mut)]
    /// Fee payer pays rollover fee.
    pub fee_payer: Signer<'info>,
    /// Checked whether instruction is accessed by a registered program or owner = authority.
    pub authority: Signer<'info>,
    /// Some assumes that the Merkle trees are accessed by a registered program.
    /// None assumes that the Merkle trees are accessed by its owner.
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    pub system_program: Program<'info, System>,
}

impl GroupAccess for StateMerkleTreeAccount {
    fn get_owner(&self) -> &Pubkey {
        &self.metadata.access_metadata.owner
    }

    fn get_program_owner(&self) -> &Pubkey {
        &self.metadata.access_metadata.program_owner
    }
}

impl<'info> GroupAccounts<'info> for AppendLeaves<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

pub fn process_append_leaves_to_merkle_trees<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, AppendLeaves<'info>>,
    leaves: Vec<(u8, [u8; 32])>,
) -> Result<()> {
    let leaves_processed = batch_append_leaves(&ctx, &leaves)?;
    if leaves_processed != leaves.len() {
        return err!(crate::errors::AccountCompressionErrorCode::NotAllLeavesProcessed);
    }
    Ok(())
}

/// Perform batch appends to Merkle trees provided as remaining accounts. Leaves
/// are assumed to be ordered by Merkle tree account.
/// 1. Iterate over all remaining accounts (Merkle tree accounts)
/// 2. get first leaves that points to current Merkle tree account
/// 3. get last leaf that points to current Merkle tree account
/// 4. check Merkle tree account discriminator (AccountLoader)
/// 5. check signer elibility to write into Merkle tree account
///    (check_signer_is_registered_or_authority)
/// 6. append batch to Merkle tree
/// 7. transfer rollover fee
/// 8. get next Merkle tree account
fn batch_append_leaves<'a, 'c: 'info, 'info>(
    ctx: &Context<'a, '_, 'c, 'info, AppendLeaves<'info>>,
    leaves: &'a [(u8, [u8; 32])],
) -> Result<usize> {
    let mut leaves_processed: usize = 0;
    let len = ctx.remaining_accounts.len();
    for i in 0..len {
        let merkle_tree_acc_info = &ctx.remaining_accounts[i];
        let rollover_fee: u64 = {
            let start = match leaves.iter().position(|x| x.0 as usize == i) {
                Some(pos) => Ok(pos),
                None => err!(AccountCompressionErrorCode::NoLeavesForMerkleTree),
            }?;
            let end = match leaves[start..].iter().position(|x| x.0 as usize != i) {
                Some(pos) => pos + start,
                None => leaves.len(),
            };
            let batch_size = end - start;
            leaves_processed += batch_size;

            let rollover_fee = {
                let merkle_tree_account =
                    AccountLoader::<StateMerkleTreeAccount>::try_from(merkle_tree_acc_info)
                        .map_err(ProgramError::from)?;

                {
                    let merkle_tree_account = merkle_tree_account.load()?;
                    let rollover_fee = merkle_tree_account.metadata.rollover_metadata.rollover_fee
                        * batch_size as u64;

                    check_signer_is_registered_or_authority::<AppendLeaves, StateMerkleTreeAccount>(
                        ctx,
                        &merkle_tree_account,
                    )?;

                    rollover_fee
                }
            };

            let mut merkle_tree = merkle_tree_acc_info.try_borrow_mut_data()?;
            let mut merkle_tree = state_merkle_tree_from_bytes_zero_copy_mut(&mut merkle_tree)?;

            merkle_tree
                .append_batch(
                    leaves[start..end]
                        .iter()
                        .map(|x| &x.1)
                        .collect::<Vec<&[u8; 32]>>()
                        .as_slice(),
                )
                .map_err(ProgramError::from)?;

            rollover_fee
        };
        transfer_lamports_cpi(&ctx.accounts.fee_payer, merkle_tree_acc_info, rollover_fee)?;
    }
    Ok(leaves_processed)
}

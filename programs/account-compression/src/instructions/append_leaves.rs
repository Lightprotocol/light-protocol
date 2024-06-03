use crate::{
    errors::AccountCompressionErrorCode,
    state::StateMerkleTreeAccount,
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
    /// Signer used to pay rollover and protocol fees.
    pub fee_payer: Signer<'info>,
    /// CHECK: should only be accessed by a registered program or owner.
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    pub system_program: Program<'info, System>,
}

impl GroupAccess for StateMerkleTreeAccount {
    fn get_owner(&self) -> &Pubkey {
        &self.metadata.access_metadata.owner
    }

    fn get_delegate(&self) -> &Pubkey {
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
    let leaves_processed = process_batch(&ctx, &leaves)?;
    if leaves_processed != leaves.len() {
        return err!(crate::errors::AccountCompressionErrorCode::NotAllLeavesProcessed);
    }
    Ok(())
}

fn process_batch<'a, 'c: 'info, 'info>(
    ctx: &Context<'a, '_, 'c, 'info, AppendLeaves<'info>>,
    leaves: &'a [(u8, [u8; 32])],
) -> Result<usize> {
    // 1. init with first Merkle tree account
    // 2. iterate over all leaves
    // 3. if leaf belongs to current Merkle tree account insert into batch
    // 4. if leaf does not belong to current Merkle tree account
    // 5. append batch to Merkle tree
    // 6. insert rollover fee into vector
    // 7. reset batch
    // 8. get next Merkle tree account
    // 9. append leaf of different Merkle tree account to batch
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

            let merkle_tree_account =
                AccountLoader::<StateMerkleTreeAccount>::try_from(merkle_tree_acc_info)
                    .map_err(ProgramError::from)?;

            let mut merkle_tree_account = merkle_tree_account.load_mut()?;
            let rollover_fee =
                merkle_tree_account.metadata.rollover_metadata.rollover_fee * batch_size as u64;

            check_signer_is_registered_or_authority::<AppendLeaves, StateMerkleTreeAccount>(
                ctx,
                &merkle_tree_account,
            )?;

            let merkle_tree = merkle_tree_account.load_merkle_tree_mut()?;
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

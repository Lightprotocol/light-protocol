use std::collections::HashMap;

use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_concurrent_merkle_tree::event::Changelogs;
use light_macros::heap_neutral;

use crate::{
    emit_indexer_event,
    state::StateMerkleTreeAccount,
    utils::check_registered_or_signer::{check_registered_or_signer, GroupAccess, GroupAccounts},
    RegisteredProgram,
};

#[derive(Accounts)]
pub struct AppendLeaves<'info> {
    #[account(mut)]
    /// Signer used to pay rollover and protocol fees.
    pub fee_payer: Signer<'info>,
    /// CHECK: should only be accessed by a registered program/owner/delegate.
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    /// CHECK: in event emitting
    pub log_wrapper: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl GroupAccess for StateMerkleTreeAccount {
    fn get_owner(&self) -> &Pubkey {
        &self.owner
    }

    fn get_delegate(&self) -> &Pubkey {
        &self.delegate
    }
}

impl<'info> GroupAccounts<'info> for AppendLeaves<'info> {
    fn get_signing_address(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

/// for every leaf one Merkle tree account has to be passed as remaining account
/// for every leaf could be inserted into a different Merkle tree account
/// 1. deduplicate Merkle trees and identify into which tree to insert what leaf
/// 2. iterate over every unique Merkle tree and batch insert leaves
#[heap_neutral]
pub fn process_append_leaves_to_merkle_trees<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, AppendLeaves<'info>>,
    leaves: &'a [[u8; 32]],
) -> Result<()> {
    if leaves.len() != ctx.remaining_accounts.len() {
        return err!(crate::errors::AccountCompressionErrorCode::NumberOfLeavesMismatch);
    }
    let mut merkle_tree_map = HashMap::<Pubkey, (&AccountInfo, Vec<&[u8; 32]>)>::new();
    for (i, mt) in ctx.remaining_accounts.iter().enumerate() {
        match merkle_tree_map.get(&mt.key()) {
            Some(_) => {}
            None => {
                merkle_tree_map.insert(mt.key(), (mt, Vec::new()));
            }
        };
        merkle_tree_map
            .get_mut(&mt.key())
            .unwrap()
            .1
            .push(&leaves[i]);
    }

    let mut changelog_events = Vec::new();
    for (mt, leaves) in merkle_tree_map.values() {
        let merkle_tree_account = AccountLoader::<StateMerkleTreeAccount>::try_from(mt).unwrap();
        let merkle_tree_pubkey = merkle_tree_account.key();
        let lamports: u64;
        {
            let mut merkle_tree = merkle_tree_account.load_mut()?;
            lamports = merkle_tree.tip + merkle_tree.rollover_fee;
            check_registered_or_signer::<AppendLeaves, StateMerkleTreeAccount>(&ctx, &merkle_tree)?;

            msg!("inserting leaves: {:?}", leaves);
            let merkle_tree = merkle_tree.load_merkle_tree_mut()?;
            let (first_changelog_index, first_sequence_number) = merkle_tree
                .append_batch(&leaves[..])
                .map_err(ProgramError::from)?;
            let changelog_event = merkle_tree
                .get_changelog_event(
                    merkle_tree_pubkey.to_bytes(),
                    first_changelog_index,
                    first_sequence_number,
                    leaves.len(),
                )
                .map_err(ProgramError::from)?;
            changelog_events.push(changelog_event);
        }
        if lamports > 0 {
            transfer_lamports_cpi(
                &ctx.accounts.fee_payer,
                &merkle_tree_account.to_account_info(),
                lamports,
            )?;
        }
    }
    let changelog_event = Changelogs {
        changelogs: changelog_events,
    };
    emit_indexer_event(
        changelog_event.try_to_vec()?,
        &ctx.accounts.log_wrapper,
        &ctx.accounts.authority,
    )?;

    Ok(())
}

pub fn transfer_lamports<'info>(
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    lamports: u64,
) -> Result<()> {
    let compressed_sol_pda_lamports = from.as_ref().lamports();

    **from.as_ref().try_borrow_mut_lamports()? =
        compressed_sol_pda_lamports.checked_sub(lamports).unwrap();
    let recipient_lamports = to.as_ref().lamports();
    **to.as_ref().try_borrow_mut_lamports()? = recipient_lamports.checked_add(lamports).unwrap();
    Ok(())
}

pub fn transfer_lamports_cpi<'info>(
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    lamports: u64,
) -> Result<()> {
    let instruction =
        anchor_lang::solana_program::system_instruction::transfer(from.key, to.key, lamports);
    anchor_lang::solana_program::program::invoke(&instruction, &[from.clone(), to.clone()])?;
    Ok(())
}

use std::collections::HashMap;

use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use crate::{
    emit_indexer_event,
    state::ConcurrentMerkleTreeAccount,
    state_merkle_tree_from_bytes_mut,
    utils::check_registered_or_signer::{GroupAccess, GroupAccounts},
    ChangelogEvent, ChangelogEventV1, Changelogs, RegisteredProgram,
};

// TODO: implement group access control
#[derive(Accounts)]
pub struct InsertTwoLeavesParallel<'info> {
    /// CHECK: should only be accessed by a registered program/owner/delegate.
    #[account(mut)]
    pub authority: Signer<'info>,
    pub registered_verifier_pda: Option<Account<'info, RegisteredProgram>>,
    /// CHECK: in event emitting
    pub log_wrapper: UncheckedAccount<'info>,
}

impl GroupAccess for ConcurrentMerkleTreeAccount {
    fn get_owner(&self) -> &Pubkey {
        &self.owner
    }

    fn get_delegate(&self) -> &Pubkey {
        &self.delegate
    }
}

impl<'info> GroupAccounts<'info> for InsertTwoLeavesParallel<'info> {
    fn get_signing_address(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_verifier_pda
    }
}

/// for every leaf one Merkle tree account has to be passed as remaing account
/// for every leaf could be inserted into a different Merkle tree account
/// 1. deduplicate Merkle trees and identify into which tree to insert what leaf
/// 2. iterate over every unique Merkle tree and batch insert leaves
pub fn process_insert_leaves_into_merkle_trees<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, InsertTwoLeavesParallel<'info>>,
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
        let merkle_tree = AccountLoader::<ConcurrentMerkleTreeAccount>::try_from(mt).unwrap();
        let mut merkle_tree_account = merkle_tree.load_mut()?;
        // TODO: activate when group access control is implemented
        // check_registered_or_signer::<InsertTwoLeavesParallel, ConcurrentMerkleTreeAccount>(
        //     &ctx,
        //     &merkle_tree_account,
        // )?;

        let state_merkle_tree =
            state_merkle_tree_from_bytes_mut(&mut merkle_tree_account.state_merkle_tree);
        let changelog_entries = state_merkle_tree.append_batch(&leaves[..])?;
        changelog_events.push(ChangelogEvent::V1(ChangelogEventV1::new(
            mt.key(),
            &changelog_entries,
            state_merkle_tree.sequence_number,
        )?));
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

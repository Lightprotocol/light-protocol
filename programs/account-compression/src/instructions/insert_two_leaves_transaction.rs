use std::collections::HashMap;

use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use crate::{
    emit_indexer_event,
    state::StateMerkleTreeAccount,
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
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    /// CHECK: in event emitting
    pub log_wrapper: UncheckedAccount<'info>,
}

impl GroupAccess for StateMerkleTreeAccount {
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
        &self.registered_program_pda
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
        let merkle_tree = AccountLoader::<StateMerkleTreeAccount>::try_from(mt).unwrap();
        let mut merkle_tree_account = merkle_tree.load_mut()?;
        // TODO: activate when group access control is implemented
        // check_registered_or_signer::<InsertTwoLeavesParallel, StateMerkleTreeAccount>(
        //     &ctx,
        //     &merkle_tree_account,
        // )?;

        let state_merkle_tree =
            state_merkle_tree_from_bytes_mut(&mut merkle_tree_account.state_merkle_tree);
        let changelog_entries = state_merkle_tree.append_batch(&leaves[..]).unwrap();
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

#[cfg(not(target_os = "solana"))]
pub mod sdk {
    use anchor_lang::{system_program, InstructionData, ToAccountMetas};
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    };

    pub fn create_initialize_merkle_tree_instruction(
        payer: Pubkey,
        merkle_tree_pubkey: Pubkey,
    ) -> Instruction {
        let instruction_data: crate::instruction::InitializeConcurrentMerkleTree =
            crate::instruction::InitializeConcurrentMerkleTree {
                index: 1u64,
                owner: payer,
                delegate: None,
            };
        Instruction {
            program_id: crate::ID,
            accounts: vec![
                AccountMeta::new(payer, true),
                AccountMeta::new(merkle_tree_pubkey, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: instruction_data.data(),
        }
    }

    pub fn create_insert_leaves_instruction(
        leaves: Vec<[u8; 32]>,
        payer: Pubkey,
        merkle_tree_pubkeys: Vec<Pubkey>,
    ) -> Instruction {
        let instruction_data = crate::instruction::InsertLeavesIntoMerkleTrees { leaves };

        let accounts = crate::accounts::InsertTwoLeavesParallel {
            authority: payer,
            registered_program_pda: None,
            log_wrapper: crate::state::change_log_event::NOOP_PROGRAM_ID,
        };
        let merkle_tree_account_metas = merkle_tree_pubkeys
            .iter()
            .map(|pubkey| AccountMeta::new(*pubkey, false))
            .collect::<Vec<AccountMeta>>();

        Instruction {
            program_id: crate::ID,
            accounts: [
                accounts.to_account_metas(Some(true)),
                merkle_tree_account_metas,
            ]
            .concat(),
            data: instruction_data.data(),
        }
    }
}

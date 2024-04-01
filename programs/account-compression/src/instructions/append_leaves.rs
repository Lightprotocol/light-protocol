use std::collections::HashMap;

use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_macros::heap_neutral;

use crate::{
    emit_indexer_event,
    state::StateMerkleTreeAccount,
    utils::check_registered_or_signer::{check_registered_or_signer, GroupAccess, GroupAccounts},
    RegisteredProgram,
};

#[derive(Accounts)]
pub struct AppendLeaves<'info> {
    /// CHECK: should only be accessed by a registered program/owner/delegate.
    pub authority: Signer<'info>,
    // TODO: Add fee payer.
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

impl<'info> GroupAccounts<'info> for AppendLeaves<'info> {
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
        let mut merkle_tree = merkle_tree_account.load_mut()?;

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

    use crate::utils::constants::{
        STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_CHANGELOG, STATE_MERKLE_TREE_HEIGHT,
        STATE_MERKLE_TREE_ROOTS,
    };

    pub fn create_initialize_merkle_tree_instruction(
        payer: Pubkey,
        merkle_tree_pubkey: Pubkey,
        associated_queue: Option<Pubkey>,
    ) -> Instruction {
        let instruction_data: crate::instruction::InitializeStateMerkleTree =
            crate::instruction::InitializeStateMerkleTree {
                index: 1u64,
                owner: payer,
                delegate: None,
                height: STATE_MERKLE_TREE_HEIGHT,
                changelog_size: STATE_MERKLE_TREE_CHANGELOG,
                roots_size: STATE_MERKLE_TREE_ROOTS,
                canopy_depth: STATE_MERKLE_TREE_CANOPY_DEPTH,
                associated_queue,
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
        let instruction_data = crate::instruction::AppendLeavesToMerkleTrees { leaves };

        let accounts = crate::accounts::AppendLeaves {
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

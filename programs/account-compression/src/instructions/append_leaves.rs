use std::{cmp, collections::BTreeMap};

use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_concurrent_merkle_tree::event::{ChangelogEvent, ChangelogEventV1, Changelogs, PathNode};
use light_macros::heap_neutral;

use crate::{
    emit_indexer_event,
    errors::AccountCompressionErrorCode,
    state::StateMerkleTreeAccount,
    utils::check_registered_or_signer::{check_registered_or_signer, GroupAccess, GroupAccounts},
    RegisteredProgram,
};

const BATCH_SIZE: usize = 7;

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

fn build_merkle_tree_map<'a, 'c: 'info, 'info>(
    leaves: &'a [[u8; 32]],
    remaining_accounts: &'c [AccountInfo<'info>],
) -> BTreeMap<Pubkey, (&'c AccountInfo<'info>, Vec<&'a [u8; 32]>)> {
    let mut merkle_tree_map = BTreeMap::new();

    for (i, merkle_tree) in remaining_accounts.iter().enumerate() {
        merkle_tree_map
            .entry(merkle_tree.key())
            .or_insert_with(|| (merkle_tree, Vec::new()))
            .1
            .push(&leaves[i]);
    }

    merkle_tree_map
}

/// for every leaf could be inserted into a different Merkle tree account
/// 1. deduplicate Merkle trees and identify into which tree to insert what leaf
/// 2. iterate over every unique Merkle tree and batch insert leaves
#[heap_neutral]
pub fn process_append_leaves_to_merkle_trees<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, AppendLeaves<'info>>,
    leaves: &'a [[u8; 32]],
) -> Result<()> {
    #[cfg(target_os = "solana")]
    light_heap::GLOBAL_ALLOCATOR.log_total_heap("append_leaves: start");

    if leaves.len() != ctx.remaining_accounts.len() {
        return err!(crate::errors::AccountCompressionErrorCode::NumberOfLeavesMismatch);
    }

    let mut merkle_tree_map = build_merkle_tree_map(leaves, ctx.remaining_accounts);

    #[cfg(target_os = "solana")]
    light_heap::GLOBAL_ALLOCATOR.log_total_heap("append_leaves: merkle tree map");

    let mut leaves_start = 0;
    while !merkle_tree_map.is_empty() {
        process_batch(&ctx, &mut leaves_start, &mut merkle_tree_map)?;
    }

    Ok(())
}

#[heap_neutral]
#[inline(never)]
fn process_batch<'a, 'c: 'info, 'info>(
    ctx: &Context<'a, '_, 'c, 'info, AppendLeaves<'info>>,
    leaves_start: &mut usize,
    merkle_tree_map: &mut BTreeMap<Pubkey, (&'c AccountInfo<'info>, Vec<&'a [u8; 32]>)>,
) -> Result<()> {
    let mut leaves_in_batch = 0;
    let mut changelog_events = Vec::with_capacity(BATCH_SIZE);

    // A vector of trees which become fully processed and should be removed
    // from the `merkle_tree_map`.
    let mut processed_merkle_trees = Vec::new();

    {
        let mut merkle_tree_map_iter = merkle_tree_map.values();
        let mut merkle_tree_map_pair = merkle_tree_map_iter.next();

        while let Some((merkle_tree, leaves)) = merkle_tree_map_pair {
            let leaves_to_process =
                cmp::min(leaves.len() - *leaves_start, BATCH_SIZE - leaves_in_batch);
            let leaves_end = *leaves_start + leaves_to_process;

            let merkle_tree =
                AccountLoader::<StateMerkleTreeAccount>::try_from(merkle_tree).unwrap();
            let merkle_tree_pubkey = merkle_tree.key();
            let mut merkle_tree = merkle_tree.load_mut()?;

            check_registered_or_signer::<AppendLeaves, StateMerkleTreeAccount>(ctx, &merkle_tree)?;

            // Insert leaves to the Merkle tree.
            let merkle_tree = merkle_tree.load_merkle_tree_mut()?;
            let (first_changelog_index, first_sequence_number) = merkle_tree
                .append_batch(&leaves[*leaves_start..leaves_end])
                .map_err(ProgramError::from)?;

            let mut paths = Vec::with_capacity(leaves_to_process);

            // TODO: Move this code somewhere else, without affecting the feap neutrality.
            for changelog_index in first_changelog_index..first_changelog_index + leaves_to_process
            {
                let mut path = Vec::with_capacity(merkle_tree.height);

                for (level, node) in merkle_tree.changelog[changelog_index]
                    .path
                    .iter()
                    .enumerate()
                {
                    let level = u32::try_from(level)
                        .map_err(|_| AccountCompressionErrorCode::IntegerOverflow)?;
                    let index = (1 << (merkle_tree.height as u32 - level))
                        + (merkle_tree.changelog[changelog_index].index as u32 >> level);
                    path.push(PathNode {
                        node: node.to_owned(),
                        index,
                    });
                }

                paths.push(path);
            }

            changelog_events.push(ChangelogEvent::V1(ChangelogEventV1 {
                id: merkle_tree_pubkey.to_bytes(),
                paths,
                seq: first_sequence_number as u64,
                index: merkle_tree.changelog[first_changelog_index].index as u32,
            }));
            // END

            leaves_in_batch += leaves_to_process;
            *leaves_start += leaves_to_process;

            if *leaves_start == leaves.len() {
                // We processed all the leaves from the current Merkle tree.
                // Move to the next one.
                *leaves_start = 0;
                merkle_tree_map_pair = merkle_tree_map_iter.next();
                processed_merkle_trees.push(merkle_tree_pubkey.to_owned());
            }

            if leaves_in_batch == BATCH_SIZE {
                // We reached the batch limit.
                break;
            }
        }
    }

    emit_event(ctx, changelog_events)?;

    for processed_merkle_tree in processed_merkle_trees {
        merkle_tree_map.remove(&processed_merkle_tree);
    }

    Ok(())
}

#[heap_neutral]
#[inline(never)]
fn emit_event<'a, 'b, 'c: 'info, 'info>(
    ctx: &Context<'a, 'b, 'c, 'info, AppendLeaves<'info>>,
    changelog_events: Vec<ChangelogEvent>,
) -> Result<()> {
    // Emit the event.
    let changelog_event = Changelogs {
        changelogs: changelog_events,
    };

    // Calling `try_to_vec` allocates too much memory. Allocate the memory, up
    // to the instruction limit, manually.
    #[cfg(target_os = "solana")]
    light_heap::GLOBAL_ALLOCATOR.log_total_heap("before borsh serialization");
    let mut data = Vec::with_capacity(10240);
    changelog_event.serialize(&mut data)?;
    #[cfg(target_os = "solana")]
    light_heap::GLOBAL_ALLOCATOR.log_total_heap("after borsh serialization");

    emit_indexer_event(data, &ctx.accounts.log_wrapper, &ctx.accounts.authority)?;

    #[cfg(target_os = "solana")]
    light_heap::GLOBAL_ALLOCATOR.log_total_heap("after invoke");

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

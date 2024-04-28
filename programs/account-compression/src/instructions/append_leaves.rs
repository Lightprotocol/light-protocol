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

    let mut merkle_tree_map = build_merkle_tree_map(leaves, ctx.remaining_accounts);

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

        while let Some((merkle_tree_acc_info, leaves)) = merkle_tree_map_pair {
            let leaves_to_process =
                cmp::min(leaves.len() - *leaves_start, BATCH_SIZE - leaves_in_batch);
            let leaves_end = *leaves_start + leaves_to_process;

            let rollover_fee;
            let tip;
            {
                let merkle_tree =
                    AccountLoader::<StateMerkleTreeAccount>::try_from(merkle_tree_acc_info)
                        .unwrap();
                let merkle_tree_pubkey = merkle_tree.key();
                let mut merkle_tree = merkle_tree.load_mut()?;
                rollover_fee = merkle_tree.rollover_fee;
                tip = merkle_tree.tip;
                check_registered_or_signer::<AppendLeaves, StateMerkleTreeAccount>(
                    ctx,
                    &merkle_tree,
                )?;

                // Insert leaves to the Merkle tree.
                let merkle_tree = merkle_tree.load_merkle_tree_mut()?;
                let (first_changelog_index, first_sequence_number) = merkle_tree
                    .append_batch(&leaves[*leaves_start..leaves_end])
                    .map_err(ProgramError::from)?;

                let mut paths = Vec::with_capacity(leaves_to_process);

                // NOTE: It's tricky to factor our this code to a separate function
                // without increasing the heap usage.
                // If you feel brave enough to refactor it, don't break the test
                // which appends 60 leaves!
                for changelog_index in
                    first_changelog_index..first_changelog_index + leaves_to_process
                {
                    let changelog_index = changelog_index % merkle_tree.changelog_capacity;
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
                // NOTE: Making this to work would be the part of the potential
                // refactor mentioned above.
                //
                // let changelog_event = merkle_tree
                //     .get_changelog_event(
                //         merkle_tree_pubkey.to_bytes(),
                //         first_changelog_index,
                //         first_sequence_number,
                //         leaves.len(),
                //     )
                //     .map_err(ProgramError::from)?;
                // changelog_events.push(changelog_event);
            }

            let lamports = rollover_fee * leaves.len() as u64 + tip;
            if lamports > 0 && leaves_end == leaves.len() {
                msg!("transferring rollover fee: {}", lamports);
                transfer_lamports_cpi(&ctx.accounts.fee_payer, merkle_tree_acc_info, lamports)?;
            }

            leaves_in_batch += leaves_to_process;
            *leaves_start += leaves_to_process;

            if *leaves_start == leaves.len() {
                // We processed all the leaves from the current Merkle tree.
                // Move to the next one.
                *leaves_start = 0;
                merkle_tree_map_pair = merkle_tree_map_iter.next();
                processed_merkle_trees.push(merkle_tree_acc_info.key().to_owned());
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
    let mut data = Vec::with_capacity(10240);
    changelog_event.serialize(&mut data)?;

    emit_indexer_event(data, &ctx.accounts.log_wrapper, &ctx.accounts.authority)
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

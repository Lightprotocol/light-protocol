use std::cmp;

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

/// for every leaf one Merkle tree account has to be passed as remaining account
/// for every leaf could be inserted into a different Merkle tree account
/// 1. deduplicate Merkle trees and identify into which tree to insert what leaf
/// 2. iterate over every unique Merkle tree and batch insert leaves
#[heap_neutral]
pub fn process_append_leaves_to_merkle_trees<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, AppendLeaves<'info>>,
    leaves: &'a [(u8, [u8; 32])],
) -> Result<()> {
    let mut fee_vec = Vec::<(u8, u64)>::with_capacity(leaves.len());
    for i in 0..(leaves.len().div_ceil(BATCH_SIZE)) {
        let leaves_start = i * BATCH_SIZE;
        let leaves_to_process = cmp::min(leaves.len().saturating_sub(i * BATCH_SIZE), BATCH_SIZE);
        let leaves_end = leaves_start + leaves_to_process;
        process_batch(&ctx, &leaves[i * BATCH_SIZE..leaves_end], &mut fee_vec)?;
    }

    for (index, lamports) in fee_vec {
        transfer_lamports_cpi(
            &ctx.accounts.fee_payer,
            &ctx.remaining_accounts[index as usize].to_account_info(),
            lamports,
        )?;
    }
    Ok(())
}

#[heap_neutral]
#[inline(never)]
fn process_batch<'a, 'c: 'info, 'info>(
    ctx: &Context<'a, '_, 'c, 'info, AppendLeaves<'info>>,
    leaves: &'a [(u8, [u8; 32])],
    fee_vec: &mut Vec<(u8, u64)>,
) -> Result<()> {
    let mut leaves_in_batch = 0;
    let mut batch = Vec::with_capacity(BATCH_SIZE);
    let mut changelog_events = Vec::with_capacity(BATCH_SIZE);

    {
        // init with first Merkle tree account
        // iterate over all leaves
        // if leaf belongs to current Merkle tree account insert into batch
        // if leaf does not belong to current Merkle tree account
        // append batch to Merkle tree
        // insert rollover fee into vector
        // reset batch
        // get next Merkle tree account
        // append leaf of different Merkle tree account to batch
        let mut current_mt_index = leaves[0].0 as usize;
        let mut merkle_tree_acc_info = &ctx.remaining_accounts[current_mt_index];
        let merkle_tree_account =
            AccountLoader::<StateMerkleTreeAccount>::try_from(merkle_tree_acc_info).unwrap();
        let mut merkle_tree_pubkey = merkle_tree_acc_info.key();

        for leaf in leaves.iter() {
            if leaf.0 as usize == current_mt_index {
                batch.push(&leaf.1);
                leaves_in_batch += 1;
            }

            if leaf.0 as usize != current_mt_index || leaves_in_batch == leaves.len() {
                // Insert leaves to the Merkle tree.
                {
                    let mut merkle_tree_account = merkle_tree_account.load_mut()?;
                    let index = fee_vec.iter().position(|x| x.0 == current_mt_index as u8);
                    match index {
                        Some(i) => {
                            fee_vec[i].1 +=
                                merkle_tree_account.rollover_fee * leaves_in_batch as u64;
                        }
                        None => {
                            fee_vec.push((
                                current_mt_index as u8,
                                merkle_tree_account.rollover_fee * leaves_in_batch as u64
                                    + merkle_tree_account.tip,
                            ));
                        }
                    }
                    check_registered_or_signer::<AppendLeaves, StateMerkleTreeAccount>(
                        ctx,
                        &merkle_tree_account,
                    )?;
                    let merkle_tree = merkle_tree_account.load_merkle_tree_mut()?;
                    let (first_changelog_index, first_sequence_number) = merkle_tree
                        .append_batch(&batch)
                        .map_err(ProgramError::from)?;
                    let mut paths = Vec::with_capacity(leaves_in_batch);

                    // NOTE: It's tricky to factor our this code to a separate function
                    // without increasing the heap usage.
                    // If you feel brave enough to refactor it, don't break the test
                    // which appends 60 leaves!
                    for changelog_index in
                        first_changelog_index..first_changelog_index + leaves_in_batch
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
                };
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
                current_mt_index = leaf.0 as usize;
                merkle_tree_acc_info = &ctx.remaining_accounts[current_mt_index];
                merkle_tree_pubkey = merkle_tree_acc_info.key();
                batch = Vec::with_capacity(BATCH_SIZE);
                batch.push(&leaf.1);
                leaves_in_batch += 1;
                if leaves_in_batch == BATCH_SIZE {
                    // We reached the batch limit.
                    break;
                }
            }
        }

        emit_event(ctx, changelog_events)?;
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

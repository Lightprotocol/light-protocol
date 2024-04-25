use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_macros::heap_neutral;

use crate::instructions::{InstructionDataTransfer, TransferInstruction};

/// 1. Checks that the nullifier queue account is associated with a state Merkle tree account.
/// 2. Inserts nullifiers into the queue.
#[heap_neutral]
pub fn insert_nullifiers<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    nullifiers: &'a [[u8; 32]],
) -> Result<()> {
    let state_merkle_tree_account_infos = inputs
        .input_compressed_accounts_with_merkle_context
        .iter()
        .map(|account| ctx.remaining_accounts[account.merkle_tree_pubkey_index as usize].clone())
        .collect::<Vec<AccountInfo<'info>>>();
    let mut nullifier_queue_account_infos = Vec::<AccountInfo>::new();
    for account in inputs.input_compressed_accounts_with_merkle_context.iter() {
        nullifier_queue_account_infos
            .push(ctx.remaining_accounts[account.nullifier_queue_pubkey_index as usize].clone());
    }

    insert_nullifiers_cpi(
        ctx.program_id,
        &ctx.accounts.account_compression_program,
        &ctx.accounts.fee_payer.to_account_info(),
        &ctx.accounts.account_compression_authority,
        &ctx.accounts.registered_program_pda.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        nullifier_queue_account_infos,
        state_merkle_tree_account_infos,
        nullifiers.to_vec(),
    )?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[allow(unused_variables)]
#[inline(never)]
pub fn insert_nullifiers_cpi<'a, 'b>(
    program_id: &Pubkey,
    account_compression_program_id: &'b AccountInfo<'a>,
    fee_payer: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    registered_program_pda: &'b AccountInfo<'a>,
    system_program: &'b AccountInfo<'a>,
    nullifier_queue_account_infos: Vec<AccountInfo<'a>>,
    merkle_tree_account_infos: Vec<AccountInfo<'a>>,
    nullifiers: Vec<[u8; 32]>,
) -> Result<()> {
    let (_, bump) =
        anchor_lang::prelude::Pubkey::find_program_address(&[b"cpi_authority"], program_id);
    let bump = &[bump];
    let seeds = &[&[b"cpi_authority".as_slice(), bump][..]];

    let accounts = account_compression::cpi::accounts::InsertIntoNullifierQueues {
        fee_payer: fee_payer.to_account_info(),
        authority: authority.to_account_info(),
        registered_program_pda: Some(registered_program_pda.to_account_info()),
        system_program: system_program.to_account_info(),
    };

    let mut cpi_ctx =
        CpiContext::new_with_signer(account_compression_program_id.clone(), accounts, seeds);
    cpi_ctx
        .remaining_accounts
        .extend(nullifier_queue_account_infos);
    cpi_ctx.remaining_accounts.extend(merkle_tree_account_infos);
    msg!("inserting nullifiers {:?}", nullifiers);
    account_compression::cpi::insert_into_nullifier_queues(cpi_ctx, nullifiers)
}

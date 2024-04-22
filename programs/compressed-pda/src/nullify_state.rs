use account_compression::NullifierQueueAccount;
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_macros::heap_neutral;

use crate::{
    append_state::get_seeds,
    instructions::{InstructionDataTransfer, TransferInstruction},
};

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
        let unpacked_queue_account = AccountLoader::<NullifierQueueAccount>::try_from(
            &ctx.remaining_accounts[account.nullifier_queue_pubkey_index as usize],
        )
        .unwrap();
        let array_account = unpacked_queue_account.load()?;

        let account_is_associated_with_state_merkle_tree = state_merkle_tree_account_infos
            .iter()
            .any(|x| x.key() == array_account.associated_merkle_tree);

        if !account_is_associated_with_state_merkle_tree {
            msg!(
                "Nullifier queue account {:?} is not associated with any state Merkle tree. Provided state Merkle trees {:?}",
                ctx.remaining_accounts[account.nullifier_queue_pubkey_index as usize].key(), state_merkle_tree_account_infos);
            return Err(crate::ErrorCode::InvalidNullifierQueue.into());
        }
    }

    insert_nullifiers_cpi(
        ctx.program_id,
        &ctx.accounts.account_compression_program,
        &ctx.accounts.account_compression_authority,
        &ctx.accounts.registered_program_pda.to_account_info(),
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
    authority: &'b AccountInfo<'a>,
    registered_program_pda: &'b AccountInfo<'a>,
    nullifier_queue_account_infos: Vec<AccountInfo<'a>>,
    merkle_tree_account_infos: Vec<AccountInfo<'a>>,
    nullifiers: Vec<[u8; 32]>,
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, &authority.key())?;
    let bump = &[bump];
    let seeds = &[&[b"cpi_authority", seed.as_slice(), bump][..]];

    let accounts = account_compression::cpi::accounts::InsertIntoNullifierQueues {
        authority: authority.to_account_info(),
        registered_program_pda: Some(registered_program_pda.to_account_info()),
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

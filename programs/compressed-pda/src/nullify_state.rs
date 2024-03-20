use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use crate::{
    append_state::get_seeds,
    instructions::{InstructionDataTransfer, TransferInstruction},
};
pub fn insert_nullifiers<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    nullifiers: &'a [[u8; 32]],
) -> anchor_lang::Result<()> {
    let mut indexed_array_account_infos = Vec::<AccountInfo>::new();
    for account in inputs.input_compressed_accounts_with_merkle_context.iter() {
        indexed_array_account_infos
            .push(ctx.remaining_accounts[account.index_nullifier_array_account as usize].clone());
    }

    insert_nullifiers_cpi(
        ctx.program_id,
        &ctx.accounts.account_compression_program,
        &ctx.accounts.psp_account_compression_authority,
        &ctx.accounts.registered_program_pda,
        indexed_array_account_infos,
        nullifiers.to_vec(),
    )
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
    nullifiers: Vec<[u8; 32]>,
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, &authority.key())?;
    let bump = &[bump];
    let seeds = &[&[b"cpi_authority", seed.as_slice(), bump][..]];

    let accounts = account_compression::cpi::accounts::InsertIntoIndexedArrays {
        authority: authority.to_account_info(),
        registered_program_pda: Some(registered_program_pda.to_account_info()),
    };

    let mut cpi_ctx =
        CpiContext::new_with_signer(account_compression_program_id.clone(), accounts, seeds);
    cpi_ctx.remaining_accounts = nullifier_queue_account_infos;
    msg!("inserting nullifiers {:?}", nullifiers);
    account_compression::cpi::insert_into_indexed_arrays(cpi_ctx, nullifiers)
}

use anchor_lang::prelude::*;

use crate::instructions::{InstructionDataTransfer, TransferInstruction};

pub fn insert_addresses_into_address_merkle_tree_queue<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    addresses: &'a [[u8; 32]],
) -> anchor_lang::Result<()> {
    let mut remaining_accounts =
        Vec::<AccountInfo>::with_capacity(inputs.new_address_params.len() * 2);
    inputs.new_address_params.iter().for_each(|params| {
        remaining_accounts
            .push(ctx.remaining_accounts[params.address_queue_account_index as usize].clone());
        remaining_accounts
            .push(ctx.remaining_accounts[params.address_merkle_tree_account_index as usize].clone())
    });

    insert_addresses_cpi(
        ctx.program_id,
        &ctx.accounts.account_compression_program,
        &ctx.accounts.fee_payer.to_account_info(),
        &ctx.accounts.account_compression_authority,
        &ctx.accounts.registered_program_pda.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        remaining_accounts,
        addresses.to_vec(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn insert_addresses_cpi<'a, 'b>(
    program_id: &Pubkey,
    account_compression_program_id: &'b AccountInfo<'a>,
    fee_payer: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    registered_program_pda: &'b AccountInfo<'a>,
    system_program: &'b AccountInfo<'a>,
    remaining_accounts: Vec<AccountInfo<'a>>,
    addresses: Vec<[u8; 32]>,
) -> Result<()> {
    let (_, bump) =
        anchor_lang::prelude::Pubkey::find_program_address(&[b"cpi_authority"], program_id);
    let bump = &[bump];
    let seeds = &[&[b"cpi_authority".as_slice(), bump][..]];
    let accounts = account_compression::cpi::accounts::InsertAddresses {
        fee_payer: fee_payer.to_account_info(),
        authority: authority.to_account_info(),
        registered_program_pda: Some(registered_program_pda.to_account_info()),
        system_program: system_program.to_account_info(),
    };

    let mut cpi_ctx =
        CpiContext::new_with_signer(account_compression_program_id.clone(), accounts, seeds);
    cpi_ctx.remaining_accounts.extend(remaining_accounts);

    account_compression::cpi::insert_addresses(cpi_ctx, addresses)
}

use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::{
    prelude::{AccountMeta, Context, Pubkey},
    Bumps, InstructionData, Key, Result, ToAccountInfo,
};
use light_compressed_account::insert_into_queues::AppendNullifyCreateAddressInputs;

use crate::{
    account_traits::{InvokeAccounts, SignerAccounts},
    constants::CPI_AUTHORITY_PDA_BUMP,
    context::SystemContext,
};

pub fn create_cpi_data_and_context<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + SignerAccounts<'info> + Bumps,
>(
    ctx: &'a Context<'a, 'b, 'c, 'info, A>,
    num_leaves: u8,
    num_nullifiers: u8,
    num_new_addresses: u8,
    hashed_pubkeys_capacity: usize,
    invoking_program_id: Option<Pubkey>,
) -> Result<(SystemContext<'info>, Vec<u8>)> {
    let account_infos = vec![
        ctx.accounts
            .get_account_compression_authority()
            .to_account_info(),
        ctx.accounts.get_registered_program_pda().to_account_info(),
    ];
    let accounts = vec![
        AccountMeta::new_readonly(account_infos[0].key(), true),
        AccountMeta::new_readonly(account_infos[1].key(), false),
    ];
    let account_indices =
        Vec::<u8>::with_capacity((num_nullifiers + num_leaves + num_new_addresses) as usize);
    let bytes_size = AppendNullifyCreateAddressInputs::required_size_for_capacity(
        num_leaves,
        num_nullifiers,
        num_new_addresses,
        num_leaves,
    );
    let bytes = vec![0u8; bytes_size];
    Ok((
        SystemContext {
            account_indices,
            accounts,
            account_infos,
            hashed_pubkeys: Vec::with_capacity(hashed_pubkeys_capacity),
            // TODO: init with capacity.
            addresses: Vec::with_capacity((num_nullifiers + num_new_addresses) as usize),
            rollover_fee_payments: Vec::new(),
            address_fee_is_set: false,
            network_fee_is_set: false,
            legacy_merkle_context: Vec::new(),
            invoking_program_id,
        },
        bytes,
    ))
}

pub fn cpi_account_compression_program(cpi_context: SystemContext, bytes: Vec<u8>) -> Result<()> {
    let SystemContext {
        accounts,
        account_infos,
        ..
    } = cpi_context;
    let instruction_data = account_compression::instruction::InsertIntoQueues { bytes };

    let data = instruction_data.data();
    let bump = &[CPI_AUTHORITY_PDA_BUMP];
    let seeds = &[&[CPI_AUTHORITY_PDA_SEED, bump][..]];
    let instruction = anchor_lang::solana_program::instruction::Instruction {
        program_id: account_compression::ID,
        accounts,
        data,
    };
    anchor_lang::solana_program::program::invoke_signed(
        &instruction,
        account_infos.as_slice(),
        seeds,
    )?;
    Ok(())
}

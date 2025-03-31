use std::cmp::min;

use crate::{constants::CPI_AUTHORITY_PDA_SEED, Result};
// use anchor_lang::{
//     prelude::{AccountMeta, Context, Pubkey},
//     Bumps, InstructionData, Key, Result, ToAccountInfo,
// };

use light_compressed_account::{
    constants::ACCOUNT_COMPRESSION_PROGRAM_ID, discriminators::DISCRIMINATOR_INSERT_INTO_QUEUES,
    instruction_data::insert_into_queues::InsertIntoQueuesInstructionDataMut,
};
use pinocchio::{
    account_info::AccountInfo,
    cpi::{invoke_signed, slice_invoke_signed},
    instruction::{AccountMeta, Instruction, Seed, Signer},
    pubkey::Pubkey,
};

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
    A: InvokeAccounts<'info> + SignerAccounts<'info>,
>(
    ctx: &'a A,
    num_leaves: u8,
    num_nullifiers: u8,
    num_new_addresses: u8,
    hashed_pubkeys_capacity: usize,
    invoking_program_id: Option<Pubkey>,
    remaining_accounts: &'info [AccountInfo],
) -> Result<(SystemContext<'info>, Vec<u8>)> {
    let account_infos = vec![
        ctx.get_account_compression_authority(),
        ctx.get_registered_program_pda(),
    ];
    let accounts = vec![
        AccountMeta::new(account_infos[0].key(), false, true),
        AccountMeta::readonly(account_infos[1].key()),
    ];
    let account_indices =
        Vec::<u8>::with_capacity((num_nullifiers + num_leaves + num_new_addresses) as usize);
    // Min (remaining accounts or num values) for there cannot be more trees than accounts or values.
    let bytes_size = InsertIntoQueuesInstructionDataMut::required_size_for_capacity(
        num_leaves,
        num_nullifiers,
        num_new_addresses,
        min(remaining_accounts.len() as u8, num_leaves),
        min(remaining_accounts.len() as u8, num_nullifiers),
        min(remaining_accounts.len() as u8, num_new_addresses),
    );
    // Data size + 8 bytes for discriminator + 4 bytes for length.
    let byte_len = bytes_size + 8 + 4;
    let mut bytes = vec![0u8; byte_len];
    bytes[..8].copy_from_slice(&DISCRIMINATOR_INSERT_INTO_QUEUES);
    // Vec len.
    bytes[8..12].copy_from_slice(&byte_len.to_le_bytes());
    Ok((
        SystemContext {
            account_indices,
            accounts,
            account_infos,
            hashed_pubkeys: Vec::with_capacity(hashed_pubkeys_capacity),
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

    let bump = &[CPI_AUTHORITY_PDA_BUMP];
    let instruction = Instruction {
        program_id: &ACCOUNT_COMPRESSION_PROGRAM_ID,
        accounts: accounts.as_slice(),
        data: bytes.as_slice(),
    };
    let seed_array = [Seed::from(CPI_AUTHORITY_PDA_SEED), Seed::from(bump)];
    let signer = Signer::from(&seed_array);

    slice_invoke_signed(&instruction, account_infos.as_slice(), &[signer])
}

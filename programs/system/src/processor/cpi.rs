use std::cmp::min;

use light_compressed_account::{
    constants::ACCOUNT_COMPRESSION_PROGRAM_ID, discriminators::DISCRIMINATOR_INSERT_INTO_QUEUES,
    instruction_data::insert_into_queues::InsertIntoQueuesInstructionDataMut,
};
use light_program_profiler::profile;
use pinocchio::{
    account_info::AccountInfo,
    cpi::slice_invoke_signed,
    instruction::{AccountMeta, Instruction, Seed, Signer},
    pubkey::Pubkey,
};

use crate::{
    accounts::account_traits::{InvokeAccounts, SignerAccounts},
    constants::{CPI_AUTHORITY_PDA_BUMP, CPI_AUTHORITY_PDA_SEED},
    context::SystemContext,
    errors::SystemProgramError,
    Result,
};
#[profile]
#[allow(clippy::too_many_arguments)]
pub fn create_cpi_data_and_context<'info, A: InvokeAccounts<'info> + SignerAccounts<'info>>(
    ctx: &A,
    num_leaves: u8,
    num_nullifiers: u8,
    num_new_addresses: u8,
    hashed_pubkeys_capacity: usize,
    cpi_data_len: usize,
    invoking_program_id: Option<Pubkey>,
    remaining_accounts: &'info [AccountInfo],
) -> Result<(SystemContext<'info>, Vec<u8>)> {
    let account_infos = vec![
        ctx.get_account_compression_authority()?,
        ctx.get_registered_program_pda()?,
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
    // Data size + 8 bytes for discriminator + 4 bytes for vec length + 4 bytes for cpi data vec length + cpi data length.
    let byte_len = bytes_size + 8 + 4 + 4 + cpi_data_len;

    // Enforce CPI account growth limit of 10KB
    const MAX_CPI_BUFFER_SIZE: usize = 10240;
    if byte_len > MAX_CPI_BUFFER_SIZE {
        return Err(SystemProgramError::AccountCompressionCpiDataExceedsLimit.into());
    }
    let mut bytes = vec![0u8; byte_len];
    bytes[..8].copy_from_slice(&DISCRIMINATOR_INSERT_INTO_QUEUES);
    // Vec len.
    bytes[8..12].copy_from_slice(&u32::try_from(byte_len - 12).unwrap().to_le_bytes());
    Ok((
        SystemContext {
            account_indices,
            accounts,
            account_infos,
            hashed_pubkeys: Vec::with_capacity(hashed_pubkeys_capacity),
            addresses: Vec::with_capacity((num_nullifiers + num_new_addresses) as usize),
            rollover_fee_payments: Vec::new(),
            network_fee_is_set: false,
            legacy_merkle_context: Vec::new(),
            invoking_program_id,
        },
        bytes,
    ))
}

#[profile]
pub fn cpi_account_compression_program(
    cpi_context: SystemContext<'_>,
    bytes: Vec<u8>,
) -> Result<()> {
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

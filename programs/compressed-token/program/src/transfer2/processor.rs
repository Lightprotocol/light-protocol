use anchor_compressed_token::check_cpi_context;
use anchor_lang::prelude::ProgramError;
use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly;
use light_ctoken_types::{
    context::TokenContext,
    instructions::transfer2::{validate_instruction_data, CompressedTokenInstructionDataTransfer2},
};
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_zero_copy::{borsh::Deserialize, ZeroCopyNew};
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;

use crate::{
    shared::cpi::execute_cpi_invoke,
    transfer2::{
        accounts::Transfer2Accounts, change_account::process_change_lamports,
        cpi::allocate_cpi_bytes, native_compression::process_token_compression,
        sum_check::sum_check_multi_mint, token_inputs::set_input_compressed_accounts,
        token_outputs::set_output_compressed_accounts,
    },
};

/// Process a token transfer instruction
/// build inputs -> sum check -> build outputs -> add token data to inputs -> invoke cpi
/// 1.  Unpack compressed input accounts and input token data, this uses
///     standardized signer / delegate and will fail in proof verification in
///     case either is invalid.
/// 2.  Check that compressed accounts are of same mint.
/// 3.  Check that sum of input compressed accounts is equal to sum of output
///     compressed accounts
/// 4.  create_output_compressed_accounts
/// 5.  Serialize and add token_data data to in compressed_accounts.
/// 6.  Invoke light_system_program::execute_compressed_transaction.
#[inline(always)]
pub fn process_transfer2(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Parse instruction data first to determine optional accounts
    let (inputs, _) = CompressedTokenInstructionDataTransfer2::zero_copy_at(instruction_data)
        .map_err(ProgramError::from)?;

    // Check CPI context validity (multi-transfer modifies Solana account state)
    check_cpi_context(&inputs.cpi_context).map_err(ProgramError::from)?;

    let total_input_lamports = if let Some(inputs) = inputs.in_lamports.as_ref() {
        inputs.iter().map(|input| u64::from(**input)).sum()
    } else {
        0
    };
    let total_output_lamports = if let Some(inputs) = inputs.out_lamports.as_ref() {
        inputs.iter().map(|input| u64::from(**input)).sum()
    } else {
        0
    };

    // Determine optional account flags from instruction data
    let with_sol_pool = total_input_lamports != total_output_lamports;
    let decompress_sol = total_input_lamports < total_output_lamports;
    let with_cpi_context = inputs.cpi_context.is_some();
    msg!("with_cpi_context: {}", with_cpi_context);
    let write_to_cpi_context = inputs
        .cpi_context
        .as_ref()
        .map(|x| x.first_set_context || x.set_context)
        .unwrap_or_default();
    msg!("write_to_cpi_context: {}", write_to_cpi_context);
    // Skip first account (light-system-program) and validate remaining accounts
    let validated_accounts = Transfer2Accounts::validate_and_parse(
        &accounts,
        with_sol_pool,
        decompress_sol,
        with_cpi_context,
        write_to_cpi_context,
    )?;
    // Validate instruction data consistency
    validate_instruction_data(&inputs)?;
    bench_sbf_start!("t_context_and_check_sig");
    // anchor_lang::solana_program::log::msg!("inputs {:?}", inputs);

    // Create TokenContext for hash caching
    let mut context = TokenContext::new();

    // Allocate CPI bytes and create zero-copy structure
    let (mut cpi_bytes, config) = allocate_cpi_bytes(&inputs);

    let (mut cpi_instruction_struct, _) =
        InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
            .map_err(ProgramError::from)?;
    cpi_instruction_struct.initialize(
        crate::LIGHT_CPI_SIGNER.bump,
        &crate::LIGHT_CPI_SIGNER.program_id.into(),
        inputs.proof,
        &inputs.cpi_context,
    )?;

    // Process input compressed accounts
    set_input_compressed_accounts(
        &mut cpi_instruction_struct,
        &mut context,
        &inputs,
        &validated_accounts.packed_accounts,
    )?;

    // Process output compressed accounts
    set_output_compressed_accounts(
        &mut cpi_instruction_struct,
        &mut context,
        &inputs,
        &validated_accounts.packed_accounts,
    )?;
    bench_sbf_end!("t_create_output_compressed_accounts");
    //msg!("cpi_instruction_struct {:?}", cpi_instruction_struct);

    process_change_lamports(
        &inputs,
        &validated_accounts.packed_accounts,
        cpi_instruction_struct,
        total_input_lamports,
        total_output_lamports,
    )?;
    // Process token compressions/decompressions
    // TODO: support spl
    process_token_compression(&inputs, &validated_accounts.packed_accounts)?;
    bench_sbf_end!("t_context_and_check_sig");
    bench_sbf_start!("t_sum_check");
    sum_check_multi_mint(
        &inputs.in_token_data,
        &inputs.out_token_data,
        inputs.compressions.as_deref(),
    )
    .map_err(|e| ProgramError::Custom(e as u32))?;
    bench_sbf_end!("t_sum_check");
    msg!("here");
    if let Some(system_accounts) = validated_accounts.system.as_ref() {
        msg!("here");
        // Get CPI accounts slice and tree accounts for light-system-program invocation
        let (cpi_accounts, tree_pubkeys) =
            validated_accounts.cpi_accounts(accounts, &inputs, &validated_accounts.packed_accounts);
        // Debug prints keep for now.
        {
            let _solana_tree_accounts = tree_pubkeys
                .iter()
                .map(|&x| solana_pubkey::Pubkey::new_from_array(*x))
                .collect::<Vec<_>>();
            let _cpi_accounts = cpi_accounts
                .iter()
                .map(|x| solana_pubkey::Pubkey::new_from_array(*x.key()))
                .collect::<Vec<_>>();
            msg!("account infos {:?}", _cpi_accounts);
            msg!("tree pubkeys {:?}", _solana_tree_accounts);
        }
        // Execute CPI call to light-system-program
        execute_cpi_invoke(
            cpi_accounts,
            cpi_bytes,
            tree_pubkeys.as_slice(),
            with_sol_pool,
            system_accounts.sol_decompression_recipient.map(|x| x.key()),
            system_accounts.cpi_context.map(|x| *x.key()),
            false,
        )?;
    } else if let Some(system_accounts) = validated_accounts.write_to_cpi_context_system.as_ref() {
        if with_sol_pool {
            unimplemented!("")
        }
        // Execute CPI call to light-system-program
        execute_cpi_invoke(
            &accounts[1..4],
            cpi_bytes,
            &[],
            false,
            None,
            Some(*system_accounts.cpi_context.key()),
            true,
        )?;
    } else {
        unreachable!()
    }
    Ok(())
}

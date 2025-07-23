use anchor_lang::prelude::ProgramError;
use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly;
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_zero_copy::{borsh::Deserialize, ZeroCopyNew};
use pinocchio::account_info::AccountInfo;

use crate::{
    multi_transfer::{
        accounts::MultiTransferValidatedAccounts, change_account::process_change_lamports,
        cpi::allocate_cpi_bytes, native_compression::process_token_compression,
        sum_check::sum_check_multi_mint, token_inputs::set_input_compressed_accounts,
        token_outputs::set_output_compressed_accounts,
    },
    shared::cpi::execute_cpi_invoke,
};
use anchor_compressed_token::check_cpi_context;
use light_ctoken_types::{
    context::TokenContext,
    instructions::multi_transfer::{
        validate_instruction_data, CompressedTokenInstructionDataMultiTransfer,
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
pub fn process_multi_transfer(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Parse instruction data first to determine optional accounts
    let (inputs, _) = CompressedTokenInstructionDataMultiTransfer::zero_copy_at(instruction_data)
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
    msg!("with_sol_pool {}", with_sol_pool);
    let with_cpi_context = inputs.cpi_context.is_some();

    // Skip first account (light-system-program) and validate remaining accounts
    let (validated_accounts, packed_accounts) = MultiTransferValidatedAccounts::validate_and_parse(
        &accounts[MultiTransferValidatedAccounts::CPI_ACCOUNTS_OFFSET..],
        with_sol_pool,
        with_cpi_context,
    )?;
    use anchor_lang::solana_program::msg;
    // Validate instruction data consistency
    validate_instruction_data(&inputs)?;
    msg!("validate_instruction_data");
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
        inputs.cpi_context,
    )?;

    msg!("pre set_input_compressed_accounts");

    // Process input compressed accounts
    set_input_compressed_accounts(
        &mut cpi_instruction_struct,
        &mut context,
        &inputs,
        &packed_accounts,
    )?;
    msg!("pre sum_check_multi_mint");
    bench_sbf_end!("t_context_and_check_sig");
    bench_sbf_start!("t_sum_check");
    sum_check_multi_mint(
        &inputs.in_token_data,
        &inputs.out_token_data,
        inputs.compressions.as_deref(),
    )
    .map_err(|e| ProgramError::Custom(e as u32))?;
    bench_sbf_end!("t_sum_check");
    msg!("pre set_output_compressed_accounts");

    // Process output compressed accounts
    set_output_compressed_accounts(
        &mut cpi_instruction_struct,
        &mut context,
        &inputs,
        &packed_accounts,
    )?;
    bench_sbf_end!("t_create_output_compressed_accounts");
    //msg!("cpi_instruction_struct {:?}", cpi_instruction_struct);

    msg!("pre process_change_lamports");
    process_change_lamports(
        &inputs,
        &packed_accounts,
        cpi_instruction_struct,
        total_input_lamports,
        total_output_lamports,
    )?;
    // Process token compressions/decompressions
    // TODO: support spl
    process_token_compression(&inputs, &packed_accounts)?;

    // Get CPI accounts slice and tree accounts for light-system-program invocation
    let (cpi_accounts, tree_pubkeys) =
        validated_accounts.cpi_accounts(accounts, &inputs, &packed_accounts);
    // Debug prints keep for now.
    {
        let solana_tree_accounts = tree_pubkeys
            .iter()
            .map(|&x| solana_pubkey::Pubkey::new_from_array(*x))
            .collect::<Vec<_>>();
        msg!("solana_tree_accounts {:?}", solana_tree_accounts);
        let _cpi_accounts = cpi_accounts
            .iter()
            .map(|x| solana_pubkey::Pubkey::new_from_array(*x.key()))
            .collect::<Vec<_>>();
        msg!("cpi_accounts {:?}", _cpi_accounts);
    }
    // Execute CPI call to light-system-program
    execute_cpi_invoke(
        cpi_accounts,
        cpi_bytes,
        tree_pubkeys.as_slice(),
        with_sol_pool,
        validated_accounts.cpi_context_account.map(|x| *x.key()),
    )?;

    Ok(())
}

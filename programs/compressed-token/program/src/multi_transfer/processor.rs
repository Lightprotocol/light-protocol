use anchor_lang::prelude::{AccountInfo, ProgramError};
use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly;
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_zero_copy::{borsh::Deserialize, ZeroCopyNew};

use crate::{
    multi_transfer::{
        accounts::MultiTransferValidatedAccounts,
        assign_inputs::assign_input_compressed_accounts,
        assign_outputs::assign_output_compressed_accounts,
        change_account::process_change_lamports,
        cpi::{allocate_cpi_bytes, get_packed_cpi_accounts},
        instruction_data::{
            validate_instruction_data, CompressedTokenInstructionDataMultiTransfer,
        },
        sum_check::sum_check_multi_mint,
    },
    shared::{context::TokenContext, cpi::execute_cpi_invoke},
    LIGHT_CPI_SIGNER,
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
pub fn process_multi_transfer<'info>(
    accounts: &'info [AccountInfo<'info>],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Parse instruction data first to determine optional accounts
    let (inputs, _) = CompressedTokenInstructionDataMultiTransfer::zero_copy_at(instruction_data)
        .map_err(ProgramError::from)?;

    // Determine optional account flags from instruction data
    let with_sol_pool = inputs.compressions.is_some();
    let with_cpi_context = inputs.cpi_context.is_some();

    // Validate and parse accounts
    let (validated_accounts, packed_accounts) = MultiTransferValidatedAccounts::validate_and_parse(
        accounts,
        &crate::ID,
        with_sol_pool,
        with_cpi_context,
    )?;
    // Validate instruction data consistency
    validate_instruction_data(&inputs)?;
    bench_sbf_start!("t_context_and_check_sig");

    // Create TokenContext for hash caching
    let mut context = TokenContext::new();

    // Allocate CPI bytes and create zero-copy structure
    let (mut cpi_bytes, config) = allocate_cpi_bytes(&inputs);

    let (mut cpi_instruction_struct, _) =
        InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
            .map_err(ProgramError::from)?;

    // Set CPI signer information
    cpi_instruction_struct.bump = LIGHT_CPI_SIGNER.bump;
    cpi_instruction_struct.invoking_program_id = LIGHT_CPI_SIGNER.program_id.into();

    // Process input compressed accounts
    let total_input_lamports = assign_input_compressed_accounts(
        &mut cpi_instruction_struct,
        &mut context,
        &inputs,
        &packed_accounts,
    )?;
    bench_sbf_end!("t_context_and_check_sig");
    bench_sbf_start!("t_sum_check");
    sum_check_multi_mint(
        &inputs.in_token_data,
        &inputs.out_token_data,
        inputs.compressions.as_deref(),
    )
    .map_err(|e| ProgramError::Custom(e as u32))?;
    bench_sbf_end!("t_sum_check");

    // Process output compressed accounts
    let total_output_lamports = assign_output_compressed_accounts(
        &mut cpi_instruction_struct,
        &mut context,
        &inputs,
        &packed_accounts,
    )?;
    bench_sbf_end!("t_create_output_compressed_accounts");
    let with_sol_pool = total_input_lamports != total_output_lamports;
    process_change_lamports(
        &inputs,
        &packed_accounts,
        cpi_instruction_struct,
        total_input_lamports,
        total_output_lamports,
    )?;

    // Extract tree accounts from merkle contexts for CPI call
    let tree_accounts = get_packed_cpi_accounts(&inputs, &packed_accounts);

    // Execute CPI call to light-system-program
    execute_cpi_invoke(
        accounts,
        cpi_bytes,
        &tree_accounts,
        with_sol_pool,
        validated_accounts.cpi_context_account.map(|x| *x.key),
    )?;

    Ok(())
}

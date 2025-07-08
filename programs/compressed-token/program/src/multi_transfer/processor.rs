use anchor_lang::prelude::ProgramError;
use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly;
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_zero_copy::{borsh::Deserialize, ZeroCopyNew};
use pinocchio::account_info::AccountInfo;

use crate::{
    multi_transfer::{
        accounts::{MultiTransferValidatedAccounts, MultiTransferPackedAccounts},
        assign_inputs::assign_input_compressed_accounts,
        assign_outputs::assign_output_compressed_accounts,
        change_account::process_change_lamports,
        cpi::allocate_cpi_bytes,
        instruction_data::{
            validate_instruction_data, CompressedTokenInstructionDataMultiTransfer, ZCompressedTokenInstructionDataMultiTransfer,
        },
        native_compression::process_token_compression,
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
pub fn process_multi_transfer(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Parse instruction data first to determine optional accounts
    let (inputs, _) = CompressedTokenInstructionDataMultiTransfer::zero_copy_at(instruction_data)
        .map_err(ProgramError::from)?;

    // Determine optional account flags from instruction data
    let with_sol_pool = inputs.compressions.is_some();
    let with_cpi_context = inputs.cpi_context.is_some();

    // Skip first account (light-system-program) and validate remaining accounts
    let (validated_accounts, packed_accounts) = MultiTransferValidatedAccounts::validate_and_parse(
        &accounts[1..],
        with_sol_pool,
        with_cpi_context,
    )?;
    use anchor_lang::solana_program::msg;
    // Validate instruction data consistency
    validate_instruction_data(&inputs)?;
    msg!("validate_instruction_data");
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
    msg!("pre assign_input_compressed_accounts");

    // Process input compressed accounts
    let total_input_lamports = assign_input_compressed_accounts(
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
    msg!("pre assign_output_compressed_accounts");

    // Process output compressed accounts
    let total_output_lamports = assign_output_compressed_accounts(
        &mut cpi_instruction_struct,
        &mut context,
        &inputs,
        &packed_accounts,
    )?;
    bench_sbf_end!("t_create_output_compressed_accounts");
    let with_sol_pool = total_input_lamports != total_output_lamports;
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

    // Extract tree accounts using highest index approach
    let (tree_accounts, tree_accounts_count) = extract_tree_accounts(&inputs, &packed_accounts);

    // Calculate static accounts count after skipping index 0 (system accounts only)
    let static_accounts_count =
        8 + if with_sol_pool { 2 } else { 0 } + if with_cpi_context { 1 } else { 0 };

    // Include static CPI accounts + tree accounts based on highest index
    let cpi_accounts_end = 1 + static_accounts_count + tree_accounts_count;
    let cpi_accounts = &accounts[1..cpi_accounts_end];
    let solana_tree_accounts = tree_accounts
        .iter()
        .map(|&x| solana_pubkey::Pubkey::new_from_array(*x))
        .collect::<Vec<_>>();
    msg!("solana_tree_accounts {:?}", solana_tree_accounts);
    let _cpi_accounts = cpi_accounts
        .iter()
        .map(|x| solana_pubkey::Pubkey::new_from_array(*x.key()))
        .collect::<Vec<_>>();
    msg!("cpi_accounts {:?}", _cpi_accounts);
    // Execute CPI call to light-system-program
    execute_cpi_invoke(
        cpi_accounts,
        cpi_bytes,
        tree_accounts.as_slice(),
        with_sol_pool,
        validated_accounts.cpi_context_account.map(|x| *x.key()),
    )?;

    Ok(())
}

/// Extract tree accounts by finding the highest tree index and using it as closing offset
fn extract_tree_accounts<'a>(
    inputs: &ZCompressedTokenInstructionDataMultiTransfer,
    packed_accounts: &'a MultiTransferPackedAccounts<'a>,
) -> (Vec<&'a pinocchio::pubkey::Pubkey>, usize) {
    // Find highest tree index from input and output data to determine tree accounts range
    let mut highest_tree_index = 0u8;
    for input_data in inputs.in_token_data.iter() {
        highest_tree_index = highest_tree_index.max(input_data.merkle_context.merkle_tree_pubkey_index);
        highest_tree_index = highest_tree_index.max(input_data.merkle_context.queue_pubkey_index);
    }
    for output_data in inputs.out_token_data.iter() {
        highest_tree_index = highest_tree_index.max(output_data.merkle_tree);
    }
    
    // Tree accounts span from index 0 to highest_tree_index in remaining accounts
    let tree_accounts_count = (highest_tree_index + 1) as usize;
    
    // Extract tree account pubkeys from the determined range
    let mut tree_accounts = Vec::new();
    for i in 0..tree_accounts_count {
        if let Some(account) = packed_accounts.accounts.get(i) {
            tree_accounts.push(account.key());
        }
    }
    
    (tree_accounts, tree_accounts_count)
}

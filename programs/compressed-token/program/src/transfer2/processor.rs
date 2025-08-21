use anchor_compressed_token::{check_cpi_context, ErrorCode};
use anchor_lang::prelude::ProgramError;
use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly;
use light_ctoken_types::{
    hash_cache::HashCache,
    instructions::transfer2::{validate_instruction_data, CompressedTokenInstructionDataTransfer2},
};
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_zero_copy::{traits::ZeroCopyAt, ZeroCopyNew};
use pinocchio::account_info::AccountInfo;

use crate::{
    shared::cpi::execute_cpi_invoke,
    transfer2::{
        accounts::Transfer2Accounts, change_account::process_change_lamports,
        config::Transfer2Config, cpi::allocate_cpi_bytes,
        native_compression::process_token_compression, sum_check::sum_check_multi_mint,
        token_inputs::set_input_compressed_accounts, token_outputs::set_output_compressed_accounts,
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

    // Check CPI  context validity (multi-transfer modifies Solana account state)
    check_cpi_context(&inputs.cpi_context)?;

    // Create configuration from instruction data (replaces manual boolean derivation)
    let transfer_config = Transfer2Config::from_instruction_data(&inputs)?;

    // Validate accounts using clean config interface
    let validated_accounts = Transfer2Accounts::validate_and_parse(accounts, &transfer_config)?;
    // Validate instruction data consistency
    validate_instruction_data(&inputs)?;
    bench_sbf_start!("t_context_and_check_sig");

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

    // Create HashCache for hash caching
    let mut hash_cache = HashCache::new();

    // Process input compressed accounts
    set_input_compressed_accounts(
        &mut cpi_instruction_struct,
        &mut hash_cache,
        &inputs,
        &validated_accounts.packed_accounts,
    )?;

    // Process output compressed accounts
    set_output_compressed_accounts(
        &mut cpi_instruction_struct,
        &mut hash_cache,
        &inputs,
        &validated_accounts.packed_accounts,
    )?;
    bench_sbf_end!("t_create_output_compressed_accounts");

    process_change_lamports(
        &inputs,
        &validated_accounts.packed_accounts,
        cpi_instruction_struct,
        &transfer_config,
    )?;
    // Process token compressions/decompressions (native tokens supported, SPL framework added)
    if let Some(system) = validated_accounts.system.as_ref() {
        process_token_compression(
            &inputs,
            &validated_accounts.packed_accounts,
            system.cpi_authority_pda,
        )?;
    } else if inputs.compressions.is_some() {
        pinocchio::msg!("Compressions must not be set for write to cpi context.");
        // TODO: add correct error
        return Err(ErrorCode::OwnerMismatch.into());
    }
    bench_sbf_end!("t_context_and_check_sig");
    bench_sbf_start!("t_sum_check");
    sum_check_multi_mint(
        &inputs.in_token_data,
        &inputs.out_token_data,
        inputs.compressions.as_deref(),
    )
    .map_err(|e| ProgramError::Custom(e as u32))?;
    bench_sbf_end!("t_sum_check");
    if let Some(system_accounts) = validated_accounts.system.as_ref() {
        // Get CPI accounts slice and tree accounts for light-system-program invocation
        let (cpi_accounts, tree_pubkeys) = validated_accounts.cpi_accounts(
            accounts,
            &inputs,
            &validated_accounts.packed_accounts,
        )?;
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
        }
        // Execute CPI call to light-system-program
        execute_cpi_invoke(
            cpi_accounts,
            cpi_bytes,
            tree_pubkeys.as_slice(),
            transfer_config.sol_pool_required,
            system_accounts.sol_decompression_recipient.map(|x| x.key()),
            system_accounts.cpi_context.map(|x| *x.key()),
            false,
        )?;
    } else if let Some(system_accounts) = validated_accounts.write_to_cpi_context_system.as_ref() {
        if transfer_config.sol_pool_required {
            return Err(ErrorCode::Transfer2CpiContextWriteWithSolPool.into());
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

use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_array_map::ArrayMap;
use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly;
use light_ctoken_types::{
    hash_cache::HashCache,
    instructions::transfer2::{
        CompressedTokenInstructionDataTransfer2, ZCompressedTokenInstructionDataTransfer2,
    },
    CTokenError,
};
use light_program_profiler::profile;
use light_zero_copy::{traits::ZeroCopyAt, ZeroCopyNew};
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;

use crate::{
    shared::{convert_program_error, cpi::execute_cpi_invoke},
    transfer2::{
        accounts::Transfer2Accounts,
        compression::{close_for_compress_and_close, process_token_compression},
        config::Transfer2Config,
        cpi::allocate_cpi_bytes,
        sum_check::{sum_check_multi_mint, validate_mint_uniqueness},
        token_inputs::set_input_compressed_accounts,
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
#[profile]
pub fn process_transfer2(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Parse instruction data first to determine optional accounts
    let (inputs, _) = CompressedTokenInstructionDataTransfer2::zero_copy_at(instruction_data)
        .map_err(ProgramError::from)?;

    validate_instruction_data(&inputs)?;

    let transfer_config = Transfer2Config::from_instruction_data(&inputs)?;

    let validated_accounts = Transfer2Accounts::validate_and_parse(accounts, &transfer_config)?;

    if transfer_config.no_compressed_accounts {
        // No compressed accounts are invalidated or created in this transaction
        //  -> no need to invoke the light system program.
        process_no_system_program_cpi(&inputs, &validated_accounts)
    } else {
        process_with_system_program_cpi(accounts, &inputs, &validated_accounts, transfer_config)
    }
}

/// Validate instruction data consistency (lamports, TLV, and CPI context checks)
#[profile]
#[inline(always)]
pub fn validate_instruction_data(
    inputs: &ZCompressedTokenInstructionDataTransfer2,
) -> Result<(), CTokenError> {
    // Check maximum input accounts limit
    if inputs.in_token_data.len() > crate::shared::cpi_bytes_size::MAX_INPUT_ACCOUNTS {
        msg!(
            "Too many input accounts: {} (max allowed: {})",
            inputs.in_token_data.len(),
            crate::shared::cpi_bytes_size::MAX_INPUT_ACCOUNTS
        );
        return Err(CTokenError::TooManyInputAccounts);
    }

    if inputs.in_lamports.is_some() {
        msg!("in_lamports are unimplemented",);
        return Err(CTokenError::TokenDataTlvUnimplemented);
    }
    if inputs.out_lamports.is_some() {
        msg!("outlamports are unimplemented",);
        return Err(CTokenError::TokenDataTlvUnimplemented);
    }
    if inputs.in_tlv.is_some() {
        return Err(CTokenError::CompressedTokenAccountTlvUnimplemented);
    }
    if inputs.out_tlv.is_some() {
        return Err(CTokenError::CompressedTokenAccountTlvUnimplemented);
    }

    // Check CPI context write mode doesn't have compressions.
    // Write to cpi context must not modify any solana account state
    // in this instruction other than the cpi context account.
    if let Some(cpi_context) = inputs.cpi_context.as_ref() {
        if (cpi_context.set_context() || cpi_context.first_set_context())
            && inputs.compressions.is_some()
        {
            msg!("Compressions not allowed when writing to CPI context");
            return Err(CTokenError::InvalidInstructionData);
        }
    }

    Ok(())
}

#[profile]
#[inline(always)]
fn process_no_system_program_cpi(
    inputs: &ZCompressedTokenInstructionDataTransfer2,
    validated_accounts: &Transfer2Accounts,
) -> Result<(), ProgramError> {
    let fee_payer = validated_accounts
        .compressions_only_fee_payer
        .ok_or(ErrorCode::CompressionsOnlyMissingFeePayer)?;
    let cpi_authority_pda = validated_accounts
        .compressions_only_cpi_authority_pda
        .ok_or(ErrorCode::CompressionsOnlyMissingCpiAuthority)?;

    let compressions = inputs
        .compressions
        .as_ref()
        .ok_or(ErrorCode::NoInputsProvided)?;

    let mint_map: ArrayMap<u8, u64, 5> =
        sum_check_multi_mint(&[], &[], Some(compressions.as_slice()))
            .map_err(|e| ProgramError::Custom(e as u32 + 6000))?;

    // Validate mint uniqueness
    validate_mint_uniqueness(&mint_map, &validated_accounts.packed_accounts)
        .map_err(|e| ProgramError::Custom(e as u32 + 6000))?;

    process_token_compression(
        fee_payer,
        inputs,
        &validated_accounts.packed_accounts,
        cpi_authority_pda,
    )?;

    close_for_compress_and_close(compressions.as_slice(), validated_accounts)?;

    Ok(())
}

#[profile]
#[inline(always)]
fn process_with_system_program_cpi(
    accounts: &[AccountInfo],
    inputs: &ZCompressedTokenInstructionDataTransfer2,
    validated_accounts: &Transfer2Accounts,
    transfer_config: Transfer2Config,
) -> Result<(), ProgramError> {
    // Allocate CPI bytes for zero-copy structure
    let (mut cpi_bytes, config) = allocate_cpi_bytes(inputs).map_err(convert_program_error)?;

    // Create zero copy to populate cpi bytes.
    let (mut cpi_instruction_struct, remaining_bytes) =
        InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
            .map_err(ProgramError::from)?;
    assert!(remaining_bytes.is_empty());

    cpi_instruction_struct.initialize(
        crate::LIGHT_CPI_SIGNER.bump,
        &crate::LIGHT_CPI_SIGNER.program_id.into(),
        inputs.proof,
        &inputs.cpi_context,
    )?;

    // Create HashCache to cache hashed pubkeys.
    let mut hash_cache = HashCache::new();

    // Process input compressed accounts.
    set_input_compressed_accounts(
        &mut cpi_instruction_struct,
        &mut hash_cache,
        inputs,
        &validated_accounts.packed_accounts,
    )?;

    // Process output compressed accounts.
    set_output_compressed_accounts(
        &mut cpi_instruction_struct,
        &mut hash_cache,
        inputs,
        &validated_accounts.packed_accounts,
    )?;

    // Perform sum check and get mint map
    let mint_map = sum_check_multi_mint(
        &inputs.in_token_data,
        &inputs.out_token_data,
        inputs.compressions.as_deref(),
    )
    .map_err(|e| ProgramError::Custom(e as u32 + 6000))?;

    // Validate mint uniqueness
    validate_mint_uniqueness(&mint_map, &validated_accounts.packed_accounts)
        .map_err(|e| ProgramError::Custom(e as u32 + 6000))?;

    if let Some(system_accounts) = validated_accounts.system.as_ref() {
        // Process token compressions/decompressions/close_and_compress
        process_token_compression(
            system_accounts.fee_payer,
            inputs,
            &validated_accounts.packed_accounts,
            system_accounts.cpi_authority_pda,
        )?;

        // Get CPI accounts slice and tree accounts for light-system-program invocation
        let (cpi_accounts, tree_pubkeys) =
            validated_accounts.cpi_accounts(accounts, &validated_accounts.packed_accounts)?;

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

        // Close ctoken accounts at the end of the instruction.
        if let Some(compressions) = inputs.compressions.as_ref() {
            close_for_compress_and_close(compressions.as_slice(), validated_accounts)?;
        }
    } else if let Some(system_accounts) = validated_accounts.write_to_cpi_context_system.as_ref() {
        // CPI context write mode expects exactly 4 accounts:
        // 0 - light-system-program - skip
        // 1 - fee_payer
        // 2 - cpi_authority_pda
        // 3 - cpi_context
        if accounts.len() != 4 {
            return Err(ErrorCode::Transfer2CpiContextWriteInvalidAccess.into());
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

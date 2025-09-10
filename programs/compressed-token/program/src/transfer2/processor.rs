use anchor_compressed_token::{check_cpi_context, ErrorCode};
use anchor_lang::prelude::ProgramError;
use arrayvec::ArrayVec;
use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly;
use light_ctoken_types::{
    hash_cache::HashCache,
    instructions::transfer2::{
        CompressedTokenInstructionDataTransfer2, ZCompressedTokenInstructionDataTransfer2,
        ZCompressionMode,
    },
    CTokenError,
};
use light_profiler::profile;
use light_zero_copy::{traits::ZeroCopyAt, ZeroCopyNew};
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;

use crate::{
    close_token_account::{accounts::CloseTokenAccountAccounts, processor::close_token_account},
    shared::cpi::execute_cpi_invoke,
    transfer2::{
        accounts::Transfer2Accounts,
        config::Transfer2Config,
        cpi::allocate_cpi_bytes,
        native_compression::process_token_compression,
        sum_check::{sum_check_multi_mint, sum_compressions},
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

    // Check CPI  context validity (multi-transfer modifies Solana account state)
    check_cpi_context(&inputs.cpi_context)?;

    // Validate instruction data consistency
    validate_instruction_data(&inputs)?;

    // Create configuration from instruction data (replaces manual boolean derivation)
    let transfer_config = Transfer2Config::from_instruction_data(&inputs)?;

    // Validate accounts using clean config interface
    let validated_accounts = Transfer2Accounts::validate_and_parse(accounts, &transfer_config)?;
    // Process token compressions/decompressions (native tokens supported, SPL framework added)
    if let Some(system) = validated_accounts.system.as_ref() {
        process_token_compression(
            system.fee_payer,
            &inputs,
            &validated_accounts.packed_accounts,
            system.cpi_authority_pda,
        )?;
    } else if let Some(cpi_authority_pda) = validated_accounts
        .decompressed_only_cpi_authority_pda
        .as_ref()
    {
        process_token_compression(
            &validated_accounts.packed_accounts.accounts[0], // TODO: add fee payer for decompressed only instructions
            &inputs,
            &validated_accounts.packed_accounts,
            cpi_authority_pda,
        )?;
    } else if inputs.compressions.is_some() && !transfer_config.no_compressed_accounts {
        pinocchio::msg!("Compressions must not be set for write to cpi context.");
        // TODO: add correct error
        return Err(ErrorCode::OwnerMismatch.into());
    }
    // No compressed accounts are invalidated or created in this transaction
    //  -> no need to invoke the light system program.
    if transfer_config.no_compressed_accounts {
        // Close ctoken accounts at the end of the instruction.
        if let Some(compressions) = inputs.compressions.as_ref() {
            // ArrayVec with 5 entries: (mint_index, sum)
            let mut mint_sums: ArrayVec<(u8, u64), 5> = ArrayVec::new();
            sum_compressions(compressions, &mut mint_sums)?;
            for compression in compressions
                .iter()
                .filter(|c| c.mode == ZCompressionMode::CompressAndClose)
            {
                let token_account_info = validated_accounts.packed_accounts.get_u8(
                    compression.source_or_recipient,
                    "CompressAndClose: source_or_recipient",
                )?;
                let destination = validated_accounts.packed_accounts.get_u8(
                    compression.get_rent_recipient_index()?,
                    "CompressAndClose: destination",
                )?;
                let authority = validated_accounts
                    .packed_accounts
                    .get_u8(compression.authority, "CompressAndClose: authority")?;
                close_token_account(&CloseTokenAccountAccounts {
                    token_account: token_account_info,
                    destination,
                    authority,
                })?;
            }
        }
        Ok(())
    } else {
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

        // process_change_lamports(
        //     &inputs,
        //     &validated_accounts.packed_accounts,
        //     cpi_instruction_struct,
        //     &transfer_config,
        // )?;

        sum_check_multi_mint(
            &inputs.in_token_data,
            &inputs.out_token_data,
            inputs.compressions.as_deref(),
        )
        .map_err(|e| ProgramError::Custom(e as u32))?;

        if let Some(system_accounts) = validated_accounts.system.as_ref() {
            // Get CPI accounts slice and tree accounts for light-system-program invocation
            let (cpi_accounts, tree_pubkeys) =
                validated_accounts.cpi_accounts(accounts, &validated_accounts.packed_accounts)?;
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

            // Close ctoken accounts at the end of the instruction.
            if let Some(compressions) = inputs.compressions.as_ref() {
                for compression in compressions
                    .iter()
                    .filter(|c| c.mode == ZCompressionMode::CompressAndClose)
                {
                    let token_account_info = validated_accounts.packed_accounts.get_u8(
                        compression.source_or_recipient,
                        "CompressAndClose: source_or_recipient",
                    )?;
                    let destination = validated_accounts.packed_accounts.get_u8(
                        compression.get_rent_recipient_index()?,
                        "CompressAndClose: destination",
                    )?;
                    let authority = validated_accounts
                        .packed_accounts
                        .get_u8(compression.authority, "CompressAndClose: authority")?;
                    close_token_account(&CloseTokenAccountAccounts {
                        token_account: token_account_info,
                        destination,
                        authority,
                    })?;
                }
            }
        } else if let Some(system_accounts) =
            validated_accounts.write_to_cpi_context_system.as_ref()
        {
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
}

/// Validate instruction data consistency (lamports and TLV checks)
#[profile]
#[inline(always)]
pub fn validate_instruction_data(
    inputs: &ZCompressedTokenInstructionDataTransfer2,
) -> Result<(), CTokenError> {
    if let Some(ref in_lamports) = inputs.in_lamports {
        if in_lamports.len() != inputs.in_token_data.len() {
            msg!(
                "in_lamports {} != inputs in_token_data {}",
                in_lamports.len(),
                inputs.in_token_data.len()
            );
            return Err(CTokenError::InputAccountsLamportsLengthMismatch);
        }
    }
    if let Some(ref out_lamports) = inputs.out_lamports {
        if out_lamports.len() != inputs.out_token_data.len() {
            msg!(
                "outlamports {} != inputs out_token_data {}",
                out_lamports.len(),
                inputs.out_token_data.len()
            );
            return Err(CTokenError::OutputAccountsLamportsLengthMismatch);
        }
    }
    if inputs.in_tlv.is_some() {
        return Err(CTokenError::CompressedTokenAccountTlvUnimplemented);
    }
    if inputs.out_tlv.is_some() {
        return Err(CTokenError::CompressedTokenAccountTlvUnimplemented);
    }
    Ok(())
}

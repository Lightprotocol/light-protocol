use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use borsh::BorshSerialize;
use light_compressed_account::instruction_data::data::ZOutputCompressedAccountWithPackedContextMut;
use light_compressible::rent::get_rent_exemption_lamports;
use light_ctoken_interface::{
    hash_cache::HashCache,
    instructions::mint_action::ZMintActionCompressedInstructionData,
    state::{CompressedMint, ExtensionStruct},
};
use light_hasher::{sha256::Sha256BE, Hasher};
use light_program_profiler::profile;
use pinocchio::sysvars::{clock::Clock, rent::Rent, Sysvar};
use spl_pod::solana_msg::msg;

use crate::{
    constants::COMPRESSED_MINT_DISCRIMINATOR,
    mint_action::{
        accounts::{AccountsConfig, MintActionAccounts},
        actions::process_actions,
        queue_indices::QueueIndices,
    },
    shared::{convert_program_error, transfer_lamports::transfer_lamports},
};

/// Processes the output compressed mint account and returns the modified mint for CMint sync.
#[profile]
pub fn process_output_compressed_account<'a>(
    parsed_instruction_data: &ZMintActionCompressedInstructionData,
    validated_accounts: &MintActionAccounts,
    output_compressed_accounts: &'a mut [ZOutputCompressedAccountWithPackedContextMut<'a>],
    hash_cache: &mut HashCache,
    queue_indices: &QueueIndices,
    mut compressed_mint: CompressedMint,
    accounts_config: &AccountsConfig,
) -> Result<(), ProgramError> {
    let (mint_account, token_accounts) = split_mint_and_token_accounts(output_compressed_accounts);

    process_actions(
        parsed_instruction_data,
        validated_accounts,
        &mut token_accounts.iter_mut(),
        hash_cache,
        queue_indices,
        &validated_accounts.packed_accounts,
        &mut compressed_mint,
    )?;

    // AUTO-SYNC OUTPUT: If CMint account was passed, update it with new state
    // SKIP if CompressAndCloseCMint action is present (CMint is being closed, not synced)
    if let Some(cmint_account) = validated_accounts.get_cmint() {
        if !accounts_config.has_compress_and_close_cmint_action {
            // Check if CMint has Compressible extension and handle top-up
            if let Some(ref mut extensions) = compressed_mint.extensions {
                if let Some(ExtensionStruct::Compressible(ref mut compression_info)) = extensions
                    .iter_mut()
                    .find(|e| matches!(e, ExtensionStruct::Compressible(_)))
                {
                    // Get current slot for top-up calculation
                    let current_slot = Clock::get()
                        .map_err(|_| ProgramError::UnsupportedSysvar)?
                        .slot;

                    let num_bytes = cmint_account.data_len() as u64;
                    let current_lamports = cmint_account.lamports();
                    let rent_exemption = get_rent_exemption_lamports(num_bytes)
                        .map_err(|_| ErrorCode::CMintTopUpCalculationFailed)?;

                    // Calculate top-up amount
                    let top_up = compression_info
                        .calculate_top_up_lamports(
                            num_bytes,
                            current_slot,
                            current_lamports,
                            rent_exemption,
                        )
                        .map_err(|_| ErrorCode::CMintTopUpCalculationFailed)?;

                    if top_up > 0 {
                        let fee_payer = validated_accounts
                            .executing
                            .as_ref()
                            .map(|exec| exec.system.fee_payer)
                            .ok_or(ProgramError::NotEnoughAccountKeys)?;
                        transfer_lamports(top_up, fee_payer, cmint_account)
                            .map_err(convert_program_error)?;
                    }

                    // Update last_claimed_slot to current slot
                    compression_info.last_claimed_slot = current_slot;
                }
            }

            let serialized = compressed_mint
                .try_to_vec()
                .map_err(|_| ErrorCode::MintActionOutputSerializationFailed)?;
            let required_size = serialized.len();

            // Resize if needed (e.g., metadata extensions added)
            if cmint_account.data_len() < required_size {
                cmint_account
                    .resize(required_size)
                    .map_err(|_| ErrorCode::CMintResizeFailed)?;

                // Transfer additional lamports for rent if resized
                let rent = Rent::get().map_err(|_| ProgramError::UnsupportedSysvar)?;
                let required_lamports = rent.minimum_balance(required_size);
                if cmint_account.lamports() < required_lamports {
                    let fee_payer = validated_accounts
                        .executing
                        .as_ref()
                        .map(|exec| exec.system.fee_payer)
                        .ok_or(ProgramError::NotEnoughAccountKeys)?;
                    transfer_lamports(
                        required_lamports - cmint_account.lamports(),
                        fee_payer,
                        cmint_account,
                    )
                    .map_err(convert_program_error)?;
                }
            }

            let mut cmint_data = cmint_account
                .try_borrow_mut_data()
                .map_err(|_| ProgramError::AccountBorrowFailed)?;
            if cmint_data.len() < serialized.len() {
                msg!(
                    "CMint account too small: {} < {}",
                    cmint_data.len(),
                    serialized.len()
                );
                return Err(ErrorCode::CMintResizeFailed.into());
            }
            cmint_data[..serialized.len()].copy_from_slice(&serialized);
        }
    }

    // When decompressed (CMint is source of truth), use zero values
    let cmint_is_source_of_truth = accounts_config.cmint_is_source_of_truth();
    let compressed_account_data = mint_account
        .compressed_account
        .data
        .as_mut()
        .ok_or(ErrorCode::MintActionOutputSerializationFailed)?;

    let (discriminator, data_hash) = if cmint_is_source_of_truth {
        // Zero sentinel values indicate "data lives in CMint"
        // Data buffer is empty (data_len=0), no serialization needed
        ([0u8; 8], [0u8; 32])
    } else {
        // Serialize compressed mint for compressed account
        let data = compressed_mint
            .try_to_vec()
            .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;
        if data.len() != compressed_account_data.data.len() {
            msg!(
                "Data allocation for output mint account is wrong: {} != {}",
                data.len(),
                compressed_account_data.data.len()
            );
            return Err(ProgramError::InvalidAccountData);
        }

        // Copy data and compute hash
        compressed_account_data
            .data
            .copy_from_slice(data.as_slice());
        (
            COMPRESSED_MINT_DISCRIMINATOR,
            Sha256BE::hash(compressed_account_data.data)?,
        )
    };

    // Set mint output compressed account fields except the data.
    mint_account.set(
        crate::LIGHT_CPI_SIGNER.program_id.into(),
        0,
        Some(parsed_instruction_data.compressed_address),
        queue_indices.output_queue_index,
        discriminator,
        data_hash,
    )?;

    Ok(())
}

#[inline(always)]
fn split_mint_and_token_accounts<'a>(
    output_compressed_accounts: &'a mut [ZOutputCompressedAccountWithPackedContextMut<'a>],
) -> (
    &'a mut ZOutputCompressedAccountWithPackedContextMut<'a>,
    &'a mut [ZOutputCompressedAccountWithPackedContextMut<'a>],
) {
    if output_compressed_accounts.len() == 1 {
        (&mut output_compressed_accounts[0], &mut [])
    } else {
        let (mint_account, token_accounts) = output_compressed_accounts.split_at_mut(1);
        (&mut mint_account[0], token_accounts)
    }
}

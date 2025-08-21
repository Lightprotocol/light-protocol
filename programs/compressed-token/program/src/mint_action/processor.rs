use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_compressed_account::instruction_data::{
    data::ZOutputCompressedAccountWithPackedContextMut,
    with_readonly::{
        InstructionDataInvokeCpiWithReadOnly, ZInstructionDataInvokeCpiWithReadOnlyMut,
    },
};
use light_ctoken_types::{
    hash_cache::HashCache,
    instructions::{
        extensions::ZExtensionInstructionData,
        mint_action::{
            MintActionCompressedInstructionData, ZAction, ZMintActionCompressedInstructionData,
        },
    },
    state::ZCompressedMintMut,
};
use light_sdk::instruction::PackedMerkleContext;
use light_zero_copy::{traits::ZeroCopyAt, ZeroCopyNew};
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;
use spl_token::solana_program::log::sol_log_compute_units;

use crate::{
    mint_action::{
        accounts::{AccountsConfig, MintActionAccounts},
        create_mint::process_create_mint_action,
        create_spl_mint::process_create_spl_mint_action,
        mint_input::create_input_compressed_mint_account,
        mint_output::process_output_compressed_account,
        mint_to::process_mint_to_action,
        mint_to_decompressed::process_mint_to_decompressed_action,
        queue_indices::QueueIndices,
        update_authority::validate_and_update_authority,
        update_metadata::{
            process_remove_metadata_key_action, process_update_metadata_authority_action,
            process_update_metadata_field_action,
        },
        zero_copy_config::{cleanup_removed_metadata_keys, get_zero_copy_configs},
    },
    shared::cpi::execute_cpi_invoke,
};

/// Steps:
/// 1. parse instruction data
/// 2.
///
///
/// Checks:
/// 1.
/// check mint_signer (compressed mint randomness) is signer
/// 2.
pub fn process_mint_action(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    sol_log_compute_units();
    // 1. parse instruction data
    // 677 CU
    let (mut parsed_instruction_data, _) =
        MintActionCompressedInstructionData::zero_copy_at(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;

    sol_log_compute_units();
    // 112 CU write to cpi contex
    let accounts_config = AccountsConfig::new(&parsed_instruction_data);
    // Validate and parse
    let validated_accounts = MintActionAccounts::validate_and_parse(
        accounts,
        &accounts_config,
        &parsed_instruction_data.mint.spl_mint.into(),
        parsed_instruction_data.token_pool_index,
        parsed_instruction_data.token_pool_bump,
    )?;

    let (config, mut cpi_bytes, mint_size_config) =
        get_zero_copy_configs(&mut parsed_instruction_data)?;
    sol_log_compute_units();
    let (mut cpi_instruction_struct, _) =
        InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
            .map_err(ProgramError::from)?;
    cpi_instruction_struct.initialize(
        crate::LIGHT_CPI_SIGNER.bump,
        &crate::LIGHT_CPI_SIGNER.program_id.into(),
        parsed_instruction_data.proof,
        &parsed_instruction_data.cpi_context,
    )?;
    if !accounts_config.write_to_cpi_context
        && !parsed_instruction_data.prove_by_index()
        && parsed_instruction_data.proof.is_none()
    {
        return Err(ErrorCode::MintActionProofMissing.into());
    }

    sol_log_compute_units();
    let mut hash_cache = HashCache::new();
    // TODO: unify with cpi context
    let queue_indices = QueueIndices::new(&parsed_instruction_data, &validated_accounts)?;
    set_compressed_lamports(
        &parsed_instruction_data.actions,
        &mut cpi_instruction_struct,
    )?;
    // If create mint
    // 1. derive spl mint pda
    // 2. set create address
    // else
    // 1. set input compressed mint account
    if parsed_instruction_data.create_mint() {
        process_create_mint_action(
            &parsed_instruction_data,
            &validated_accounts,
            &mut cpi_instruction_struct,
            // Used for the address tree when creating the mint since
            // we don't have an input compressed account in this case.
            queue_indices.in_tree_index,
        )?;
    } else {
        // Process input compressed mint account
        create_input_compressed_mint_account(
            &mut cpi_instruction_struct.input_compressed_accounts[0],
            &mut hash_cache,
            &parsed_instruction_data,
            PackedMerkleContext {
                merkle_tree_pubkey_index: queue_indices.in_tree_index,
                queue_pubkey_index: queue_indices.in_queue_index,
                leaf_index: parsed_instruction_data.leaf_index.into(),
                prove_by_index: parsed_instruction_data.prove_by_index(),
            },
        )?;
    }

    // Clean up removed metadata keys from instruction data after input hash is calculated
    // This handles both idempotent and non-idempotent cases internally
    cleanup_removed_metadata_keys(&mut parsed_instruction_data)?;

    process_output_compressed_account(
        &parsed_instruction_data,
        &validated_accounts,
        &accounts_config,
        &mut cpi_instruction_struct.output_compressed_accounts,
        mint_size_config,
        &mut hash_cache,
        &queue_indices,
    )?;

    sol_log_compute_units();

    let cpi_accounts = validated_accounts.get_cpi_accounts(queue_indices.deduplicated, accounts)?;
    if let Some(executing) = validated_accounts.executing.as_ref() {
        // Execute CPI to light-system-program
        execute_cpi_invoke(
            cpi_accounts,
            cpi_bytes,
            validated_accounts
                .tree_pubkeys(queue_indices.deduplicated)
                .as_slice(),
            accounts_config.with_lamports,
            None,
            executing.system.cpi_context.map(|x| *x.key()),
            false, // write to cpi context account
        )
    } else {
        if validated_accounts.write_to_cpi_context_system.is_none() {
            return Err(ErrorCode::CpiContextExpected.into());
        }
        execute_cpi_invoke(
            cpi_accounts,
            cpi_bytes,
            &[],
            false, // no sol_pool_pda for create_compressed_mint
            None,
            validated_accounts
                .write_to_cpi_context_system
                .as_ref()
                .map(|x| *x.cpi_context.key()),
            true, // TODO: make const generic
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub fn process_actions<'a>(
    parsed_instruction_data: &ZMintActionCompressedInstructionData,
    validated_accounts: &MintActionAccounts,
    accounts_config: &AccountsConfig,
    cpi_instruction_struct: &'a mut [ZOutputCompressedAccountWithPackedContextMut<'a>],
    hash_cache: &mut HashCache,
    queue_indices: &QueueIndices,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    compressed_mint: &mut ZCompressedMintMut<'a>,
) -> Result<(), ProgramError> {
    // Centralized authority validation - extract and validate authorities at the start
    let signer_key = *validated_accounts.authority.key();
    msg!(
        "parsed_instruction_data.mint.mint_authority {:?}",
        parsed_instruction_data
            .mint
            .mint_authority
            .as_ref()
            .map(|x| solana_pubkey::Pubkey::new_from_array((**x).into()))
    );
    msg!(
        "signer_key {:?}",
        solana_pubkey::Pubkey::new_from_array(signer_key)
    );
    // Validate mint authority
    let mut _validated_mint_authority = None;
    if let Some(current_mint_auth) = parsed_instruction_data.mint.mint_authority.as_ref() {
        if current_mint_auth.to_bytes() == signer_key {
            _validated_mint_authority = Some(**current_mint_auth);
            msg!("Mint authority validated: signer matches current mint authority");
        } else {
            msg!("Mint authority validation failed: signer does not match current mint authority");
        }
    }

    // Start metadata authority with same value as mint authority
    let mut validated_metadata_authority = Some(light_compressed_account::Pubkey::from(signer_key));
    msg!(
        "validated_metadata_authority {:?}",
        validated_metadata_authority
    );
    for (index, action) in parsed_instruction_data.actions.iter().enumerate() {
        msg!("Action {}", index);
        match action {
            ZAction::MintTo(action) => {
                msg!("Processing MintTo action");
                let new_supply = process_mint_to_action(
                    action,
                    compressed_mint,
                    validated_accounts,
                    accounts_config,
                    cpi_instruction_struct,
                    hash_cache,
                    parsed_instruction_data.mint.spl_mint,
                    queue_indices.out_token_queue_index,
                    parsed_instruction_data
                        .mint
                        .mint_authority
                        .as_ref()
                        .map(|a| **a),
                )?;
                compressed_mint.supply = new_supply.into();
            }
            ZAction::UpdateMintAuthority(update_action) => {
                msg!("Processing UpdateMintAuthority action");
                validate_and_update_authority(
                    &mut compressed_mint.mint_authority,
                    parsed_instruction_data
                        .mint
                        .mint_authority
                        .as_ref()
                        .map(|a| **a),
                    update_action,
                    validated_accounts.authority.key(),
                    "mint authority",
                )?;
            }
            ZAction::UpdateFreezeAuthority(update_action) => {
                msg!("Processing UpdateFreezeAuthority action");
                validate_and_update_authority(
                    &mut compressed_mint.freeze_authority,
                    parsed_instruction_data
                        .mint
                        .freeze_authority
                        .as_ref()
                        .map(|a| **a),
                    update_action,
                    validated_accounts.authority.key(),
                    "freeze authority",
                )?;
            }
            ZAction::CreateSplMint(create_spl_action) => {
                msg!("Processing CreateSplMint action");
                process_create_spl_mint_action(
                    create_spl_action,
                    validated_accounts,
                    &parsed_instruction_data.mint,
                )?;
            }
            ZAction::MintToDecompressed(mint_to_decompressed_action) => {
                msg!("Processing MintToDecompressed action");
                let new_supply = process_mint_to_decompressed_action(
                    mint_to_decompressed_action,
                    u64::from(compressed_mint.supply),
                    compressed_mint,
                    validated_accounts,
                    accounts_config,
                    packed_accounts,
                    parsed_instruction_data.mint.spl_mint,
                    parsed_instruction_data
                        .mint
                        .mint_authority
                        .as_ref()
                        .map(|a| **a),
                )?;
                compressed_mint.supply = new_supply.into();
                msg!("done Processing MintToDecompressed action");
            }
            ZAction::UpdateMetadataField(update_metadata_action) => {
                msg!("Processing UpdateMetadataField action - START");
                msg!(
                    "UpdateMetadataField: extension_index={}, field_type={}, value_len={}",
                    update_metadata_action.extension_index,
                    update_metadata_action.field_type,
                    update_metadata_action.value.len()
                );
                process_update_metadata_field_action(
                    update_metadata_action,
                    compressed_mint,
                    &validated_metadata_authority,
                )?;
                msg!("Processing UpdateMetadataField action - COMPLETE");
            }
            ZAction::UpdateMetadataAuthority(update_metadata_authority_action) => {
                msg!("Processing UpdateMetadataAuthority action");
                let old_authority = parsed_instruction_data
                    .mint
                    .extensions
                    .as_ref()
                    .and_then(|extensions| {
                        extensions.get(update_metadata_authority_action.extension_index as usize)
                    })
                    .and_then(|ext| match ext {
                        ZExtensionInstructionData::TokenMetadata(metadata_extension) => {
                            metadata_extension.update_authority
                        }
                        _ => None,
                    });
                process_update_metadata_authority_action(
                    update_metadata_authority_action,
                    compressed_mint,
                    &old_authority,
                    &mut validated_metadata_authority,
                )?;
            }
            ZAction::RemoveMetadataKey(remove_metadata_key_action) => {
                msg!("Processing RemoveMetadataKey action");
                process_remove_metadata_key_action(
                    remove_metadata_key_action,
                    compressed_mint,
                    &validated_metadata_authority,
                )?;
            }
        }
    }

    Ok(())
}

/// Sets compressed lamports by summing all MintTo action lamports
fn set_compressed_lamports(
    actions: &[ZAction],
    cpi_instruction_struct: &mut ZInstructionDataInvokeCpiWithReadOnlyMut<'_>,
) -> Result<(), ProgramError> {
    let mut compressed_lamports: u64 = 0;
    for action in actions.iter() {
        if let ZAction::MintTo(action) = action {
            if let Some(lamports) = action.lamports {
                compressed_lamports = compressed_lamports
                    .checked_add(u64::from(*lamports))
                    .ok_or(ProgramError::InvalidInstructionData)?;
            }
        }
    }
    cpi_instruction_struct.compress_or_decompress_lamports = compressed_lamports.into();
    cpi_instruction_struct.is_compress = if compressed_lamports > 0 { 1 } else { 0 };
    Ok(())
}

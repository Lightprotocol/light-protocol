use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly;
use light_ctoken_types::{
    hash_cache::HashCache, instructions::mint_action::MintActionCompressedInstructionData,
};
use light_sdk::instruction::PackedMerkleContext;
use light_zero_copy::{traits::ZeroCopyAt, ZeroCopyNew};
use pinocchio::account_info::AccountInfo;

use crate::{
    mint_action::{
        accounts::{AccountsConfig, MintActionAccounts},
        create_mint::process_create_mint_action,
        mint_input::create_input_compressed_mint_account,
        mint_output::process_output_compressed_account,
        queue_indices::QueueIndices,
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
    // 1. parse instruction data
    // 677 CU
    let (mut parsed_instruction_data, _) =
        MintActionCompressedInstructionData::zero_copy_at(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;

    // 112 CU write to cpi contex
    let accounts_config = AccountsConfig::new(&parsed_instruction_data);
    // Validate and parse
    let validated_accounts = MintActionAccounts::validate_and_parse(
        accounts,
        &accounts_config,
        &parsed_instruction_data.mint.metadata.spl_mint.into(),
        parsed_instruction_data.token_pool_index,
        parsed_instruction_data.token_pool_bump,
    )?;

    let (config, mut cpi_bytes, output_mint_size_config) =
        get_zero_copy_configs(&mut parsed_instruction_data)?;
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

    let mut hash_cache = HashCache::new();
    let queue_indices = QueueIndices::new(&parsed_instruction_data, &validated_accounts)?;

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
            // Use the dedicated address_merkle_tree_index when creating the mint
            queue_indices.address_merkle_tree_index,
        )?;
    } else {
        // Process input compressed mint account
        create_input_compressed_mint_account(
            &mut cpi_instruction_struct.input_compressed_accounts[0],
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
        output_mint_size_config,
        &mut hash_cache,
        &queue_indices,
    )?;

    let cpi_accounts = validated_accounts.get_cpi_accounts(queue_indices.deduplicated, accounts)?;
    if let Some(executing) = validated_accounts.executing.as_ref() {
        // Execute CPI to light-system-program
        execute_cpi_invoke(
            cpi_accounts,
            cpi_bytes,
            validated_accounts
                .tree_pubkeys(queue_indices.deduplicated)
                .as_slice(),
            false, // no sol_pool_pda
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
            false, // no sol_pool_pda
            None,
            validated_accounts
                .write_to_cpi_context_system
                .as_ref()
                .map(|x| *x.cpi_context.key()),
            true, // TODO: make const generic
        )
    }
}

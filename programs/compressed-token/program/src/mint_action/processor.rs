use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly;
use light_ctoken_interface::{
    hash_cache::HashCache, instructions::mint_action::MintActionCompressedInstructionData,
    state::CompressedMint, CTokenError,
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
        zero_copy_config::get_zero_copy_configs,
    },
    shared::cpi::execute_cpi_invoke,
};

pub fn process_mint_action(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // 1. parse instruction data
    // 677 CU
    let (parsed_instruction_data, _) =
        MintActionCompressedInstructionData::zero_copy_at(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;
    // 112 CU write to cpi contex
    let accounts_config = AccountsConfig::new(&parsed_instruction_data)?;
    // Get mint pubkey from instruction data if present
    let cmint_pubkey: Option<solana_pubkey::Pubkey> = parsed_instruction_data
        .mint
        .as_ref()
        .map(|m| m.metadata.mint.into());
    // Validate and parse
    let validated_accounts = MintActionAccounts::validate_and_parse(
        accounts,
        &accounts_config,
        cmint_pubkey.as_ref(),
        parsed_instruction_data.token_pool_index,
        parsed_instruction_data.token_pool_bump,
    )?;

    // Get mint data based on source:
    // 1. Creating new mint: mint data required in instruction
    // 2. Existing compressed mint: mint data in instruction (cmint_decompressed = false)
    // 3. CMint is source of truth: read from CMint account (cmint_decompressed = true)
    let mint = if parsed_instruction_data.create_mint.is_some() {
        // Creating new mint - mint data required in instruction
        let mint_data = parsed_instruction_data
            .mint
            .as_ref()
            .ok_or(ErrorCode::MintDataRequired)?;
        CompressedMint::try_from(mint_data)?
    } else if let Some(mint_data) = parsed_instruction_data.mint.as_ref() {
        // Existing compressed mint with data in instruction
        CompressedMint::try_from(mint_data)?
    } else {
        // CMint is source of truth - read from CMint account
        let cmint_account = validated_accounts
            .get_cmint()
            .ok_or(ErrorCode::MintActionMissingCMintAccount)?;
        CompressedMint::from_account_info_checked(
            &crate::LIGHT_CPI_SIGNER.program_id,
            cmint_account,
        )?
    };

    let (config, mut cpi_bytes, _) =
        get_zero_copy_configs(&parsed_instruction_data, &accounts_config, &mint)?;
    let (mut cpi_instruction_struct, remaining_bytes) =
        InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
            .map_err(ProgramError::from)?;
    assert!(remaining_bytes.is_empty());
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
    let tokens_out_queue_exists = validated_accounts.has_tokens_out_queue();
    let queue_keys_match = validated_accounts.queue_keys_match();
    let queue_indices = QueueIndices::new(
        parsed_instruction_data.cpi_context.as_ref(),
        parsed_instruction_data.create_mint.is_some(),
        tokens_out_queue_exists,
        queue_keys_match,
        accounts_config.write_to_cpi_context,
    )?;

    // Get mint data based on instruction type:
    // 1. Creating mint: mint data from instruction (must be Some)
    // 2. Existing mint with data in instruction: use instruction data
    // 3. Existing decompressed mint (CMint): read from CMint account
    if parsed_instruction_data.create_mint.is_some() {
        // Creating new mint - mint data required in instruction
        process_create_mint_action(
            &parsed_instruction_data,
            validated_accounts
                .mint_signer
                .ok_or(CTokenError::ExpectedMintSignerAccount)
                .map_err(|_| ErrorCode::MintActionMissingExecutingAccounts)?
                .key(),
            &mut cpi_instruction_struct,
            queue_indices.address_merkle_tree_index,
        )?;
    } else {
        // Decompressed mint (CMint is source of truth) - data from CMint account
        // Set input with zero values (data lives in CMint)
        create_input_compressed_mint_account(
            &mut cpi_instruction_struct.input_compressed_accounts[0],
            &parsed_instruction_data,
            PackedMerkleContext {
                merkle_tree_pubkey_index: queue_indices.in_tree_index,
                queue_pubkey_index: queue_indices.in_queue_index,
                leaf_index: parsed_instruction_data.leaf_index.into(),
                prove_by_index: parsed_instruction_data.prove_by_index(),
            },
            &accounts_config,
        )?;
    };

    process_output_compressed_account(
        &parsed_instruction_data,
        &validated_accounts,
        &mut cpi_instruction_struct.output_compressed_accounts,
        &mut hash_cache,
        &queue_indices,
        mint,
        &accounts_config,
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
            true,
        )
    }
}

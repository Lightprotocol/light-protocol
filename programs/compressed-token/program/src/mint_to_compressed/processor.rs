use account_compression::utils::constants::NOOP_PUBKEY;
use anchor_lang::{
    prelude::{msg, AccountMeta},
    solana_program::{account_info::AccountInfo, program_error::ProgramError},
    Discriminator,
};
use arrayvec::ArrayVec;
use light_compressed_account::{
    instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly, Pubkey,
};
use light_sdk::cpi::invoke_light_system_program;
use light_sdk_types::{
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, LIGHT_SYSTEM_PROGRAM_ID, REGISTERED_PROGRAM_PDA,
};
use light_zero_copy::{borsh::Deserialize, ZeroCopyNew};
use spl_token::solana_program::log::sol_log_compute_units;
use zerocopy::little_endian::U64;

use crate::{
    mint_to_compressed::{
        accounts::MintToCompressedAccounts,
        instructions::{MintToCompressedInstructionData, ZCompressedMintInputs},
    },
    shared::cpi_bytes_size::{
        allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
    },
    LIGHT_CPI_SIGNER,
};

pub fn process_mint_to_compressed<'info>(
    program_id: Pubkey,
    accounts: &'info [AccountInfo<'info>],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    sol_log_compute_units();

    // Parse instruction data using zero-copy
    let (parsed_instruction_data, _) =
        MintToCompressedInstructionData::zero_copy_at(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;

    sol_log_compute_units();

    // Validate and parse accounts
    let validated_accounts =
        MintToCompressedAccounts::validate_and_parse(accounts, &program_id.into())?;

    // Convert to the format expected by the existing mint logic
    let compressed_mint_inputs = Some(parsed_instruction_data.compressed_mint_inputs);
    Ok(())
    // // Call the existing mint logic - this mirrors the anchor implementation
    // process_mint_to_or_compress_native(
    //     &validated_accounts,
    //     &parsed_instruction_data.public_keys.as_slice(),
    //     parsed_instruction_data.amounts.as_slice(),
    //     parsed_instruction_data.lamports,
    //     None, // index - not used for mint_to_compressed
    //     None, // bump - not used for mint_to_compressed
    //     compressed_mint_inputs,
    //     &program_id,
    // )
}

// Native implementation of process_mint_to_or_compress adapted from anchor version
fn process_mint_to_or_compress_native<'a, 'info>(
    accounts: &MintToCompressedAccounts<'info>,
    recipient_pubkeys: &[Pubkey],
    amounts: &[U64],
    lamports: Option<U64>,
    index: Option<u8>,
    bump: Option<u8>,
    compressed_mint_inputs: Option<ZCompressedMintInputs>,
    program_id: &Pubkey,
) -> Result<(), ProgramError> {
    if recipient_pubkeys.len() != amounts.len() {
        return Err(ProgramError::InvalidInstructionData);
    }

    if recipient_pubkeys.is_empty() {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Build configuration for CPI instruction data using the generalized function
    let compressed_mint_with_freeze_authority = compressed_mint_inputs
        .as_ref()
        .map(|mint_inputs| mint_inputs.compressed_mint_input.freeze_authority_is_set != 0)
        .unwrap_or(false);

    let config_input = CpiConfigInput::mint_to_compressed(
        amounts.len(),
        compressed_mint_inputs.is_some(),
        compressed_mint_with_freeze_authority,
    );

    let config = cpi_bytes_config(config_input);
    let mut cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);

    sol_log_compute_units();
    let (mut cpi_instruction_struct, _) =
        InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
            .map_err(ProgramError::from)?;
    sol_log_compute_units();

    // Populate the CPI instruction data
    // create_mint_to_compressed_cpi_data(
    //     &mut cpi_instruction_struct,
    //     recipient_pubkeys,
    //     amounts,
    //     lamports,
    //     compressed_mint_inputs,
    //     accounts,
    // )?;

    sol_log_compute_units();

    // Execute CPI to light-system-program
    execute_mint_to_compressed_cpi(accounts, cpi_bytes, program_id)
}

fn execute_mint_to_compressed_cpi<'info>(
    accounts: &MintToCompressedAccounts<'info>,
    cpi_bytes: Vec<u8>,
    program_id: &Pubkey,
) -> Result<(), ProgramError> {
    // Build account metas in the correct order for light-system-program
    let account_metas = vec![
        AccountMeta::new(*accounts.fee_payer.key, true), // fee_payer (signer, mutable)
        AccountMeta::new_readonly(LIGHT_CPI_SIGNER.cpi_signer.into(), true), // authority (cpi_authority_pda)
        AccountMeta::new_readonly(REGISTERED_PROGRAM_PDA.into(), false), // registered_program_pda
        AccountMeta::new_readonly(NOOP_PUBKEY.into(), false),            // noop_program
        AccountMeta::new_readonly(ACCOUNT_COMPRESSION_AUTHORITY_PDA.into(), false), // account_compression_authority
        AccountMeta::new_readonly(account_compression::ID, false), // account_compression_program
        AccountMeta::new_readonly((*program_id).into(), false), // invoking_program (self_program)
        AccountMeta::new_readonly(
            if let Some(sol_pool) = accounts.sol_pool_pda {
                *sol_pool.key
            } else {
                LIGHT_SYSTEM_PROGRAM_ID.into()
            },
            false,
        ), // sol_pool_pda
        AccountMeta::new_readonly(LIGHT_SYSTEM_PROGRAM_ID.into(), false), // decompression_recipient (None, using default)
        AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false), // system_program
        AccountMeta::new_readonly(LIGHT_SYSTEM_PROGRAM_ID.into(), false), // cpi_context_account (None, using default)
        AccountMeta::new(*accounts.merkle_tree.key, false),               // merkle_tree (mutable)
    ];

    let instruction = anchor_lang::solana_program::instruction::Instruction {
        program_id: LIGHT_SYSTEM_PROGRAM_ID.into(),
        accounts: account_metas,
        data: cpi_bytes,
    };

    // Collect all account infos for the CPI call
    let mut account_infos = vec![
        accounts.fee_payer.clone(),
        accounts.cpi_authority_pda.clone(),
        accounts.registered_program_pda.clone(),
        accounts.noop_program.clone(),
        accounts.account_compression_authority.clone(),
        accounts.account_compression_program.clone(),
        accounts.self_program.clone(),
    ];

    if let Some(sol_pool) = accounts.sol_pool_pda {
        account_infos.push(sol_pool.clone());
    } else {
        account_infos.push(accounts.light_system_program.clone());
    }

    account_infos.extend_from_slice(&[
        accounts.light_system_program.clone(), // decompression_recipient placeholder
        accounts.system_program.clone(),
        accounts.light_system_program.clone(), // cpi_context_account placeholder
        accounts.merkle_tree.clone(),
    ]);

    invoke_light_system_program(&account_infos, instruction, LIGHT_CPI_SIGNER.bump)?;

    Ok(())
}

use account_compression::utils::constants::NOOP_PUBKEY;
use anchor_lang::{
    prelude::{msg, AccountMeta},
    solana_program::{account_info::AccountInfo, program_error::ProgramError},
    Discriminator,
};
use arrayvec::ArrayVec;
use light_compressed_account::{
    hash_to_bn254_field_size_be,
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
    shared::{
        context::TokenContext,
        cpi_bytes_size::{
            allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
        },
        outputs::create_output_compressed_account,
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
    let _validated_accounts =
        MintToCompressedAccounts::validate_and_parse(accounts, &program_id.into())?;

    // Build configuration for CPI instruction data using the generalized function
    let compressed_mint_with_freeze_authority = parsed_instruction_data
        .compressed_mint_inputs
        .compressed_mint_input
        .freeze_authority_is_set
        != 0;

    let config_input = CpiConfigInput::mint_to_compressed(
        parsed_instruction_data.recipients.len(),
        true,
        compressed_mint_with_freeze_authority,
    );

    let config = cpi_bytes_config(config_input);
    let mut cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);

    sol_log_compute_units();
    let (cpi_instruction_struct, _) =
        InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
            .map_err(ProgramError::from)?;
    let mut context = TokenContext::new();
    let mint = parsed_instruction_data
        .compressed_mint_inputs
        .compressed_mint_input
        .spl_mint;

    let hashed_mint = hash_to_bn254_field_size_be(mint.as_ref());

    // Create output token accounts
    create_output_compressed_token_accounts(
        parsed_instruction_data,
        cpi_instruction_struct,
        &mut context,
        mint,
        hashed_mint,
    )?;
    Ok(())
}

fn create_output_compressed_token_accounts(
    parsed_instruction_data: super::instructions::ZMintToCompressedInstructionData<'_>,
    mut cpi_instruction_struct: light_compressed_account::instruction_data::with_readonly::ZInstructionDataInvokeCpiWithReadOnlyMut<'_>,
    context: &mut TokenContext,
    mint: Pubkey,
    hashed_mint: [u8; 32],
) -> Result<(), ProgramError> {
    let lamports = parsed_instruction_data
        .lamports
        .map(|lamports| u64::from(*lamports));
    for (recipient, output_account) in parsed_instruction_data
        .recipients
        .iter()
        .zip(cpi_instruction_struct.output_compressed_accounts.iter_mut())
    {
        let output_delegate = None;

        create_output_compressed_account(
            output_account,
            context,
            recipient.recipient,
            output_delegate,
            recipient.amount,
            lamports,
            mint,
            &hashed_mint,
            0,
        )?;
    }
    Ok(())
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

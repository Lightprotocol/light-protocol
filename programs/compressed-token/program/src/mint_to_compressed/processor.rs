use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::{
    instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly, Pubkey,
};
use light_ctoken_types::{
    context::TokenContext, instructions::mint_to_compressed::MintToCompressedInstructionData,
    state::CompressedMintConfig,
};
use light_sdk::instruction::PackedMerkleContext;
use light_zero_copy::{borsh::Deserialize, ZeroCopyNew};
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;
use spl_token::solana_program::log::sol_log_compute_units;
use zerocopy::little_endian::U64;

use crate::{
    mint::{
        mint_input::create_input_compressed_mint_account,
        mint_output::create_output_compressed_mint_account,
    },
    mint_to_compressed::accounts::MintToCompressedAccounts,
    shared::{
        cpi::execute_cpi_invoke,
        cpi_bytes_size::{
            allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
        },
        mint_to_token_pool,
        token_output::set_output_compressed_account,
    },
    LIGHT_CPI_SIGNER,
};

pub fn process_mint_to_compressed(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    sol_log_compute_units();

    // Parse instruction data using zero-copy
    let (parsed_instruction_data, _) =
        MintToCompressedInstructionData::zero_copy_at(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;

    sol_log_compute_units();
    let with_sol_pool = parsed_instruction_data.lamports.is_some();
    msg!(" with sol pool: {}", with_sol_pool);
    let is_decompressed = parsed_instruction_data
        .compressed_mint_inputs
        .mint
        .is_decompressed();
    msg!("is_decompressed: {}", is_decompressed);
    let write_to_cpi_context = parsed_instruction_data
        .cpi_context
        .as_ref()
        .map(|x| x.first_set_context || x.set_context)
        .unwrap_or_default();
    msg!("write_to_cpi_context: {}", write_to_cpi_context);
    // Validate and parse accounts
    let validated_accounts = MintToCompressedAccounts::validate_and_parse(
        accounts,
        with_sol_pool,
        is_decompressed,
        parsed_instruction_data.cpi_context.is_some(),
        write_to_cpi_context,
    )?;
    let (config, mut cpi_bytes) = get_zero_copy_configs(&parsed_instruction_data)?;

    sol_log_compute_units();
    let (mut cpi_instruction_struct, _) =
        InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
            .map_err(ProgramError::from)?;

    cpi_instruction_struct.initialize(
        LIGHT_CPI_SIGNER.bump,
        &LIGHT_CPI_SIGNER.program_id.into(),
        parsed_instruction_data.proof,
        parsed_instruction_data.cpi_context,
    )?;

    if let Some(lamports) = parsed_instruction_data.lamports {
        cpi_instruction_struct.compress_or_decompress_lamports =
            U64::from(parsed_instruction_data.recipients.len() as u64) * *lamports;
        cpi_instruction_struct.is_compress = 1;
    }

    let mut context = TokenContext::new();
    let mint_pda = parsed_instruction_data.compressed_mint_inputs.mint.spl_mint;

    let hashed_mint_authority = context.get_or_hash_pubkey(validated_accounts.authority.key());

    {
        // Process input compressed mint account
        create_input_compressed_mint_account(
            &mut cpi_instruction_struct.input_compressed_accounts[0],
            &mut context,
            &parsed_instruction_data.compressed_mint_inputs,
            &hashed_mint_authority,
            PackedMerkleContext {
                merkle_tree_pubkey_index: 0,
                queue_pubkey_index: 1,
                leaf_index: parsed_instruction_data
                    .compressed_mint_inputs
                    .leaf_index
                    .into(),
                prove_by_index: parsed_instruction_data
                    .compressed_mint_inputs
                    .prove_by_index(),
            },
        )?;

        let mint_inputs = &parsed_instruction_data.compressed_mint_inputs.mint;
        let decimals = mint_inputs.decimals;
        let freeze_authority = mint_inputs
            .freeze_authority
            .as_ref()
            .map(|freeze_authority| (**freeze_authority));

        // Process extensions from input mint
        let (has_extensions, extensions_config, _) =
            crate::extensions::process_extensions_config(mint_inputs.extensions.as_ref())?;
        // TODO: get from get_zero_copy_configs
        let mint_config = CompressedMintConfig {
            mint_authority: (true, ()),
            freeze_authority: (mint_inputs.freeze_authority.is_some(), ()),
            extensions: (has_extensions, extensions_config),
        };
        let sum_amounts: U64 = parsed_instruction_data
            .recipients
            .iter()
            .map(|x| u64::from(x.amount))
            .sum::<u64>()
            .into();
        let supply = mint_inputs.supply + sum_amounts;

        // Compressed mint account is the last output
        create_output_compressed_mint_account(
            &mut cpi_instruction_struct.output_compressed_accounts
                [parsed_instruction_data.recipients.len()],
            mint_pda,
            decimals,
            freeze_authority,
            Some(Pubkey::from(*validated_accounts.authority.key())),
            supply,
            mint_config,
            parsed_instruction_data.compressed_mint_inputs.address,
            2,
            parsed_instruction_data.compressed_mint_inputs.mint.version,
            parsed_instruction_data
                .compressed_mint_inputs
                .mint
                .is_decompressed(),
            mint_inputs.extensions.as_deref(),
            &mut context,
        )?;
    }

    if let Some(system_accounts) = validated_accounts.executing.as_ref() {
        // If mint is decompressed, mint tokens to the token pool to maintain SPL mint supply consistency
        if is_decompressed {
            let sum_amounts: u64 = parsed_instruction_data
                .recipients
                .iter()
                .map(|x| u64::from(x.amount))
                .sum();

            let mint_account = system_accounts
                .mint
                .ok_or(ProgramError::InvalidAccountData)?;
            let token_pool_account = system_accounts
                .token_pool_pda
                .ok_or(ProgramError::InvalidAccountData)?;
            let token_program = system_accounts
                .token_program
                .ok_or(ProgramError::InvalidAccountData)?;

            mint_to_token_pool(
                mint_account,
                token_pool_account,
                token_program,
                validated_accounts.cpi_authority()?,
                sum_amounts,
            )?;
        }
    }
    msg!("cpi_instruction_struct {:?}", cpi_instruction_struct);
    // Create output token accounts
    create_output_compressed_token_accounts(
        parsed_instruction_data,
        cpi_instruction_struct,
        &mut context,
        mint_pda,
    )?;

    if let Some(system_accounts) = validated_accounts.executing {
        // Extract tree accounts for the generalized CPI call
        let tree_accounts = [
            system_accounts.tree_accounts.in_merkle_tree.key(),
            system_accounts.tree_accounts.in_output_queue.key(),
            system_accounts.tree_accounts.out_output_queue.key(),
            system_accounts.tokens_out_queue.key(),
        ];
        let start_index = if is_decompressed { 5 } else { 2 };
        msg!("start_index: {}", start_index);
        msg!(
            " system_accounts.system.sol_pool_pda.is_some(): {}",
            system_accounts.system.sol_pool_pda.is_some()
        );
        msg!(
            "accounts {:?}",
            &accounts
                .iter()
                .map(|x| solana_pubkey::Pubkey::new_from_array(*x.key()))
                .collect::<Vec<_>>()
        );
        execute_cpi_invoke(
            &accounts[start_index..], // Skip first 5 non-CPI accounts (authority, mint, token_pool_pda, token_program, light_system_program)
            cpi_bytes,
            tree_accounts.as_slice(),
            system_accounts.system.sol_pool_pda.is_some(),
            None,
            None,  // no cpi_context_account for mint_to_compressed
            false, // write to cpi context account
        )?;
    } else if let Some(system_accounts) = validated_accounts.write_to_cpi_context_system.as_ref() {
        if with_sol_pool {
            unimplemented!("")
        }
        if is_decompressed {
            unimplemented!("")
        }
        // Execute CPI call to light-system-program
        execute_cpi_invoke(
            &accounts[3..6],
            cpi_bytes,
            &[],
            false,
            None,
            Some(*system_accounts.cpi_context.key()),
            true, // write to cpi context account
        )?;
    } else {
        unreachable!()
    }
    Ok(())
}


fn get_zero_copy_configs(parsed_instruction_data: &light_ctoken_types::instructions::mint_to_compressed::ZMintToCompressedInstructionData<'_>) -> Result<(light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnlyConfig, Vec<u8>), ProgramError>{
    // Build configuration for CPI instruction data using the generalized function
    let compressed_mint_with_freeze_authority = parsed_instruction_data
        .compressed_mint_inputs
        .mint
        .freeze_authority
        .is_some();

    // Process extensions to get the proper config for CPI bytes allocation
    // The mint contains ZExtensionInstructionData, so we can use process_extensions_config directly
    let (_, extensions_config, _) = crate::extensions::process_extensions_config(
        parsed_instruction_data
            .compressed_mint_inputs
            .mint
            .extensions
            .as_ref(),
    )?;

    let mut config_input = CpiConfigInput::mint_to_compressed(
        parsed_instruction_data.recipients.len(),
        parsed_instruction_data.proof.is_some(),
        compressed_mint_with_freeze_authority,
    );
    // Override the empty extensions_config with the actual one
    config_input.extensions_config = extensions_config;

    let config = cpi_bytes_config(config_input);
    let cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);

    Ok((config, cpi_bytes))
}

fn create_output_compressed_token_accounts(
    parsed_instruction_data: light_ctoken_types::instructions::mint_to_compressed::ZMintToCompressedInstructionData<'_>,
    mut cpi_instruction_struct: light_compressed_account::instruction_data::with_readonly::ZInstructionDataInvokeCpiWithReadOnlyMut<'_>,
    context: &mut TokenContext,
    mint: Pubkey,
) -> Result<(), ProgramError> {
    let hashed_mint = context.get_or_hash_mint(&mint.to_bytes())?;

    let lamports = parsed_instruction_data
        .lamports
        .map(|lamports| u64::from(*lamports));
    for (recipient, output_account) in parsed_instruction_data
        .recipients
        .iter()
        .zip(cpi_instruction_struct.output_compressed_accounts.iter_mut())
    {
        let output_delegate = None;
        set_output_compressed_account::<false>(
            output_account,
            context,
            recipient.recipient,
            output_delegate,
            recipient.amount,
            lamports,
            mint,
            &hashed_mint,
            2,
            parsed_instruction_data.token_account_version,
        )?;
    }
    Ok(())
}

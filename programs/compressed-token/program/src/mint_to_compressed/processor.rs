use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::{
    hash_to_bn254_field_size_be,
    instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly, Pubkey,
};
use light_zero_copy::{borsh::Deserialize, ZeroCopyNew};
use pinocchio::account_info::AccountInfo;
use spl_token::solana_program::log::sol_log_compute_units;
use zerocopy::little_endian::U64;

use crate::{
    mint::{
        input::create_input_compressed_mint_account, output::create_output_compressed_mint_account,
        state::CompressedMint,
    },
    mint_to_compressed::{
        accounts::MintToCompressedAccounts, instructions::MintToCompressedInstructionData,
    },
    shared::{
        context::TokenContext,
        cpi::execute_cpi_invoke,
        cpi_bytes_size::{
            allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
        },
        outputs::create_output_compressed_account,
    },
    LIGHT_CPI_SIGNER,
};

pub fn process_mint_to_compressed(
    program_id: pinocchio::pubkey::Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    sol_log_compute_units();

    // Parse instruction data using zero-copy
    let (parsed_instruction_data, _) =
        MintToCompressedInstructionData::zero_copy_at(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;

    sol_log_compute_units();

    // Validate and parse accounts
    let validated_accounts = MintToCompressedAccounts::validate_and_parse(
        accounts,
        parsed_instruction_data.lamports.is_some(),
        parsed_instruction_data
            .compressed_mint_inputs
            .compressed_mint_input
            .is_decompressed(),
    )?;
    // Build configuration for CPI instruction data using the generalized function
    let compressed_mint_with_freeze_authority = parsed_instruction_data
        .compressed_mint_inputs
        .compressed_mint_input
        .freeze_authority_is_set
        != 0;

    let config_input = CpiConfigInput::mint_to_compressed(
        parsed_instruction_data.recipients.len(),
        parsed_instruction_data.proof.is_some(),
        compressed_mint_with_freeze_authority,
    );

    let config = cpi_bytes_config(config_input);
    let mut cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);

    sol_log_compute_units();
    let (mut cpi_instruction_struct, _) =
        InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
            .map_err(ProgramError::from)?;
    cpi_instruction_struct.bump = LIGHT_CPI_SIGNER.bump;
    cpi_instruction_struct.invoking_program_id = LIGHT_CPI_SIGNER.program_id.into();
    if let Some(lamports) = parsed_instruction_data.lamports {
        cpi_instruction_struct.compress_or_decompress_lamports =
            U64::from(parsed_instruction_data.recipients.len() as u64) * *lamports;
        cpi_instruction_struct.is_compress = 1;
    }

    let mut context = TokenContext::new();
    let mint = parsed_instruction_data
        .compressed_mint_inputs
        .compressed_mint_input
        .spl_mint;

    let hashed_mint = hash_to_bn254_field_size_be(mint.as_ref());
    let hashed_mint_authority = context.get_or_hash_pubkey(validated_accounts.authority.key());

    {
        // Process input compressed mint account
        create_input_compressed_mint_account(
            &mut cpi_instruction_struct.input_compressed_accounts[0],
            &mut context,
            &parsed_instruction_data.compressed_mint_inputs,
            &hashed_mint_authority,
        )?;
        let mint_inputs = &parsed_instruction_data
            .compressed_mint_inputs
            .compressed_mint_input;
        let mint_pda = mint_inputs.spl_mint;
        let decimals = mint_inputs.decimals;
        // TODO: make option in ix data.
        let freeze_authority = if mint_inputs.freeze_authority_is_set() {
            Some(mint_inputs.freeze_authority)
        } else {
            None
        };
        use crate::mint::state::CompressedMintConfig;
        
        // Process extensions from input mint
        let (has_extensions, extensions_config, _) = 
            crate::extensions::process_extensions_config(
                mint_inputs.extensions.as_ref()
            );
        
        let mint_config = CompressedMintConfig {
            mint_authority: (true, ()),
            freeze_authority: (mint_inputs.freeze_authority_is_set(), ()),
            extensions: (has_extensions, extensions_config),
        };
        let compressed_account_address = *parsed_instruction_data.compressed_mint_inputs.address;
        let sum_amounts: U64 = parsed_instruction_data
            .recipients
            .iter()
            .map(|x| u64::from(x.amount))
            .sum::<u64>()
            .into();
        let supply = mint_inputs.supply + sum_amounts;

        // Extensions are already in zero-copy format, so we can pass them directly
        let z_extensions = mint_inputs.extensions.as_deref();

        // Compressed mint account is the last output
        create_output_compressed_mint_account(
            &mut cpi_instruction_struct.output_compressed_accounts
                [parsed_instruction_data.recipients.len()],
            mint_pda,
            decimals,
            freeze_authority,
            Some(Pubkey::from(*validated_accounts.authority.key())),
            supply,
            &program_id.into(),
            mint_config,
            compressed_account_address,
            2,
            parsed_instruction_data.compressed_mint_inputs.compressed_mint_input.version,
            z_extensions,
        )?;
    }

    let is_decompressed = parsed_instruction_data
        .compressed_mint_inputs
        .compressed_mint_input
        .is_decompressed();
    // Create output token accounts
    create_output_compressed_token_accounts(
        parsed_instruction_data,
        cpi_instruction_struct,
        &mut context,
        mint,
        hashed_mint,
    )?;

    // Extract tree accounts for the generalized CPI call
    let tree_accounts = [
        validated_accounts.mint_in_merkle_tree.key(),
        validated_accounts.mint_in_queue.key(),
        validated_accounts.mint_out_queue.key(),
        validated_accounts.tokens_out_queue.key(),
    ];
    let start_index = if is_decompressed { 5 } else { 2 };

    execute_cpi_invoke(
        &accounts[start_index..], // Skip first 5 non-CPI accounts (authority, mint, token_pool_pda, token_program, light_system_program)
        cpi_bytes,
        tree_accounts.as_slice(),
        validated_accounts.sol_pool_pda.is_some(),
        None, // no cpi_context_account for mint_to_compressed
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
            2,
        )?;
    }
    Ok(())
}

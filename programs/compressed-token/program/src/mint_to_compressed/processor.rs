use anchor_lang::solana_program::{account_info::AccountInfo, program_error::ProgramError};
use light_compressed_account::{
    hash_to_bn254_field_size_be,
    instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly, Pubkey,
};
use light_zero_copy::{borsh::Deserialize, ZeroCopyNew};
use spl_token::solana_program::log::sol_log_compute_units;

use crate::{
    mint::{
        input::create_input_compressed_mint_account, output::create_output_compressed_mint_account,
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
    let (mut cpi_instruction_struct, _) =
        InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
            .map_err(ProgramError::from)?;
    let mut context = TokenContext::new();
    let mint = parsed_instruction_data
        .compressed_mint_inputs
        .compressed_mint_input
        .spl_mint;

    let hashed_mint = hash_to_bn254_field_size_be(mint.as_ref());

    {
        // Process input compressed mint account
        create_input_compressed_mint_account(
            &mut cpi_instruction_struct.input_compressed_accounts[0],
            &mut context,
            &parsed_instruction_data.compressed_mint_inputs,
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
        let mint_config = CompressedMintConfig {
            mint_authority: (true, ()),
            freeze_authority: (mint_inputs.freeze_authority_is_set(), ()),
        };
        let compressed_account_address = *parsed_instruction_data.compressed_mint_inputs.address;

        // Compressed mint account is the last output
        create_output_compressed_mint_account(
            &mut cpi_instruction_struct.output_compressed_accounts
                [parsed_instruction_data.recipients.len()],
            mint_pda,
            decimals,
            freeze_authority,
            Some((*validated_accounts.authority.key).into()),
            &program_id,
            mint_config,
            compressed_account_address,
            parsed_instruction_data
                .compressed_mint_inputs
                .output_merkle_tree_index,
        )?;
    }
    // Create output token accounts
    create_output_compressed_token_accounts(
        parsed_instruction_data,
        cpi_instruction_struct,
        &mut context,
        mint,
        hashed_mint,
    )?;
    // Extract tree accounts for the generalized CPI call
    let tree_accounts = [*validated_accounts.merkle_tree.key];
    
    execute_cpi_invoke(
        accounts,
        cpi_bytes,
        &tree_accounts,
        validated_accounts.sol_pool_pda.map(|acc| *acc.key),
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
            0,
        )?;
    }
    Ok(())
}


use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly;
use light_ctoken_types::{
    context::TokenContext,
    instructions::update_compressed_mint::{
        CompressedMintAuthorityType, UpdateCompressedMintInstructionDataV2,
        ZUpdateCompressedMintInstructionDataV2,
    },
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
    shared::{
        cpi::execute_cpi_invoke,
        cpi_bytes_size::{
            allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
        },
    },
    update_mint::accounts::UpdateCompressedMintAccounts,
    LIGHT_CPI_SIGNER,
};

/// Note, even once a cmint is decompressed we only update the compressed mint because we ultimately use the compressed mint's authority.
pub fn process_update_compressed_mint(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    sol_log_compute_units();

    // Parse instruction data using zero-copy
    let (parsed_instruction_data, _) =
        UpdateCompressedMintInstructionDataV2::zero_copy_at(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;

    // Parse and validate authority type
    let authority_type =
        CompressedMintAuthorityType::try_from(parsed_instruction_data.authority_type)?;

    sol_log_compute_units();

    let write_to_cpi_context = parsed_instruction_data
        .cpi_context
        .as_ref()
        .map(|x| x.first_set_context() || x.set_context())
        .unwrap_or_default();

    // Validate and parse accounts
    let validated_accounts = UpdateCompressedMintAccounts::validate_and_parse(
        accounts,
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
        parsed_instruction_data.compressed_mint_inputs.proof,
        &parsed_instruction_data.cpi_context,
    )?;

    let mut context = TokenContext::new();
    let mint_pda = parsed_instruction_data.compressed_mint_inputs.mint.spl_mint;
    let mint_data = &parsed_instruction_data.compressed_mint_inputs.mint;

    // The authority validation happens when creating the input compressed account
    // The signer must be the current authority that can perform this operation
    let hashed_mint_authority = context.get_or_hash_pubkey(validated_accounts.authority.key());

    {
        let merkle_tree_pubkey_index =
            if let Some(cpi_context) = parsed_instruction_data.cpi_context.as_ref() {
                cpi_context.in_tree_index
            } else {
                0
            };
        let queue_pubkey_index =
            if let Some(cpi_context) = parsed_instruction_data.cpi_context.as_ref() {
                cpi_context.in_queue_index
            } else {
                1
            };

        // Process input compressed mint account
        create_input_compressed_mint_account(
            &mut cpi_instruction_struct.input_compressed_accounts[0],
            &mut context,
            &parsed_instruction_data.compressed_mint_inputs,
            &hashed_mint_authority,
            PackedMerkleContext {
                merkle_tree_pubkey_index,
                queue_pubkey_index,
                leaf_index: parsed_instruction_data
                    .compressed_mint_inputs
                    .leaf_index
                    .into(),
                prove_by_index: parsed_instruction_data
                    .compressed_mint_inputs
                    .prove_by_index
                    != 0,
            },
        )?;

        // Apply authority update based on authority type and new_authority field
        let (mint_authority, freeze_authority) = match authority_type {
            CompressedMintAuthorityType::MintTokens => {
                let new_mint_authority = parsed_instruction_data
                    .new_authority
                    .as_ref()
                    .map(|auth| **auth); // None = revoke, Some(key) = set new authority

                (
                    new_mint_authority,
                    mint_data.freeze_authority.as_ref().map(|fa| **fa),
                )
            }
            CompressedMintAuthorityType::FreezeAccount => {
                let new_freeze_authority = parsed_instruction_data
                    .new_authority
                    .as_ref()
                    .map(|auth| **auth); // None = revoke, Some(key) = set new authority

                // Use the mint authority from instruction data to preserve it
                let current_mint_authority = parsed_instruction_data
                    .mint_authority
                    .as_ref()
                    .map(|auth| **auth);
                (current_mint_authority, new_freeze_authority)
            }
        };

        let decimals = mint_data.decimals;
        let supply = U64::from(mint_data.supply);

        // Process extensions from input mint
        let (has_extensions, extensions_config, _) =
            crate::extensions::process_extensions_config(mint_data.extensions.as_ref())?;

        let mint_config = CompressedMintConfig {
            mint_authority: (mint_authority.is_some(), ()),
            freeze_authority: (freeze_authority.is_some(), ()),
            extensions: (has_extensions, extensions_config),
        };

        let queue_pubkey_index =
            if let Some(cpi_context) = parsed_instruction_data.cpi_context.as_ref() {
                cpi_context.out_queue_index
            } else {
                2
            };

        // Create output compressed mint account with updated authorities
        create_output_compressed_mint_account(
            &mut cpi_instruction_struct.output_compressed_accounts[0],
            mint_pda,
            decimals,
            freeze_authority,
            mint_authority,
            supply,
            mint_config,
            parsed_instruction_data.compressed_mint_inputs.address,
            queue_pubkey_index,
            mint_data.version,
            mint_data.is_decompressed(),
            mint_data.extensions.as_deref(),
            &mut context,
        )?;
    }

    if let Some(system_accounts) = validated_accounts.executing {
        // Extract tree accounts for the generalized CPI call
        let tree_accounts = [
            system_accounts.tree_accounts.in_merkle_tree.key(),
            system_accounts.tree_accounts.in_output_queue.key(),
            system_accounts.tree_accounts.out_output_queue.key(),
        ];

        execute_cpi_invoke(
            &accounts[2..], // Skip first 2 non-CPI accounts (light_system_program, authority)
            cpi_bytes,
            tree_accounts.as_slice(),
            false, // no sol pool for mint updates
            None,
            None,  // no cpi_context_account for update_mint
            false, // write to cpi context account
        )?;
    } else if let Some(system_accounts) = validated_accounts.write_to_cpi_context_system.as_ref() {
        // Execute CPI call to light-system-program
        execute_cpi_invoke(
            &accounts[2..],
            cpi_bytes,
            &[],
            false,
            None,
            Some(*system_accounts.cpi_context.key()),
            true, // write to cpi context account
        )?;
    } else {
        msg!("no system accounts");
        unreachable!()
    }
    Ok(())
}

fn get_zero_copy_configs(
    parsed_instruction_data: &ZUpdateCompressedMintInstructionDataV2,
) -> Result<(
    light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnlyConfig,
    Vec<u8>,
), ProgramError>{
    // Parse authority type to determine which authority is being updated
    let authority_type =
        CompressedMintAuthorityType::try_from(parsed_instruction_data.authority_type)?;

    // Calculate updated authorities for consistent config
    let (updated_mint_authority, updated_freeze_authority) = match authority_type {
        CompressedMintAuthorityType::MintTokens => {
            let new_mint_authority = parsed_instruction_data.new_authority.is_some();
            let current_freeze_authority = parsed_instruction_data
                .compressed_mint_inputs
                .mint
                .freeze_authority
                .is_some();
            (new_mint_authority, current_freeze_authority)
        }
        CompressedMintAuthorityType::FreezeAccount => {
            let new_freeze_authority = parsed_instruction_data.new_authority.is_some();
            let current_mint_authority = parsed_instruction_data.mint_authority.is_some();
            (current_mint_authority, new_freeze_authority)
        }
    };

    // Process extensions to get the proper config for CPI bytes allocation
    let (_, extensions_config, _) = crate::extensions::process_extensions_config(
        parsed_instruction_data
            .compressed_mint_inputs
            .mint
            .extensions
            .as_ref(),
    )?;

    let mut config_input = CpiConfigInput::update_mint(
        parsed_instruction_data
            .compressed_mint_inputs
            .proof
            .is_some(),
        updated_freeze_authority,
        updated_mint_authority,
    );
    // Override the empty extensions_config with the actual one
    config_input.extensions_config = extensions_config;

    let config = cpi_bytes_config(config_input);
    let cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);

    Ok((config, cpi_bytes))
}

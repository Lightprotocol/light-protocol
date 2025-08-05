use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::program_error::ProgramError;
use arrayvec::ArrayVec;
use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnlyConfig;
use light_ctoken_types::{
    instructions::{
        extensions::ZExtensionInstructionData,
        mint_actions::{ZAction, ZMintActionCompressedInstructionData},
    },
    state::{CompressedMintConfig, ExtensionStructConfig},
};

use spl_pod::solana_msg::msg;

use crate::shared::cpi_bytes_size::{
    allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
};

pub fn get_zero_copy_configs(
    parsed_instruction_data: &mut ZMintActionCompressedInstructionData<'_>,
) -> Result<
    (
        InstructionDataInvokeCpiWithReadOnlyConfig,
        Vec<u8>,
        CompressedMintConfig,
        bool,
    ),
    ProgramError,
> {
    let mut idempotent = false;
    use light_ctoken_types::state::CompressedMintConfig;
    msg!("get_zero_copy_configs");
    // Process extensions to get the proper config for CPI bytes allocation
    let (_, mut extensions_config, _) = crate::extensions::process_extensions_config(
        parsed_instruction_data.mint.extensions.as_ref(),
    )?;
    msg!("get_zero_copy_configs1");

    // Calculate final authority states after processing all actions
    let mut final_mint_authority = parsed_instruction_data.mint.mint_authority.is_some();
    let mut final_freeze_authority = parsed_instruction_data.mint.freeze_authority.is_some();

    // Process actions in order to determine final authority states
    for action in parsed_instruction_data.actions.iter() {
        match action {
            ZAction::UpdateMintAuthority(update_action) => {
                // None = revoke authority, Some(key) = set new authority
                final_mint_authority = update_action.new_authority.is_some();
            }
            ZAction::UpdateFreezeAuthority(update_action) => {
                // None = revoke authority, Some(key) = set new authority
                final_freeze_authority = update_action.new_authority.is_some();
            }
            ZAction::RemoveMetadataKey(action) => {
                let extension_index = action.extension_index as usize;
                if extension_index >= extensions_config.len() {
                    msg!("Extension index {} out of bounds", extension_index);
                    return Err(
                        anchor_compressed_token::ErrorCode::MintActionInvalidExtensionIndex.into(),
                    );
                }
                if let ExtensionStructConfig::TokenMetadata(ref mut metadata_config) =
                    &mut extensions_config[extension_index]
                {
                    let mut found = None;
                    idempotent = action.idempotent != 0;
                    let additional_metadata =
                        if let ZExtensionInstructionData::TokenMetadata(metadata_pair) =
                            &mut parsed_instruction_data.mint.extensions.as_mut().unwrap()
                                [extension_index]
                        {
                            metadata_pair.additional_metadata.as_mut().unwrap()
                        } else {
                            &mut vec![]
                        };
                    for (index, metadata_pair) in additional_metadata.iter().enumerate() {
                        if metadata_pair.key == action.key {
                            found = Some(index);
                            break;
                        }
                    }
                    if let Some(index) = found {
                        // remove it from ix data
                        additional_metadata.remove(index);
                        // remove it from config
                        metadata_config.additional_metadata.remove(index);
                    } else if !idempotent {
                        msg!("Adding new custom key-value pair not supported in zero-copy mode");
                        return Err(ErrorCode::MintActionUnsupportedOperation.into());
                    }
                }
            }
            ZAction::UpdateMetadataAuthority(auth_action) => {
                // Check if authority is being revoked (set to zero pubkey)
                if auth_action.new_authority.to_bytes() == [0u8; 32] {
                    // Update specific extension config to allocate with None authority
                    let extension_index = auth_action.extension_index as usize;
                    if extension_index >= extensions_config.len() {
                        msg!("Extension index {} out of bounds", extension_index);
                        return Err(
                            anchor_compressed_token::ErrorCode::MintActionInvalidExtensionIndex
                                .into(),
                        );
                    }
                    if let ExtensionStructConfig::TokenMetadata(ref mut metadata_config) =
                        &mut extensions_config[extension_index]
                    {
                        metadata_config.update_authority = (false, ());
                    }
                }
            }
            _ => {} // Other actions don't affect authority or extension states
        }
    }
    msg!("get_zero_copy_configs2");

    // Output mint config (always present) with final authority states
    let output_mint_config = CompressedMintConfig {
        mint_authority: (final_mint_authority, ()),
        freeze_authority: (final_freeze_authority, ()),
        extensions: (!extensions_config.is_empty(), extensions_config),
    };

    // Count recipients from MintTo actions
    let num_recipients = parsed_instruction_data
        .actions
        .iter()
        .map(|action| match action {
            ZAction::MintTo(mint_to_action) => mint_to_action.recipients.len(),
            _ => 0,
        })
        .sum();
    msg!("get_zero_copy_configs2");

    let input = CpiConfigInput {
        input_accounts: {
            let mut inputs = ArrayVec::new();
            // Add input mint if not creating mint
            if !parsed_instruction_data.create_mint() {
                inputs.push(true); // Input mint has address
            }
            inputs
        },
        output_accounts: {
            let mut outputs = ArrayVec::new();
            // First output is always the mint account
            outputs.push((
                true,
                crate::shared::cpi_bytes_size::mint_data_len(&output_mint_config),
            ));

            // Add token accounts for recipients
            for _ in 0..num_recipients {
                outputs.push((false, crate::shared::cpi_bytes_size::token_data_len(false)));
                // No delegates for simple mint
            }
            outputs
        },
        has_proof: parsed_instruction_data.proof.is_some(),
        // Add new address params if creating a mint
        new_address_params: if parsed_instruction_data.create_mint() {
            1
        } else {
            0
        },
    };
    msg!("get_zero_copy_configs5");

    let config = cpi_bytes_config(input);
    msg!("get_zero_copy_configs6");
    let cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);
    msg!("get_zero_copy_configs7");

    Ok((config, cpi_bytes, output_mint_config, idempotent))
}

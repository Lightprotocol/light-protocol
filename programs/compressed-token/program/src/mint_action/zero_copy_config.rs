use anchor_lang::solana_program::program_error::ProgramError;
use arrayvec::ArrayVec;
use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnlyConfig;
use light_ctoken_types::{
    instructions::{
        extensions::ZExtensionInstructionData,
        mint_action::{ZAction, ZMintActionCompressedInstructionData},
    },
    state::{BaseCompressedMintConfig, CompressedMintConfig},
};
use light_profiler::profile;
use spl_pod::solana_msg::msg;

use crate::shared::cpi_bytes_size::{
    allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
};

#[profile]
pub fn get_zero_copy_configs(
    parsed_instruction_data: &mut ZMintActionCompressedInstructionData<'_>,
) -> Result<
    (
        InstructionDataInvokeCpiWithReadOnlyConfig,
        Vec<u8>,
        CompressedMintConfig,
    ),
    ProgramError,
> {
    // Generate output config based on final state after all actions (without modifying instruction data)
    let (_, output_extensions_config, _) =
        crate::extensions::process_extensions_config_with_actions(
            parsed_instruction_data.mint.extensions.as_ref(),
            &parsed_instruction_data.actions,
        )?;

    // Calculate final authority states and modify output config without touching instruction data
    let mut final_mint_authority = parsed_instruction_data.mint.base.mint_authority.is_some();
    let mut final_freeze_authority = parsed_instruction_data.mint.base.freeze_authority.is_some();

    // Process actions to determine final output state (no instruction data modification)
    for action in parsed_instruction_data.actions.iter() {
        match action {
            ZAction::UpdateMintAuthority(update_action) => {
                final_mint_authority = update_action.new_authority.is_some();
            }
            ZAction::UpdateFreezeAuthority(update_action) => {
                final_freeze_authority = update_action.new_authority.is_some();
            }
            ZAction::RemoveMetadataKey(_) => {}
            ZAction::UpdateMetadataAuthority(auth_action) => {
                // Update output config for authority revocation
                if auth_action.new_authority.to_bytes() == [0u8; 32] {
                    let extension_index = auth_action.extension_index as usize;
                    if extension_index >= output_extensions_config.len() {
                        msg!("Extension index {} out of bounds", extension_index);
                        return Err(
                            anchor_compressed_token::ErrorCode::MintActionInvalidExtensionIndex
                                .into(),
                        );
                    }
                }
            }
            _ => {}
        }
    }

    // Output mint config (always present) with final authority states
    let output_mint_config = CompressedMintConfig {
        base: BaseCompressedMintConfig {
            mint_authority: (final_mint_authority, ()),
            freeze_authority: (final_freeze_authority, ()),
        },
        extensions: (
            !output_extensions_config.is_empty(),
            output_extensions_config,
        ),
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

    let config = cpi_bytes_config(input);
    let cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);

    Ok((config, cpi_bytes, output_mint_config))
}

/// Removes metadata keys from instruction data that were marked for removal
/// This should be called AFTER input data hash calculation to avoid hash mismatch
/// Returns an error if non-idempotent key removal fails (key not found)
#[profile]
pub fn cleanup_removed_metadata_keys(
    parsed_instruction_data: &mut ZMintActionCompressedInstructionData<'_>,
) -> Result<(), ProgramError> {
    for action in parsed_instruction_data.actions.iter() {
        if let ZAction::RemoveMetadataKey(action) = action {
            let extension_index = action.extension_index as usize;
            let mut key_found = false;

            if let Some(extensions) = parsed_instruction_data.mint.extensions.as_mut() {
                if extension_index >= extensions.len() {
                    continue; // Skip invalid indices
                }
                if let ZExtensionInstructionData::TokenMetadata(ref mut metadata_pair) =
                    &mut extensions[extension_index]
                {
                    if let Some(ref mut additional_metadata) = metadata_pair.additional_metadata {
                        // Find and remove the key
                        if let Some(index) = additional_metadata
                            .iter()
                            .position(|pair| pair.key == action.key)
                        {
                            additional_metadata.remove(index);
                            key_found = true;
                        }
                    }
                }
            }

            // Check if key was found when operation is not idempotent
            if !key_found && action.idempotent == 0 {
                let key_str = String::from_utf8_lossy(action.key);
                msg!("Key '{}' not found for non-idempotent removal", key_str);
                return Err(
                    anchor_compressed_token::ErrorCode::MintActionUnsupportedOperation.into(),
                );
            }
        }
    }
    Ok(())
}

pub mod processor;
pub mod token_metadata;

// Import from ctoken-types instead of local modules
use light_ctoken_types::{
    instructions::{extensions::ZExtensionInstructionData, mint_action::ZAction},
    state::{AdditionalMetadataConfig, ExtensionStructConfig, TokenMetadata, TokenMetadataConfig},
    CTokenError,
};
use light_profiler::profile;
use light_zero_copy::ZeroCopyNew;

/// Action-aware version that calculates maximum sizes needed for field updates
/// Returns: (has_extensions, extension_configs, additional_data_len)
#[profile]
pub fn process_extensions_config_with_actions(
    extensions: Option<&Vec<ZExtensionInstructionData>>,
    actions: &[ZAction],
) -> Result<(bool, Vec<ExtensionStructConfig>, usize), CTokenError> {
    if let Some(extensions) = extensions {
        let mut additional_mint_data_len = 0;
        let mut config_vec = Vec::new();

        for (extension_index, extension) in extensions.iter().enumerate() {
            match extension {
                ZExtensionInstructionData::TokenMetadata(token_metadata_data) => {
                    process_token_metadata_config_with_actions(
                        &mut additional_mint_data_len,
                        &mut config_vec,
                        token_metadata_data,
                        actions,
                        extension_index,
                    )?
                }
                _ => return Err(CTokenError::UnsupportedExtension),
            }
        }
        Ok((true, config_vec, additional_mint_data_len))
    } else {
        Ok((false, Vec::new(), 0))
    }
}

fn process_token_metadata_config_with_actions(
    additional_mint_data_len: &mut usize,
    config_vec: &mut Vec<ExtensionStructConfig>,
    token_metadata_data: &light_ctoken_types::instructions::extensions::ZTokenMetadataInstructionData<'_>,
    actions: &[ZAction],
    extension_index: usize,
) -> Result<(), CTokenError> {
    // Calculate final sizes by applying actions sequentially to determine the actual final state
    let mut final_name_len = token_metadata_data.name.len();
    let mut final_symbol_len = token_metadata_data.symbol.len();
    let mut final_uri_len = token_metadata_data.uri.len();
    // TODO: test
    // Apply actions sequentially to determine final field sizes (last action wins)
    for action in actions.iter() {
        if let ZAction::UpdateMetadataField(update_action) = action {
            if update_action.extension_index as usize == extension_index {
                match update_action.field_type {
                    0 => final_name_len = update_action.value.len(), // name - last update determines final size
                    1 => final_symbol_len = update_action.value.len(), // symbol - last update determines final size
                    2 => final_uri_len = update_action.value.len(), // uri - last update determines final size
                    _ => {} // custom fields handled separately
                }
            }
        }
    }

    let additional_metadata_configs =
        if let Some(ref additional_metadata) = token_metadata_data.additional_metadata {
            // Get list of keys that will be removed
            let mut keys_to_remove = Vec::new();
            for action in actions.iter() {
                if let ZAction::RemoveMetadataKey(remove_action) = action {
                    if remove_action.extension_index as usize == extension_index {
                        keys_to_remove.push(&remove_action.key);
                    }
                }
            }

            // Filter out keys that will be removed
            additional_metadata
                .iter()
                .filter(|item| {
                    // Keep the key if it's not in the removal list
                    !keys_to_remove
                        .iter()
                        .any(|remove_key| *remove_key == &item.key)
                })
                .map(|item| AdditionalMetadataConfig {
                    key: item.key.len() as u32,
                    value: item.value.len() as u32,
                })
                .collect()
        } else {
            vec![]
        };

    let config = TokenMetadataConfig {
        name: final_name_len as u32,
        symbol: final_symbol_len as u32,
        uri: final_uri_len as u32,
        additional_metadata: additional_metadata_configs,
    };
    let byte_len = TokenMetadata::byte_len(&config)?;
    *additional_mint_data_len += byte_len;
    config_vec.push(ExtensionStructConfig::TokenMetadata(config));
    Ok(())
}

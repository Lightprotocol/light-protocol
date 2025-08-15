// pub mod metadata_pointer;
pub mod processor;
pub mod token_metadata;
pub mod token_metadata_ui;

// Import from ctoken-types instead of local modules
use light_ctoken_types::{
    instructions::{extensions::ZExtensionInstructionData, mint_actions::ZAction},
    state::{
        AdditionalMetadataConfig, ExtensionStructConfig, MetadataConfig, TokenMetadata,
        TokenMetadataConfig,
    },
    CTokenError,
};
use light_zero_copy::ZeroCopyNew;

/// Processes extension instruction data and returns the configuration tuple and additional data length
/// Returns: (has_extensions, extension_configs, additional_data_len)
pub fn process_extensions_config(
    extensions: Option<&Vec<ZExtensionInstructionData>>,
) -> Result<(bool, Vec<ExtensionStructConfig>, usize), CTokenError> {
    if let Some(extensions) = extensions {
        let mut additional_mint_data_len = 0;
        let mut config_vec = Vec::new();

        for extension in extensions.iter() {
            match extension {
                ZExtensionInstructionData::TokenMetadata(token_metadata_data) => {
                    process_token_metadata_config(
                        &mut additional_mint_data_len,
                        &mut config_vec,
                        token_metadata_data,
                    )
                }
                _ => return Err(CTokenError::UnsupportedExtension),
            }
        }
        Ok((true, config_vec, additional_mint_data_len))
    } else {
        Ok((false, Vec::new(), 0))
    }
}

/// Action-aware version that calculates maximum sizes needed for field updates
/// Returns: (has_extensions, extension_configs, additional_data_len)
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
                    )
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
) {
    // Calculate maximum sizes needed by scanning current data and all planned updates
    let mut max_name_len = token_metadata_data.metadata.name.len();
    let mut max_symbol_len = token_metadata_data.metadata.symbol.len();
    let mut max_uri_len = token_metadata_data.metadata.uri.len();

    // Scan actions for field updates that affect this extension
    for action in actions.iter() {
        if let ZAction::UpdateMetadataField(update_action) = action {
            if update_action.extension_index as usize == extension_index {
                match update_action.field_type {
                    0 => max_name_len = max_name_len.max(update_action.value.len()), // name
                    1 => max_symbol_len = max_symbol_len.max(update_action.value.len()), // symbol
                    2 => max_uri_len = max_uri_len.max(update_action.value.len()),   // uri
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
        update_authority: (token_metadata_data.update_authority.is_some(), ()),
        metadata: MetadataConfig {
            name: max_name_len as u32,
            symbol: max_symbol_len as u32,
            uri: max_uri_len as u32,
        },
        additional_metadata: additional_metadata_configs,
    };
    let byte_len = TokenMetadata::byte_len(&config).unwrap();
    *additional_mint_data_len += byte_len;
    config_vec.push(ExtensionStructConfig::TokenMetadata(config));
}

fn process_token_metadata_config(
    additional_mint_data_len: &mut usize,
    config_vec: &mut Vec<ExtensionStructConfig>,
    token_metadata_data: &light_ctoken_types::instructions::extensions::ZTokenMetadataInstructionData<'_>,
) {
    // Delegate to action-aware version with no actions
    process_token_metadata_config_with_actions(
        additional_mint_data_len,
        config_vec,
        token_metadata_data,
        &[],
        0, // extension_index not used when no actions
    )
}

pub mod processor;
pub mod token_metadata;

// Import from ctoken-types instead of local modules
use light_ctoken_types::{
    instructions::{
        extensions::{ZExtensionInstructionData, ZTokenMetadataInstructionData},
        mint_action::ZAction,
    },
    state::{
        AdditionalMetadataConfig, ExtensionStructConfig, TokenMetadata, TokenMetadataConfig,
        ZAdditionalMetadata,
    },
    CTokenError,
};
use light_program_profiler::profile;
use light_zero_copy::ZeroCopyNew;
use spl_pod::solana_msg::msg;

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
    token_metadata_data: &ZTokenMetadataInstructionData<'_>,
    actions: &[ZAction],
    extension_index: usize,
) -> Result<(), CTokenError> {
    // Early validation - no allocations needed
    if let Some(ref additional_metadata) = token_metadata_data.additional_metadata {
        if additional_metadata.len() > 20 {
            msg!(
                "Too many additional metadata elements: {} (max 20)",
                additional_metadata.len()
            );
            return Err(CTokenError::TooManyAdditionalMetadata);
        }

        // Check for duplicate keys (O(n²) but acceptable for max 20 items)
        for i in 0..additional_metadata.len() {
            for j in (i + 1)..additional_metadata.len() {
                if additional_metadata[i].key == additional_metadata[j].key {
                    msg!("Duplicate metadata key found at positions {} and {}", i, j);
                    return Err(CTokenError::DuplicateMetadataKey);
                }
            }
        }
    }

    // Single-pass state accumulator - track final sizes directly
    let mut final_name_len = token_metadata_data.name.len();
    let mut final_symbol_len = token_metadata_data.symbol.len();
    let mut final_uri_len = token_metadata_data.uri.len();

    // Apply actions sequentially to determine final field sizes (last action wins)
    for action in actions.iter() {
        if let ZAction::UpdateMetadataField(update_action) = action {
            if update_action.extension_index as usize == extension_index {
                match update_action.field_type {
                    0 => final_name_len = update_action.value.len(), // name
                    1 => final_symbol_len = update_action.value.len(), // symbol
                    2 => final_uri_len = update_action.value.len(),  // uri
                    _ => {}                                          // custom fields handled below
                }
            }
        }
    }

    // Build metadata config directly without intermediate collections
    let additional_metadata_configs = build_metadata_config(
        token_metadata_data.additional_metadata.as_ref(),
        actions,
        extension_index,
    );

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

/// Build metadata config directly without heap allocations using ArrayVec
/// Processes all possible keys and determines final state (SPL Token-2022 compatible)
#[inline(always)]
fn build_metadata_config(
    metadata: Option<&Vec<ZAdditionalMetadata<'_>>>,
    actions: &[ZAction],
    extension_index: usize,
) -> Vec<AdditionalMetadataConfig> {
    let mut configs = arrayvec::ArrayVec::<AdditionalMetadataConfig, 20>::new();
    let mut processed_keys = tinyvec::ArrayVec::<[&[u8]; 20]>::new();

    let should_add_key = |key: &[u8]| -> bool {
        // Key exists if it's in original metadata OR added via UpdateMetadataField
        let exists_in_original =
            metadata.is_some_and(|items| items.iter().any(|item| item.key == key));
        let added_via_update = actions.iter().any(|action| {
            matches!(action, ZAction::UpdateMetadataField(update)
                if update.extension_index as usize == extension_index
                    && update.field_type == 3
                    && update.key == key)
        });

        // Key should be included if it exists and is not removed
        let should_exist = exists_in_original || added_via_update;
        let is_removed = actions.iter().any(|action| {
            matches!(action, ZAction::RemoveMetadataKey(remove)
                if remove.extension_index as usize == extension_index
                    && remove.key == key)
        });

        should_exist && !is_removed
    };

    // Process all original metadata keys
    if let Some(items) = metadata {
        for item in items.iter() {
            if should_add_key(item.key) {
                let final_value_len = actions
                    .iter()
                    .rev()
                    .find_map(|action| match action {
                        ZAction::UpdateMetadataField(update)
                            if update.extension_index as usize == extension_index
                                && update.field_type == 3
                                && update.key == item.key =>
                        {
                            Some(update.value.len())
                        }
                        _ => None,
                    })
                    .unwrap_or(item.value.len());

                configs.push(AdditionalMetadataConfig {
                    key: item.key.len() as u32,
                    value: final_value_len as u32,
                });
                processed_keys.push(item.key);
            }
        }
    }

    // Process new keys from UpdateMetadataField actions
    for action in actions.iter() {
        if let ZAction::UpdateMetadataField(update) = action {
            if update.extension_index as usize == extension_index
                && update.field_type == 3
                && !processed_keys.contains(&update.key)
                && should_add_key(update.key)
            {
                let final_value_len = actions
                    .iter()
                    .rev()
                    .find_map(|later_action| match later_action {
                        ZAction::UpdateMetadataField(later_update)
                            if later_update.extension_index as usize == extension_index
                                && later_update.field_type == 3
                                && later_update.key == update.key =>
                        {
                            Some(later_update.value.len())
                        }
                        _ => None,
                    })
                    .unwrap_or(update.value.len());

                configs.push(AdditionalMetadataConfig {
                    key: update.key.len() as u32,
                    value: final_value_len as u32,
                });
                processed_keys.push(update.key);
            }
        }
    }

    configs.into_iter().collect()
}

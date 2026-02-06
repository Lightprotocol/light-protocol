pub mod check_mint_extensions;
pub mod processor;
pub mod token_metadata;

// Re-export extension checking functions
pub use check_mint_extensions::{
    check_mint_extensions, has_mint_extensions, parse_mint_extensions, MintExtensionChecks,
};
use light_program_profiler::profile;
// Import from light-token-interface instead of local modules
use light_token_interface::{
    instructions::mint_action::ZAction,
    state::{
        AdditionalMetadata, AdditionalMetadataConfig, ExtensionStruct, ExtensionStructConfig,
        TokenMetadata, TokenMetadataConfig,
    },
    TokenError,
};
// Re-export from token-interface (consolidated types)
pub use light_token_interface::{
    is_restricted_extension, MintExtensionFlags, ALLOWED_EXTENSION_TYPES,
    RESTRICTED_EXTENSION_TYPES,
};
use light_zero_copy::ZeroCopyNew;
use spl_pod::solana_msg::msg;

/// Returns true if extension should be included in compressed account output.
#[inline(always)]
pub fn should_include_in_compressed_output(extension: &ExtensionStruct) -> bool {
    matches!(extension, ExtensionStruct::TokenMetadata(_))
}

/// Action-aware version that calculates maximum sizes needed for field updates
/// Returns: (has_extensions, extension_configs, additional_data_len)
#[profile]
pub fn process_extensions_config_with_actions(
    extensions: Option<&Vec<ExtensionStruct>>,
    actions: &[ZAction],
) -> Result<(bool, Vec<ExtensionStructConfig>, usize), TokenError> {
    let mut additional_mint_data_len = 0;
    let mut config_vec = Vec::new();

    // Process existing extensions from state
    // NOTE: Compressible extension is NOT included in compressed account output.
    // It only lives in the CMint Solana account.
    if let Some(extensions) = extensions {
        for (extension_index, extension) in extensions.iter().enumerate() {
            if !should_include_in_compressed_output(extension) {
                continue;
            }
            match extension {
                ExtensionStruct::TokenMetadata(token_metadata) => {
                    process_token_metadata_config_with_actions(
                        &mut additional_mint_data_len,
                        &mut config_vec,
                        token_metadata,
                        actions,
                        extension_index,
                    )?
                }
                _ => return Err(TokenError::UnsupportedExtension),
            }
        }
    }

    // NOTE: DecompressMint does NOT add Compressible to compressed account output.
    // Compressible extension only lives in the CMint Solana account, not in the compressed account.
    // The CMint sync logic handles adding Compressible when writing to CMint.

    let has_extensions = !config_vec.is_empty();
    Ok((has_extensions, config_vec, additional_mint_data_len))
}

fn process_token_metadata_config_with_actions(
    additional_mint_data_len: &mut usize,
    config_vec: &mut Vec<ExtensionStructConfig>,
    token_metadata: &TokenMetadata,
    actions: &[ZAction],
    extension_index: usize,
) -> Result<(), TokenError> {
    // Early validation - no allocations needed
    if token_metadata.additional_metadata.len() > 20 {
        msg!(
            "Too many additional metadata elements: {} (max 20)",
            token_metadata.additional_metadata.len()
        );
        return Err(TokenError::TooManyAdditionalMetadata);
    }

    // Check for duplicate keys (O(n^2) but acceptable for max 20 items)
    for i in 0..token_metadata.additional_metadata.len() {
        for j in (i + 1)..token_metadata.additional_metadata.len() {
            if token_metadata.additional_metadata[i].key
                == token_metadata.additional_metadata[j].key
            {
                msg!("Duplicate metadata key found at positions {} and {}", i, j);
                return Err(TokenError::DuplicateMetadataKey);
            }
        }
    }

    // Single-pass state accumulator - track final sizes directly
    let mut final_name_len = token_metadata.name.len();
    let mut final_symbol_len = token_metadata.symbol.len();
    let mut final_uri_len = token_metadata.uri.len();

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
        &token_metadata.additional_metadata,
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
    metadata: &[AdditionalMetadata],
    actions: &[ZAction],
    extension_index: usize,
) -> Vec<AdditionalMetadataConfig> {
    let mut configs = arrayvec::ArrayVec::<AdditionalMetadataConfig, 20>::new();
    let mut processed_keys = tinyvec::ArrayVec::<[&[u8]; 20]>::new();

    let should_add_key = |key: &[u8]| -> bool {
        // Start with whether the key exists in original metadata
        let mut exists = metadata.iter().any(|item| item.key == key);
        // Process actions in order to determine final state
        // (handles add-remove-add sequences correctly)
        for action in actions {
            match action {
                ZAction::UpdateMetadataField(update)
                    if update.extension_index as usize == extension_index
                        && update.field_type == 3
                        && update.key == key =>
                {
                    exists = true;
                }
                ZAction::RemoveMetadataKey(remove)
                    if remove.extension_index as usize == extension_index
                        && remove.key == key =>
                {
                    exists = false;
                }
                _ => {}
            }
        }
        exists
    };

    // Process all original metadata keys
    for item in metadata.iter() {
        if should_add_key(&item.key) {
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
            processed_keys.push(&item.key);
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

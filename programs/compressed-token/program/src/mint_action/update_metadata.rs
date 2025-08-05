use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_compressed_account::Pubkey;
use light_ctoken_types::{
    instructions::mint_actions::{ZUpdateMetadataFieldAction, ZUpdateMetadataAuthorityAction},
    state::{ZCompressedMintMut, ZExtensionStructMut},
};
use spl_pod::solana_msg::msg;

/// Process update metadata field action - modifies the instruction data extensions directly
pub fn process_update_metadata_field_action<'a>(
    action: &ZUpdateMetadataFieldAction,
    compressed_mint: &mut ZCompressedMintMut<'a>,
    authority: &Pubkey,
) -> Result<(), ProgramError> {
    let extensions = compressed_mint.extensions.as_mut().ok_or_else(|| {
        msg!("No extensions found - cannot update metadata");
        ErrorCode::MintActionMissingMetadataExtension
    })?;

    let extension_index = action.extension_index as usize;
    if extension_index >= extensions.len() {
        msg!("Extension index {} out of bounds", extension_index);
        return Err(ErrorCode::MintActionInvalidExtensionIndex.into());
    }

    // Get the metadata extension
    match &mut extensions.as_mut_slice()[extension_index] {
        ZExtensionStructMut::TokenMetadata(ref mut metadata) => {
            if let Some(update_authority) = metadata.update_authority.as_ref() {
                if update_authority.to_bytes() != authority.to_bytes() {
                    msg!(
                        "Authority {:?} cannot update metadata, expected {:?}",
                        authority,
                        update_authority
                    );
                    return Err(ErrorCode::MintActionInvalidMintAuthority.into());
                }
            } else {
                msg!("Metadata has no update authority - cannot be updated");
                return Err(ErrorCode::MintActionInvalidMintAuthority.into());
            }

            match action.field_type {
                0 => {
                    // Update name
                    metadata.metadata.name.copy_from_slice(action.value);
                    msg!("Updated metadata name");
                }
                1 => {
                    // Update symbol
                    metadata.metadata.symbol.copy_from_slice(action.value);
                    msg!("Updated metadata symbol");
                }
                2 => {
                    // Update uri
                    metadata.metadata.uri.copy_from_slice(action.value);
                    msg!("Updated metadata uri");
                }
                _ => {
                    // Find existing key or add new one
                    let mut found = false;
                    for metadata_pair in metadata.additional_metadata.iter_mut() {
                        if metadata_pair.key == action.key {
                            metadata_pair.value.copy_from_slice(action.value);
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        msg!("Adding new custom key-value pair not supported in zero-copy mode");
                        return Err(ErrorCode::MintActionUnsupportedOperation.into());
                    }

                    let key_str = String::from_utf8_lossy(action.key);
                    msg!("Updated metadata custom key: {}", key_str);
                }
            }
        }
        _ => {
            msg!(
                "Extension at index {} is not a TokenMetadata extension",
                extension_index
            );
            return Err(ErrorCode::MintActionInvalidExtensionType.into());
        }
    }

    msg!("Successfully updated metadata field");
    Ok(())
}

/// Process update metadata authority action - modifies the instruction data extensions directly
pub fn process_update_metadata_authority_action<'a>(
    action: &ZUpdateMetadataAuthorityAction,
    compressed_mint: &mut ZCompressedMintMut<'a>,
    authority: &Pubkey,
) -> Result<(), ProgramError> {
    let extensions = compressed_mint.extensions.as_mut().ok_or_else(|| {
        msg!("No extensions found - cannot update metadata authority");
        ErrorCode::MintActionMissingMetadataExtension
    })?;

    let extension_index = action.extension_index as usize;
    if extension_index >= extensions.len() {
        msg!("Extension index {} out of bounds", extension_index);
        return Err(ErrorCode::MintActionInvalidExtensionIndex.into());
    }

    // Verify authority can update metadata
    match &extensions.as_slice()[extension_index] {
        ZExtensionStructMut::TokenMetadata(metadata) => {
            if let Some(update_authority) = metadata.update_authority.as_ref() {
                if update_authority.to_bytes() != authority.to_bytes() {
                    msg!(
                        "Authority {:?} cannot update metadata authority, expected {:?}",
                        authority,
                        update_authority
                    );
                    return Err(ErrorCode::MintActionInvalidMintAuthority.into());
                }
            } else {
                msg!("Metadata has no update authority - cannot be updated");
                return Err(ErrorCode::MintActionInvalidMintAuthority.into());
            }
        }
        _ => {
            msg!("Extension at index {} is not a TokenMetadata extension", extension_index);
            return Err(ErrorCode::MintActionInvalidExtensionType.into());
        }
    }

    // Get the metadata extension and update the authority
    match &mut extensions.as_mut_slice()[extension_index] {
        ZExtensionStructMut::TokenMetadata(ref mut metadata) => {
            let new_authority = if action.new_authority.to_bytes() == [0u8; 32] {
                None // Revoke authority by setting to None
            } else {
                Some(action.new_authority)
            };

            if let Some(authority_ref) = metadata.update_authority.as_mut() {
                if let Some(new_auth) = new_authority {
                    **authority_ref = new_auth;
                } else {
                    // Can't set to None with zero-copy, would need different approach
                    msg!("Revoking authority not supported in zero-copy mode");
                    return Err(ErrorCode::MintActionUnsupportedOperation.into());
                }
            } else if new_authority.is_some() {
                msg!("Setting authority when none exists not supported in zero-copy mode");
                return Err(ErrorCode::MintActionUnsupportedOperation.into());
            }
            msg!("Updated metadata authority");
        }
        _ => {
            msg!(
                "Extension at index {} is not a TokenMetadata extension",
                extension_index
            );
            return Err(ErrorCode::MintActionInvalidExtensionType.into());
        }
    }

    msg!("Successfully updated metadata authority");
    Ok(())
}
/*
/// Process remove metadata key action - modifies the instruction data extensions directly
pub fn process_remove_metadata_key_action(
    action: &ZRemoveMetadataKeyAction,
    extensions: &mut Option<Vec<light_ctoken_types::instructions::extensions::ZExtensionInstructionData>>,
    _validated_accounts: &MintActionAccounts,
    accounts_config: &crate::mint_action::accounts::AccountsConfig,
) -> Result<(), ProgramError> {
    // Verify this is a decompressed mint
    if !accounts_config.is_decompressed {
        msg!("Metadata operations require decompressed mints");
        return Err(ErrorCode::MintActionMetadataNotDecompressed.into());
    }

    let extensions = extensions.as_mut().ok_or_else(|| {
        msg!("No extensions found - cannot remove metadata key");
        ErrorCode::MintActionMissingMetadataExtension
    })?;

    let extension_index = action.extension_index as usize;
    if extension_index >= extensions.len() {
        msg!("Extension index {} out of bounds", extension_index);
        return Err(ErrorCode::MintActionInvalidExtensionIndex.into());
    }

    // Get the metadata extension and remove the key
    match &mut extensions[extension_index] {
        light_ctoken_types::instructions::extensions::ZExtensionInstructionData::TokenMetadata(ref mut metadata) => {
            let key_bytes = action.key.to_vec();

            if let Some(additional_metadata) = &mut metadata.additional_metadata {
                let mut found_index = None;
                // Find the key to remove
                for (index, metadata_pair) in additional_metadata.iter().enumerate() {
                    if metadata_pair.key == key_bytes {
                        found_index = Some(index);
                        break;
                    }
                }

                if let Some(index) = found_index {
                    // Efficiently remove the item at index
                    additional_metadata.remove(index);
                    let key_str = String::from_utf8_lossy(&key_bytes);
                    msg!("Removed metadata key: {}", key_str);
                } else {
                    if action.idempotent == 0 {
                        let key_str = String::from_utf8_lossy(&key_bytes);
                        msg!("Metadata key '{}' not found and idempotent is false", key_str);
                        return Err(ErrorCode::MintActionMetadataKeyNotFound.into());
                    } else {
                        let key_str = String::from_utf8_lossy(&key_bytes);
                        msg!("Metadata key '{}' not found (idempotent mode)", key_str);
                    }
                }
            } else if action.idempotent == 0 {
                msg!("No additional metadata found and idempotent is false");
                return Err(ErrorCode::MintActionMetadataKeyNotFound.into());
            }
        }
        _ => {
            msg!("Extension at index {} is not a TokenMetadata extension", extension_index);
            return Err(ErrorCode::MintActionInvalidExtensionType.into());
        }
    }

    msg!("Successfully processed remove metadata key");
    Ok(())
}*/

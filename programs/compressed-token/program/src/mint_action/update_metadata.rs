use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_compressed_account::Pubkey;
use light_ctoken_types::{
    instructions::mint_actions::{
        ZRemoveMetadataKeyAction, ZUpdateMetadataAuthorityAction, ZUpdateMetadataFieldAction,
    },
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
                msg!("Metadata authority has been revoked - cannot update fields");
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

/// Process update metadata authority action
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

    // Get the metadata extension and update the authority
    match &mut extensions.as_mut_slice()[extension_index] {
        ZExtensionStructMut::TokenMetadata(ref mut metadata) => {
            // Verify current authority
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

            // Update authority
            if let Some(authority_ref) = metadata.update_authority.as_mut() {
                if action.new_authority.to_bytes() == [0u8; 32] {
                    msg!("Authority revoked - metadata can no longer be updated");
                } else {
                    **authority_ref = action.new_authority;
                }
                msg!("Updated metadata authority");
            } else {
                // Authority is None - this should happen when revoked during allocation
                if action.new_authority.to_bytes() != [0u8; 32] {
                    msg!("Cannot set authority when none was allocated");
                    return Err(ErrorCode::MintActionUnsupportedOperation.into());
                }
                msg!("Authority remains revoked");
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

    Ok(())
}

/// Only checks authority, the key is removed during data allocation.
pub fn process_remove_metadata_key_action(
    action: &ZRemoveMetadataKeyAction,
    compressed_mint: &ZCompressedMintMut<'_>,
    authority: &Pubkey,
) -> Result<(), ProgramError> {
    let extensions = compressed_mint.extensions.as_ref().ok_or_else(|| {
        msg!("No extensions found - cannot update metadata authority");
        ErrorCode::MintActionMissingMetadataExtension
    })?;

    let extension_index = action.extension_index as usize;
    if extension_index >= extensions.len() {
        msg!("Extension index {} out of bounds", extension_index);
        return Err(ErrorCode::MintActionInvalidExtensionIndex.into());
    }

    // Get the metadata extension and update the authority
    match &extensions.as_slice()[extension_index] {
        ZExtensionStructMut::TokenMetadata(ref metadata) => {
            // Verify current authority
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
            msg!(
                "Extension at index {} is not a TokenMetadata extension",
                extension_index
            );
            return Err(ErrorCode::MintActionInvalidExtensionType.into());
        }
    }
    Ok(())
}

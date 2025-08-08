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

/// Validates metadata update authority against expected authority
fn check_metadata_update_authority(
    validated_accounts: &crate::mint_action::accounts::MintActionAccounts,
    expected_authority: &Option<Pubkey>,
    operation_name: &str,
) -> Result<(), ProgramError> {
    // Authority signer validation is handled by validate_and_parse()
    match expected_authority {
        Some(expected) => {
            if expected.to_bytes() != *validated_accounts.authority.key() {
                msg!(
                    "Authority {:?} cannot perform {}, expected {:?}",
                    validated_accounts.authority.key(),
                    operation_name,
                    expected
                );
                return Err(ErrorCode::MintActionInvalidMintAuthority.into());
            }
        }
        None => {
            msg!(
                "Metadata authority has been revoked - cannot perform {}",
                operation_name
            );
            return Err(ErrorCode::MintActionInvalidMintAuthority.into());
        }
    }
    Ok(())
}

/// Copies metadata value with length validation to prevent buffer overflow
fn safe_copy_metadata_value(
    dest: &mut [u8],
    src: &[u8],
    field_name: &str,
) -> Result<(), ProgramError> {
    // Validate source length fits in destination buffer
    if src.len() > dest.len() {
        msg!(
            "Metadata {} value too large: {} bytes, maximum allowed: {} bytes",
            field_name,
            src.len(),
            dest.len()
        );
        return Err(ErrorCode::MintActionUnsupportedOperation.into());
    }

    // Safe and efficient copy - clear entire buffer for security
    dest.fill(0);
    dest[..src.len()].copy_from_slice(src);
    Ok(())
}

/// Process update metadata field action - modifies the instruction data extensions directly
pub fn process_update_metadata_field_action(
    action: &ZUpdateMetadataFieldAction,
    compressed_mint: &mut ZCompressedMintMut<'_>,
    validated_accounts: &crate::mint_action::accounts::MintActionAccounts,
) -> Result<(), ProgramError> {
    let extensions = compressed_mint.extensions.as_mut().ok_or_else(|| {
        msg!("No extensions found - cannot update metadata");
        ErrorCode::MintActionMissingMetadataExtension
    })?;

    // Validate extension index bounds
    let extension_index = action.extension_index as usize;
    if extension_index >= extensions.len() {
        msg!(
            "Extension index {} out of bounds, available extensions: {}",
            extension_index,
            extensions.len()
        );
        return Err(ErrorCode::MintActionInvalidExtensionIndex.into());
    }

    // Get the metadata extension
    match &mut extensions.as_mut_slice()[extension_index] {
        ZExtensionStructMut::TokenMetadata(ref mut metadata) => {
            // Validate update authority matches expected authority
            let expected_authority = metadata
                .update_authority
                .as_ref()
                .map(|auth| Pubkey::from(auth.to_bytes()));
            check_metadata_update_authority(
                validated_accounts,
                &expected_authority,
                "metadata field update",
            )?;

            // Update metadata fields with length validation
            match action.field_type {
                0 => {
                    // Update name
                    safe_copy_metadata_value(&mut metadata.metadata.name, action.value, "name")?;
                    msg!("Updated metadata name");
                }
                1 => {
                    // Update symbol
                    safe_copy_metadata_value(
                        &mut metadata.metadata.symbol,
                        action.value,
                        "symbol",
                    )?;
                    msg!("Updated metadata symbol");
                }
                2 => {
                    // Update uri
                    safe_copy_metadata_value(&mut metadata.metadata.uri, action.value, "uri")?;
                    msg!("Updated metadata uri");
                }
                _ => {
                    // Find existing key or add new one
                    // Validate additional_metadata is not empty before processing
                    if metadata.additional_metadata.is_empty() {
                        msg!("No additional metadata fields available for custom key updates");
                        return Err(ErrorCode::MintActionUnsupportedOperation.into());
                    }
                    let mut found = false;
                    for metadata_pair in metadata.additional_metadata.iter_mut() {
                        if metadata_pair.key == action.key {
                            safe_copy_metadata_value(
                                &mut metadata_pair.value,
                                action.value,
                                "custom field",
                            )?;
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

    // Invariant check: Verify metadata state is valid after update
    validate_metadata_invariants(compressed_mint, "field update")?;
    Ok(())
}

/// Validates metadata invariants to ensure consistent state
fn validate_metadata_invariants(
    compressed_mint: &ZCompressedMintMut<'_>,
    operation: &str,
) -> Result<(), ProgramError> {
    if let Some(extensions) = compressed_mint.extensions.as_ref() {
        // Ensure we have at least one extension if extensions exist
        if extensions.is_empty() {
            msg!(
                "Invalid state after {}: extensions array exists but is empty",
                operation
            );
            return Err(ErrorCode::MintActionInvalidExtensionType.into());
        }
    }
    Ok(())
}

/// Process update metadata authority action
pub fn process_update_metadata_authority_action(
    action: &ZUpdateMetadataAuthorityAction,
    compressed_mint: &mut ZCompressedMintMut<'_>,
    validated_accounts: &crate::mint_action::accounts::MintActionAccounts,
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
            // Validate current authority before updating
            let expected_authority = metadata
                .update_authority
                .as_ref()
                .map(|auth| Pubkey::from(auth.to_bytes()));
            check_metadata_update_authority(
                validated_accounts,
                &expected_authority,
                "metadata authority update",
            )?;

            let new_authority = if action.new_authority.to_bytes() == [0u8; 32] {
                None
            } else {
                Some(action.new_authority)
            };

            match (metadata.update_authority.as_mut(), new_authority) {
                (Some(field_ref), Some(new_auth)) => {
                    **field_ref = new_auth;
                }
                (Some(_), None) => {
                    msg!("Authority revocation must happen at allocation time");
                    return Err(ErrorCode::MintActionUnsupportedOperation.into());
                }
                (None, Some(_)) => {
                    msg!("Cannot set authority when none was allocated");
                    return Err(ErrorCode::MintActionUnsupportedOperation.into());
                }
                (None, None) => {}
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

    // Invariant check: Verify metadata state is valid after authority update
    validate_metadata_invariants(compressed_mint, "authority update")?;
    Ok(())
}

/// Only checks authority, the key is removed during data allocation.
pub fn process_remove_metadata_key_action(
    action: &ZRemoveMetadataKeyAction,
    compressed_mint: &ZCompressedMintMut<'_>,
    validated_accounts: &crate::mint_action::accounts::MintActionAccounts,
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

    // Get the metadata extension and verify authority
    match &extensions.as_slice()[extension_index] {
        ZExtensionStructMut::TokenMetadata(ref metadata) => {
            // Validate authority before removing metadata key
            let expected_authority = metadata
                .update_authority
                .as_ref()
                .map(|auth| Pubkey::from(auth.to_bytes()));
            check_metadata_update_authority(
                validated_accounts,
                &expected_authority,
                "metadata key removal",
            )?;
        }
        _ => {
            msg!(
                "Extension at index {} is not a TokenMetadata extension",
                extension_index
            );
            return Err(ErrorCode::MintActionInvalidExtensionType.into());
        }
    }

    // Invariant check: Verify metadata state is valid after key removal
    validate_metadata_invariants(compressed_mint, "key removal")?;
    Ok(())
}

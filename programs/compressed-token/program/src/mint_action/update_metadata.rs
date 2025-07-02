use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_compressed_account::Pubkey;
use light_ctoken_types::{
    instructions::mint_actions::{
        ZRemoveMetadataKeyAction, ZUpdateMetadataAuthorityAction, ZUpdateMetadataFieldAction,
    },
    state::{ZCompressedMintMut, ZExtensionStructMut},
};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use spl_pod::solana_msg::msg;

/// Simple authority check helper - validates that authority is Some (signer was validated)
fn check_validated_metadata_authority(
    validated_metadata_authority: &Option<Pubkey>,
    authority: &<Option<Pubkey> as ZeroCopyAtMut<'_>>::ZeroCopyAtMut,
    operation_name: &str,
) -> Result<(), ProgramError> {
    if let Some(validated_metadata_authority) = validated_metadata_authority {
        msg!("authority {:?} ", authority);
        let authority = authority.as_ref().ok_or(ProgramError::from(
            ErrorCode::MintActionInvalidMintAuthority,
        ))?;

        if *validated_metadata_authority != **authority {
            msg!(
                "validated_metadata_authority {:?} authority {:?}",
                validated_metadata_authority,
                **authority
            );
            return Err(ErrorCode::MintActionInvalidMintAuthority.into());
        }
    } else {
        msg!(
            "Metadata authority validation failed for {}: no valid metadata authority",
            operation_name
        );
        return Err(ErrorCode::MintActionInvalidMintAuthority.into());
    }
    msg!(
        "Metadata authority validation passed for {}",
        operation_name
    );
    Ok(())
}

/// Copies metadata value with length validation to prevent buffer overflow
pub fn safe_copy_metadata_value(
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
    validated_metadata_authority: &Option<light_compressed_account::Pubkey>,
) -> Result<(), ProgramError> {
    msg!("update_metadata_field_action: ENTRY");
    msg!(
        "extension_index={}, field_type={}",
        action.extension_index,
        action.field_type
    );
    let extensions = compressed_mint.extensions.as_mut().ok_or_else(|| {
        msg!("No extensions found - cannot update metadata");
        ErrorCode::MintActionMissingMetadataExtension
    })?;
    msg!("Found {} extensions", extensions.len());

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
    msg!("Extension index {} is valid", extension_index);

    // Get the metadata extension
    msg!("About to match on extension type");
    match &mut extensions.as_mut_slice()[extension_index] {
        ZExtensionStructMut::TokenMetadata(ref mut metadata) => {
            msg!("Matched TokenMetadata extension");
            // Simple authority check: validated_metadata_authority must be Some
            check_validated_metadata_authority(
                validated_metadata_authority,
                &metadata.update_authority,
                "metadata field update",
            )?;

            // Update metadata fields with length validation
            msg!("About to process field type {}", action.field_type);
            match action.field_type {
                0 => {
                    msg!(
                        "Processing name field update, buffer len: {}, value len: {}",
                        metadata.metadata.name.len(),
                        action.value.len()
                    );
                    // Update name
                    safe_copy_metadata_value(metadata.metadata.name, action.value, "name")?;
                    msg!("Updated metadata name");
                }
                1 => {
                    // Update symbol
                    safe_copy_metadata_value(metadata.metadata.symbol, action.value, "symbol")?;
                    msg!("Updated metadata symbol");
                }
                2 => {
                    // Update uri
                    safe_copy_metadata_value(metadata.metadata.uri, action.value, "uri")?;
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
                                metadata_pair.value,
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

/// Updates metadata authority field when allocation and action match
fn update_metadata_authority_field(
    metadata_authority: &mut <Option<light_compressed_account::Pubkey> as ZeroCopyAtMut<'_>>::ZeroCopyAtMut,
    new_authority: Option<light_compressed_account::Pubkey>,
) -> Result<(), ProgramError> {
    match (metadata_authority.as_mut(), new_authority) {
        (Some(field_ref), Some(new_auth)) => {
            // Update existing authority to new value
            **field_ref = new_auth;
            msg!("Authority updated successfully");
        }
        (None, None) => {
            // Authority was correctly revoked during allocation - nothing to do
            msg!("Authority successfully revoked");
        }
        (Some(_), None) => {
            // This should never happen with correct allocation logic
            msg!("Internal error: authority field allocated but should be revoked");
            return Err(ErrorCode::MintActionUnsupportedOperation.into());
        }
        (None, Some(_)) => {
            // This should never happen with correct allocation logic
            msg!("Internal error: no authority field allocated but trying to set authority");
            return Err(ErrorCode::MintActionUnsupportedOperation.into());
        }
    }
    Ok(())
}

/// Process update metadata authority action
pub fn process_update_metadata_authority_action(
    action: &ZUpdateMetadataAuthorityAction,
    compressed_mint: &mut ZCompressedMintMut<'_>,
    instruction_data_mint_authority: &<Option<light_compressed_account::Pubkey> as ZeroCopyAt<
        '_,
    >>::ZeroCopyAt,
    validated_metadata_authority: &mut Option<light_compressed_account::Pubkey>,
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
            let new_authority = if action.new_authority.to_bytes() == [0u8; 32] {
                None
            } else {
                Some(action.new_authority)
            };

            if metadata.update_authority.is_none() {
                let instruction_data_mint_authority = instruction_data_mint_authority
                    .ok_or(ErrorCode::MintActionInvalidMintAuthority)?;
                msg!(
                    "instruction_data_mint_authority {:?}",
                    solana_pubkey::Pubkey::new_from_array(
                        instruction_data_mint_authority.to_bytes()
                    )
                );
                {
                    let validated_metadata_authority = validated_metadata_authority
                        .as_ref()
                        .ok_or(ErrorCode::MintActionInvalidMintAuthority)?;
                    msg!(
                        "validated_metadata_authority {:?}",
                        solana_pubkey::Pubkey::new_from_array(
                            validated_metadata_authority.to_bytes()
                        )
                    );
                    if *instruction_data_mint_authority != *validated_metadata_authority {
                        msg!(
                        "Metadata authority validation failed for metadata authority update: no valid metadata authority"
                    );
                        return Err(ErrorCode::MintActionInvalidMintAuthority.into());
                    }
                }
            } else {
                msg!("here4");
                // Simple authority check: validated_metadata_authority must be Some to perform authority operations
                check_validated_metadata_authority(
                    validated_metadata_authority,
                    &metadata.update_authority,
                    "metadata authority update",
                )?;

                update_metadata_authority_field(&mut metadata.update_authority, new_authority)?;
            } // Update the validated authority state for future actions
            *validated_metadata_authority = new_authority;
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
    validated_metadata_authority: &Option<light_compressed_account::Pubkey>,
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

    // Verify extension exists and is TokenMetadata
    match &extensions.as_slice()[extension_index] {
        ZExtensionStructMut::TokenMetadata(metadata) => {
            msg!("TokenMetadata extension validated for key removal");
            check_validated_metadata_authority(
                validated_metadata_authority,
                &metadata.update_authority,
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

use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_compressed_account::Pubkey;
use light_ctoken_types::{
    instructions::mint_action::{
        ZRemoveMetadataKeyAction, ZUpdateMetadataAuthorityAction, ZUpdateMetadataFieldAction,
    },
    state::{ZCompressedMintMut, ZExtensionStructMut},
};
use light_profiler::profile;
use light_zero_copy::traits::ZeroCopyAt;
use spl_pod::solana_msg::msg;

/// Simple authority check helper - validates that authority is Some (signer was validated)
#[profile]
fn check_validated_metadata_authority(
    validated_metadata_authority: &Option<Pubkey>,
    authority: &Pubkey,
    operation_name: &str,
) -> Result<(), ProgramError> {
    if let Some(validated_metadata_authority) = validated_metadata_authority {
        if *authority == Pubkey::new_from_array([0u8; 32]) {
            return Err(ErrorCode::MintActionInvalidMintAuthority.into());
        }

        if *validated_metadata_authority != *authority {
            return Err(ErrorCode::MintActionInvalidMintAuthority.into());
        }
    } else {
        msg!(
            "Metadata authority validation failed for {}: no valid metadata authority",
            operation_name
        );
        return Err(ErrorCode::MintActionInvalidMintAuthority.into());
    }
    Ok(())
}

/// Get mutable reference to metadata extension at specified index
#[profile]
fn get_metadata_extension_mut<'a, 'b>(
    compressed_mint: &'a mut ZCompressedMintMut<'b>,
    extension_index: usize,
    operation_name: &str,
    validated_metadata_authority: &Option<light_compressed_account::Pubkey>,
    instruction_data_mint_authority: Option<
        &<Option<light_compressed_account::Pubkey> as ZeroCopyAt<'_>>::ZeroCopyAt,
    >,
) -> Result<&'a mut light_ctoken_types::state::ZTokenMetadataMut<'b>, ProgramError> {
    let extensions = compressed_mint.extensions.as_mut().ok_or_else(|| {
        msg!("No extensions found - cannot {}", operation_name);
        ErrorCode::MintActionMissingMetadataExtension
    })?;

    // Validate extension index bounds
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
            if metadata.update_authority == Pubkey::new_from_array([0u8; 32]) {
                // Authority has been revoked (set to zero)
                if let Some(instruction_data_mint_authority) = instruction_data_mint_authority {
                    // Special case: Allow mint authority to set initial metadata authority
                    let instruction_data_mint_authority = instruction_data_mint_authority
                        .ok_or(ErrorCode::MintActionInvalidMintAuthority)?;
                    {
                        let validated_metadata_authority = validated_metadata_authority
                            .as_ref()
                            .ok_or(ErrorCode::MintActionInvalidMintAuthority)?;
                        if *instruction_data_mint_authority != *validated_metadata_authority {
                            msg!(
                                "Metadata authority validation failed for {}: no valid metadata authority",
                                operation_name
                            );
                            return Err(ErrorCode::MintActionInvalidMintAuthority.into());
                        }
                    }
                } else {
                    // Regular operations are not allowed when authority is revoked
                    msg!(
                        "Metadata authority has been revoked - {} not allowed",
                        operation_name
                    );
                    return Err(ErrorCode::MintActionInvalidMintAuthority.into());
                }
            } else {
                // Simple authority check: validated_metadata_authority must be Some to perform authority operations
                check_validated_metadata_authority(
                    validated_metadata_authority,
                    &metadata.update_authority,
                    operation_name,
                )?;
            }

            Ok(metadata)
        }
        _ => {
            msg!(
                "Extension at index {} is not a TokenMetadata extension",
                extension_index
            );
            Err(ErrorCode::MintActionInvalidExtensionType.into())
        }
    }
}

/// Conditionally updates a metadata field only if the allocated size matches the value size.
/// If sizes don't match, this action is skipped (a later action will apply the final update).
/// This is safe because the data allocation iterates over all actions and allocates accordingly.
///
/// Note: We don't need to zero out the data because we always overwrite the complete
/// metadata field with an exact size match, ensuring no stale data remains.
#[inline(always)]
#[profile]
fn conditional_metadata_update(dest: &mut [u8], src: &[u8]) {
    if dest.len() == src.len() {
        // Size matches: this is the final action for this field, apply the update
        dest.copy_from_slice(src);
    }
}

/// Process update metadata field action - modifies the instruction data extensions directly
#[profile]
pub fn process_update_metadata_field_action(
    action: &ZUpdateMetadataFieldAction,
    compressed_mint: &mut ZCompressedMintMut<'_>,
    validated_metadata_authority: &Option<light_compressed_account::Pubkey>,
) -> Result<(), ProgramError> {
    let metadata = get_metadata_extension_mut(
        compressed_mint,
        action.extension_index as usize,
        "metadata field update",
        validated_metadata_authority,
        None,
    )?;

    // Update metadata fields - only apply if allocated size matches action value size
    match action.field_type {
        0 => {
            conditional_metadata_update(metadata.name, action.value);
        }
        1 => {
            conditional_metadata_update(metadata.symbol, action.value);
        }
        2 => {
            conditional_metadata_update(metadata.uri, action.value);
        }
        _ => {
            // Find existing key and conditionally update
            if metadata.additional_metadata.is_empty() {
                return Err(ErrorCode::MintActionUnsupportedOperation.into());
            }
            if let Some(metadata_pair) = metadata
                .additional_metadata
                .iter_mut()
                .find(|metadata_pair| metadata_pair.key == action.key)
            {
                conditional_metadata_update(metadata_pair.value, action.value);
            } else {
                return Err(ErrorCode::MintActionUnsupportedOperation.into());
            }
        }
    }
    Ok(())
}

/// Updates metadata authority field when allocation and action match
#[profile]
fn update_metadata_authority_field(
    metadata_authority: &mut light_compressed_account::Pubkey,
    new_authority: Option<light_compressed_account::Pubkey>,
) -> Result<(), ProgramError> {
    match (new_authority, metadata_authority.to_bytes() == [0u8; 32]) {
        (Some(new_auth), false) => {
            // Update existing authority to new value
            *metadata_authority = new_auth;
            msg!("Authority updated successfully");
        }
        (None, false) => {
            // Revoke authority by setting to zero
            *metadata_authority = light_compressed_account::Pubkey::from([0u8; 32]);
            msg!("Authority successfully revoked");
        }
        (Some(_), true) => {
            // This should never happen with correct allocation logic
            msg!("Internal error: no authority field allocated but trying to set authority");
            return Err(ErrorCode::MintActionUnsupportedOperation.into());
        }
        (None, true) => {
            // This should never happen with correct allocation logic
            msg!("Internal error: authority field allocated but should be revoked");
            return Err(ErrorCode::MintActionUnsupportedOperation.into());
        }
    }
    Ok(())
}

/// Process update metadata authority action
#[profile]
pub fn process_update_metadata_authority_action(
    action: &ZUpdateMetadataAuthorityAction,
    compressed_mint: &mut ZCompressedMintMut<'_>,
    instruction_data_mint_authority: &<Option<light_compressed_account::Pubkey> as ZeroCopyAt<
        '_,
    >>::ZeroCopyAt,
    validated_metadata_authority: &mut Option<light_compressed_account::Pubkey>,
) -> Result<(), ProgramError> {
    let metadata = get_metadata_extension_mut(
        compressed_mint,
        action.extension_index as usize,
        "update metadata authority",
        validated_metadata_authority,
        Some(instruction_data_mint_authority),
    )?;

    let new_authority = if action.new_authority.to_bytes() == [0u8; 32] {
        None
    } else {
        Some(action.new_authority)
    };

    update_metadata_authority_field(&mut metadata.update_authority, new_authority)?;
    // Update the validated authority state for future actions
    *validated_metadata_authority = new_authority;
    Ok(())
}

/// Only checks authority, the key is removed during data allocation.
#[profile]
pub fn process_remove_metadata_key_action(
    action: &ZRemoveMetadataKeyAction,
    compressed_mint: &mut ZCompressedMintMut<'_>,
    validated_metadata_authority: &Option<light_compressed_account::Pubkey>,
) -> Result<(), ProgramError> {
    let _metadata = get_metadata_extension_mut(
        compressed_mint,
        action.extension_index as usize,
        "metadata key removal",
        validated_metadata_authority,
        None,
    )?;

    msg!("TokenMetadata extension validated for key removal");
    Ok(())
}

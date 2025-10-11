use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_compressed_account::Pubkey;
use light_ctoken_types::{
    instructions::mint_action::{
        ZRemoveMetadataKeyAction, ZUpdateMetadataAuthorityAction, ZUpdateMetadataFieldAction,
    },
    state::{CompressedMint, ExtensionStruct, TokenMetadata},
};
use light_program_profiler::profile;
use spl_pod::solana_msg::msg;

use crate::mint_action::check_authority;

/// Get mutable reference to metadata extension at specified index
#[profile]
#[track_caller]
fn get_metadata_extension_mut<'a>(
    compressed_mint: &'a mut CompressedMint,
    extension_index: usize,
    operation_name: &str,
    signer: &pinocchio::pubkey::Pubkey,
) -> Result<&'a mut TokenMetadata, ProgramError> {
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
    match &mut extensions.as_mut_slice()[extension_index] {
        ExtensionStruct::TokenMetadata(ref mut metadata) => {
            check_authority(Some(metadata.update_authority), signer, operation_name)?;
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

/// Process update metadata field action - modifies the instruction data extensions directly
#[profile]
pub fn process_update_metadata_field_action(
    action: &ZUpdateMetadataFieldAction,
    compressed_mint: &mut CompressedMint,
    signer: &pinocchio::pubkey::Pubkey,
) -> Result<(), ProgramError> {
    let metadata = get_metadata_extension_mut(
        compressed_mint,
        action.extension_index as usize,
        "metadata field update",
        signer,
    )?;

    // Update metadata fields - only apply if allocated size matches action value size
    match action.field_type {
        0 => {
            metadata.name = action.value.to_vec();
        }
        1 => {
            metadata.symbol = action.value.to_vec();
        }
        2 => {
            metadata.uri = action.value.to_vec();
        }
        _ => {
            // Find existing key and update, or add new key
            if let Some(metadata_pair) = metadata
                .additional_metadata
                .iter_mut()
                .find(|metadata_pair| metadata_pair.key == action.key)
            {
                // Update existing key
                metadata_pair.value = action.value.to_vec();
            } else {
                // TODO: Enable adding new keys for SPL Token-2022 compatibility
                // metadata.additional_metadata.push(
                //     light_ctoken_types::state::AdditionalMetadata {
                //         key: action.key.to_vec(),
                //         value: action.value.to_vec(),
                //     }
                // );
                return Err(ErrorCode::MintActionUnsupportedOperation.into());
            }
        }
    }
    Ok(())
}

/// Process update metadata authority action
#[profile]
pub fn process_update_metadata_authority_action(
    action: &ZUpdateMetadataAuthorityAction,
    compressed_mint: &mut CompressedMint,
    signer: &pinocchio::pubkey::Pubkey,
) -> Result<(), ProgramError> {
    let metadata = get_metadata_extension_mut(
        compressed_mint,
        action.extension_index as usize,
        "update metadata authority",
        signer,
    )?;

    let new_authority = if action.new_authority.to_bytes() == [0u8; 32] {
        Pubkey::default()
    } else {
        action.new_authority
    };
    metadata.update_authority = new_authority;
    Ok(())
}

/// Only checks authority, the key is removed during data allocation.
#[profile]
pub fn process_remove_metadata_key_action(
    action: &ZRemoveMetadataKeyAction,
    compressed_mint: &mut CompressedMint,
    signer: &pinocchio::pubkey::Pubkey,
) -> Result<(), ProgramError> {
    let metadata = get_metadata_extension_mut(
        compressed_mint,
        action.extension_index as usize,
        "metadata key removal",
        signer,
    )?;
    if let Some(pos) = metadata
        .additional_metadata
        .iter()
        .position(|e| e.key.as_slice() == action.key)
    {
        metadata.additional_metadata.remove(pos);
    } else if action.idempotent != 1 {
        msg!("Metadata key not found");
        return Err(ErrorCode::MintActionMetadataKeyNotFound.into());
    }

    Ok(())
}

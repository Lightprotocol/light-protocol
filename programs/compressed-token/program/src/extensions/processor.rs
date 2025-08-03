use anchor_lang::prelude::ProgramError;
use light_ctoken_types::{hash_cache::HashCache, state::ZExtensionStructMut};
use light_hasher::{Hasher, Poseidon, Sha256};
use pinocchio::{msg, pubkey::Pubkey};

use crate::extensions::{token_metadata::create_output_token_metadata, ZExtensionInstructionData};

/// Set extensions state in output compressed account.
/// Compute extensions hash chain.
pub fn extensions_state_in_output_compressed_account(
    extensions: &[ZExtensionInstructionData<'_>],
    extension_in_output_compressed_account: &mut [ZExtensionStructMut<'_>],
    mint: light_compressed_account::Pubkey,
) -> Result<(), ProgramError> {
    if extension_in_output_compressed_account.len() != extensions.len() {
        return Err(ProgramError::InvalidInstructionData);
    }
    for (extension, output_extension) in extensions
        .iter()
        .zip(extension_in_output_compressed_account.iter_mut())
    {
        match (extension, output_extension) {
            /*(
                ZExtensionInstructionData::MetadataPointer(_extension),
                ZExtensionStructMut::MetadataPointer(_output_extension),
            ) => {
                create_output_metadata_pointer(extension, output_extension, start_offset)?;
            }*/
            (
                ZExtensionInstructionData::TokenMetadata(extension),
                ZExtensionStructMut::TokenMetadata(output_extension),
            ) => create_output_token_metadata(extension, output_extension, mint)?,
            _ => {
                return Err(ProgramError::InvalidInstructionData);
            }
        };
    }
    Ok(())
}

/// Creates extension hash chain for
pub fn create_extension_hash_chain(
    extensions: &[ZExtensionInstructionData<'_>],
    hashed_spl_mint: &Pubkey,
    hash_cache: &mut HashCache,
    version: u8,
) -> Result<[u8; 32], ProgramError> {
    let mut extension_hashchain = [0u8; 32];
    if version == 0 {
        for extension in extensions {
            let extension_hash = extension.hash::<Poseidon>(hashed_spl_mint, hash_cache)?;
            extension_hashchain =
                Poseidon::hashv(&[extension_hashchain.as_slice(), extension_hash.as_slice()])?;
        }
    } else if version == 1 {
        for extension in extensions {
            let extension_hash = extension.hash::<Sha256>(hashed_spl_mint, hash_cache)?;
            extension_hashchain =
                Sha256::hashv(&[extension_hashchain.as_slice(), extension_hash.as_slice()])?;
        }
    } else {
        msg!("Invalid version");
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(extension_hashchain)
}

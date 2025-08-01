use anchor_lang::prelude::ProgramError;
use light_ctoken_types::{context::TokenContext, state::ZExtensionStructMut};
use light_hasher::Hasher;
use pinocchio::pubkey::Pubkey;

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
pub fn create_extension_hash_chain<H: Hasher>(
    extensions: &[ZExtensionInstructionData<'_>],
    hashed_spl_mint: &Pubkey,
    context: &mut TokenContext,
) -> Result<[u8; 32], ProgramError> {
    let mut extension_hashchain = [0u8; 32];
    for extension in extensions {
        let extension_hash = extension.hash::<H>(hashed_spl_mint, context)?;
        extension_hashchain =
            H::hashv(&[extension_hashchain.as_slice(), extension_hash.as_slice()])?;
    }
    Ok(extension_hashchain)
}

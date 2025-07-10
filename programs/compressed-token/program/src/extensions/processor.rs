use anchor_lang::prelude::ProgramError;
use light_hasher::Hasher;

use crate::extensions::{
    state::ZExtensionStructMut, token_metadata::create_output_token_metadata,
    ZExtensionInstructionData,
};

// Applying extension(s) to compressed accounts.
pub fn process_create_extensions<'b, H: Hasher>(
    extensions: &[ZExtensionInstructionData<'b>],
    output_compressed_account: &mut [ZExtensionStructMut<'_>],
    mint: light_compressed_account::Pubkey,
) -> Result<[u8; 32], ProgramError> {
    let mut extension_hash_chain = [0u8; 32];
    if output_compressed_account.len() != extensions.len() {
        return Err(ProgramError::InvalidInstructionData);
    }
    for (extension, output_extension) in extensions.iter().zip(output_compressed_account.iter_mut())
    {
        let hash = match (extension, output_extension) {
            (
                ZExtensionInstructionData::MetadataPointer(_extension),
                ZExtensionStructMut::MetadataPointer(_output_extension),
            ) => {
                //create_output_metadata_pointer(extension, output_extension, start_offset)?;
                unimplemented!()
            }
            (
                ZExtensionInstructionData::TokenMetadata(extension),
                ZExtensionStructMut::TokenMetadata(output_extension),
            ) => create_output_token_metadata(extension, output_extension, mint)?,
            _ => {
                return Err(ProgramError::InvalidInstructionData);
            }
        };
        extension_hash_chain = H::hashv(&[extension_hash_chain.as_slice(), hash.as_slice()])?;
    }
    Ok(extension_hash_chain)
}

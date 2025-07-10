use anchor_lang::prelude::ProgramError;
use light_compressed_account::instruction_data::data::ZOutputCompressedAccountWithPackedContextMut;
use light_hasher::Hasher;

use crate::extensions::{
    metadata_pointer::create_output_metadata_pointer, token_metadata::create_output_token_metadata,
    ZExtensionInstructionData,
};

// Applying extension(s) to compressed accounts.
pub fn process_create_extensions<'a, H: Hasher>(
    extensions: &'a [ZExtensionInstructionData<'a>],
    output_compressed_account: &mut ZOutputCompressedAccountWithPackedContextMut<'a>,
    mut start_offset: usize,
) -> Result<[u8; 32], ProgramError> {
    let mut extension_hash_chain = [0u8; 32];
    for extension in extensions {
        let hash = match extension {
            ZExtensionInstructionData::MetadataPointer(extension) => {
                let (hash, new_start_offset) = create_output_metadata_pointer(
                    extension,
                    output_compressed_account,
                    start_offset,
                )?;
                start_offset = new_start_offset;
                hash
            }
            ZExtensionInstructionData::TokenMetadata(extension) => {
                let (hash, new_start_offset) = create_output_token_metadata(
                    extension,
                    output_compressed_account,
                    start_offset,
                )?;
                start_offset = new_start_offset;
                hash
            }
        };
        extension_hash_chain = H::hashv(&[extension_hash_chain.as_slice(), hash.as_slice()])?;
    }
    Ok(extension_hash_chain)
}

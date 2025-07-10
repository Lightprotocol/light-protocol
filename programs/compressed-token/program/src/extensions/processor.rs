use anchor_lang::prelude::ProgramError;
use light_compressed_account::instruction_data::data::ZOutputCompressedAccountWithPackedContextMut;

use crate::extensions::{
    metadata_pointer::create_output_metadata_pointer, token_metadata::create_output_token_metadata,
    ZExtensionInstructionData,
};

// Applying extension(s) to compressed accounts.
pub fn process_create_extensions<'a>(
    extensions: &'a [ZExtensionInstructionData<'a>],
    output_compressed_account: &mut ZOutputCompressedAccountWithPackedContextMut<'a>,
    mut start_offset: usize,
) -> Result<(), ProgramError> {
    for extension in extensions {
        match extension {
            ZExtensionInstructionData::MetadataPointer(extension) => {
                start_offset = create_output_metadata_pointer(extension, output_compressed_account, start_offset)?;
            }
            ZExtensionInstructionData::TokenMetadata(extension) => {
                start_offset = create_output_token_metadata(extension, output_compressed_account, start_offset)?;
            }
        }
    }
    Ok(())
}

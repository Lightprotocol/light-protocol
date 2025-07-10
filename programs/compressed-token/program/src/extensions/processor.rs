use anchor_lang::prelude::ProgramError;
use light_compressed_account::instruction_data::with_readonly::ZInstructionDataInvokeCpiWithReadOnlyMut;

use crate::extensions::{
    metadata_pointer::initialize_metadata_pointer, token_metadata::initialize_token_metadata,
    ZExtensionInstructionData,
};

// Applying extension(s) to compressed accounts.
pub fn process_create_extensions<'a>(
    extensions: &'a [ZExtensionInstructionData<'a>],
    cpi_data: &mut ZInstructionDataInvokeCpiWithReadOnlyMut<'a>,
    mut start_offset: usize,
) -> Result<(), ProgramError> {
    for extension in extensions {
        match extension {
            ZExtensionInstructionData::MetadataPointer(extension) => {
                start_offset = initialize_metadata_pointer(extension, cpi_data, start_offset)?;
            }
            ZExtensionInstructionData::TokenMetadata(extension) => {
                start_offset = initialize_token_metadata(extension, cpi_data, start_offset)?;
            }
        }
    }
    Ok(())
}

use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_ctoken_types::state::ZExtensionStructMut;
use light_program_profiler::profile;

use crate::extensions::{token_metadata::create_output_token_metadata, ZExtensionInstructionData};

/// Set extensions state in output compressed account.
/// Compute extensions hash chain.
#[inline(always)]
#[profile]
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
            (
                ZExtensionInstructionData::TokenMetadata(extension),
                ZExtensionStructMut::TokenMetadata(output_extension),
            ) => create_output_token_metadata(extension, output_extension, mint)?,
            _ => {
                return Err(ErrorCode::InvalidExtensionType.into());
            }
        };
    }
    Ok(())
}

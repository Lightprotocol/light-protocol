use anchor_lang::prelude::ProgramError;
use light_compressed_account::Pubkey;
use light_ctoken_types::{
    instructions::extensions::token_metadata::ZTokenMetadataInstructionData,
    state::ZTokenMetadataMut,
};
use light_program_profiler::profile;

#[inline(always)]
#[profile]
pub fn create_output_token_metadata(
    token_metadata_data: &ZTokenMetadataInstructionData<'_>,
    token_metadata: &mut ZTokenMetadataMut<'_>,
    mint: Pubkey,
) -> Result<(), ProgramError> {
    // We assume token_metadata is allocated correctly.
    // We cannot fail on None since if we remove the update authority we allocate None.
    if let Some(authority) = token_metadata_data.update_authority.as_deref() {
        token_metadata.update_authority = *authority;
    }

    // Only copy field data if allocated size exactly matches instruction data size
    // If sizes don't match, there must be an update action that will populate this field
    if token_metadata.name.len() == token_metadata_data.name.len() {
        // Sizes match: no action will update this field, copy instruction data directly
        token_metadata
            .name
            .copy_from_slice(token_metadata_data.name);
    }
    // Size mismatch: an action will update this field, leave uninitialized

    if token_metadata.symbol.len() == token_metadata_data.symbol.len() {
        // Sizes match: no action will update this field, copy instruction data directly
        token_metadata
            .symbol
            .copy_from_slice(token_metadata_data.symbol);
    }
    // Size mismatch: an action will update this field, leave uninitialized

    if token_metadata.uri.len() == token_metadata_data.uri.len() {
        // Sizes match: no action will update this field, copy instruction data directly
        token_metadata.uri.copy_from_slice(token_metadata_data.uri);
    }
    // Size mismatch: an action will update this field, leave uninitialized

    // Set mint
    token_metadata.mint = mint;

    // Set additional metadata if provided
    if let Some(ref additional_metadata) = token_metadata_data.additional_metadata {
        for (i, item) in additional_metadata.iter().enumerate() {
            // Only copy if sizes match exactly - if sizes don't match, there must be an update action
            if token_metadata.additional_metadata[i].key.len() == item.key.len() {
                // Sizes match: no action will update this key, copy instruction data directly
                token_metadata.additional_metadata[i]
                    .key
                    .copy_from_slice(item.key);
            }
            // Size mismatch: an action will update this key, leave uninitialized

            if token_metadata.additional_metadata[i].value.len() == item.value.len() {
                // Sizes match: no action will update this value, copy instruction data directly
                token_metadata.additional_metadata[i]
                    .value
                    .copy_from_slice(item.value);
            }
            // Size mismatch: an action will update this value, leave uninitialized
        }
    }

    Ok(())
}

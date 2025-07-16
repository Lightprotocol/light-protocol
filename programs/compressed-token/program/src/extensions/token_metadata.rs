use anchor_lang::prelude::ProgramError;
use light_compressed_account::Pubkey;
use light_ctoken_types::instructions::extensions::token_metadata::{
    ZTokenMetadataInstructionData, ZTokenMetadataMut,
};
use light_hasher::DataHasher;

pub fn create_output_token_metadata(
    token_metadata_data: &ZTokenMetadataInstructionData<'_>,
    token_metadata: &mut ZTokenMetadataMut<'_>,
    mint: Pubkey,
) -> Result<[u8; 32], ProgramError> {
    if let Some(ref mut authority) = token_metadata.update_authority {
        **authority = *token_metadata_data
            .update_authority
            .ok_or(ProgramError::InvalidInstructionData)?;
    }
    token_metadata
        .metadata
        .name
        .copy_from_slice(token_metadata_data.metadata.name);
    token_metadata
        .metadata
        .symbol
        .copy_from_slice(token_metadata_data.metadata.symbol);
    token_metadata
        .metadata
        .uri
        .copy_from_slice(token_metadata_data.metadata.uri);

    // Set mint
    *token_metadata.mint = mint;

    // Set version
    *token_metadata.version = token_metadata_data.version;

    // Set additional metadata if provided
    if let Some(ref additional_metadata) = token_metadata_data.additional_metadata {
        for (i, item) in additional_metadata.iter().enumerate() {
            token_metadata.additional_metadata[i]
                .key
                .copy_from_slice(item.key);
            token_metadata.additional_metadata[i]
                .value
                .copy_from_slice(item.value);
        }
    }

    // Use the zero-copy mut struct for hashing
    let hash = token_metadata
        .hash::<light_hasher::Poseidon>()
        .map_err(|_| ProgramError::InvalidAccountData)?;

    Ok(hash)
}

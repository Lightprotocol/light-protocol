use anchor_lang::{prelude::ProgramError, solana_program::msg};
use light_compressed_account::Pubkey;
use light_ctoken_types::{
    instructions::extensions::token_metadata::ZTokenMetadataInstructionData,
    state::ZTokenMetadataMut,
};

use crate::mint_action::update_metadata::safe_copy_metadata_value;

pub fn create_output_token_metadata(
    token_metadata_data: &ZTokenMetadataInstructionData<'_>,
    token_metadata: &mut ZTokenMetadataMut<'_>,
    mint: Pubkey,
) -> Result<(), ProgramError> {
    msg!("create_output_token_metadata 1");
    if let Some(ref mut authority) = token_metadata.update_authority {
        **authority = *token_metadata_data
            .update_authority
            .ok_or(ProgramError::InvalidInstructionData)?;
    }
    msg!(
        "create_output_token_metadata 1 allocated {}, data: {}",
        token_metadata.metadata.name.len(),
        token_metadata_data.metadata.name.len()
    );
    safe_copy_metadata_value(
        token_metadata.metadata.name,
        token_metadata_data.metadata.name,
        "name",
    )?;
    msg!("create_output_token_metadata 2");
    safe_copy_metadata_value(
        token_metadata.metadata.symbol,
        token_metadata_data.metadata.symbol,
        "symbol",
    )?;
    msg!("create_output_token_metadata 3");
    safe_copy_metadata_value(
        token_metadata.metadata.uri,
        token_metadata_data.metadata.uri,
        "uri",
    )?;

    // Set mint
    *token_metadata.mint = mint;

    // Set version
    *token_metadata.version = token_metadata_data.version;

    // Set additional metadata if provided
    if let Some(ref additional_metadata) = token_metadata_data.additional_metadata {
        for (i, item) in additional_metadata.iter().enumerate() {
            msg!("additional_metadata i {}", i);
            safe_copy_metadata_value(
                token_metadata.additional_metadata[i].key,
                item.key,
                &format!("additional_metadata[{}].key", i),
            )?;
            safe_copy_metadata_value(
                token_metadata.additional_metadata[i].value,
                item.value,
                &format!("additional_metadata[{}].value", i),
            )?;
        }
    }

    Ok(())
}

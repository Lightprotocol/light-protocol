use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::instruction_data::with_readonly::ZInAccountMut;
use light_hasher::{Hasher, Poseidon};

use crate::{
    constants::COMPRESSED_MINT_DISCRIMINATOR, extensions::processor::create_extension_hash_chain,
};
use light_ctoken_types::{
    context::TokenContext,
    instructions::create_compressed_mint::ZUpdateCompressedMintInstructionData,
    state::CompressedMint,
};

/// Creates and validates an input compressed mint account.
/// This function follows the same pattern as create_output_compressed_mint_account
/// but processes existing compressed mint accounts as inputs.
///
/// Steps:
/// 1. Set InAccount fields (discriminator, merkle context, address)
/// 2. Validate the compressed mint data matches expected values
/// 3. Compute data hash using TokenContext for caching
/// 4. Return validated CompressedMint data for output processing
pub fn create_input_compressed_mint_account(
    input_compressed_account: &mut ZInAccountMut,
    context: &mut TokenContext,
    compressed_mint_inputs: &ZUpdateCompressedMintInstructionData,
    hashed_mint_authority: &[u8; 32],
) -> Result<(), ProgramError> {
    // 2. Extract and validate compressed mint data
    let compressed_mint_input = &compressed_mint_inputs.mint;
    //TODO: extract into function and test vs output hash creation
    // 1. Compute data hash using TokenContext for caching
    let data_hash = {
        let hashed_spl_mint = context
            .get_or_hash_mint(&compressed_mint_input.spl_mint.into())
            .map_err(ProgramError::from)?;
        let mut supply_bytes = [0u8; 32];
        supply_bytes[24..]
            .copy_from_slice(compressed_mint_input.supply.get().to_be_bytes().as_slice());

        let hashed_freeze_authority = compressed_mint_input
            .freeze_authority
            .as_ref()
            .map(|freeze_authority| context.get_or_hash_pubkey(&(**freeze_authority).to_bytes()));

        // Compute the data hash using the CompressedMint hash function
        let data_hash = CompressedMint::hash_with_hashed_values(
            &hashed_spl_mint,
            &supply_bytes,
            compressed_mint_input.decimals,
            compressed_mint_input.is_decompressed(),
            &Some(hashed_mint_authority), // pre-hashed mint_authority from signer
            &hashed_freeze_authority.as_ref(),
            compressed_mint_input.version,
        )
        .map_err(|_| ProgramError::InvalidAccountData)?;

        let extension_hashchain =
            compressed_mint_inputs
                .mint
                .extensions
                .as_ref()
                .map(|extensions| {
                    create_extension_hash_chain::<Poseidon>(extensions, &hashed_spl_mint, context)
                });
        if let Some(extension_hashchain) = extension_hashchain {
            Poseidon::hashv(&[data_hash.as_slice(), extension_hashchain?.as_slice()])?
        } else {
            data_hash
        }
    };

    // 2. Set InAccount fields

    input_compressed_account.set(
        COMPRESSED_MINT_DISCRIMINATOR,
        data_hash,
        &compressed_mint_inputs.merkle_context,
        *compressed_mint_inputs.root_index,
        0,
        Some(compressed_mint_inputs.address.as_ref()),
    )?;

    Ok(())
}

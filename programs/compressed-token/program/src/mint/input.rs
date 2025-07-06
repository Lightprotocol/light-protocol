use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::instruction_data::with_readonly::ZInAccountMut;

use crate::{
    constants::COMPRESSED_MINT_DISCRIMINATOR, mint::state::CompressedMint,
    mint_to_compressed::instructions::ZCompressedMintInputs, shared::context::TokenContext,
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
    compressed_mint_inputs: &ZCompressedMintInputs,
    hashed_mint_authority: &[u8; 32],
) -> Result<(), ProgramError> {
    // 1. Set InAccount fields
    {
        input_compressed_account.discriminator = COMPRESSED_MINT_DISCRIMINATOR;
        // Set merkle context fields manually due to mutability constraints
        input_compressed_account
            .merkle_context
            .merkle_tree_pubkey_index = compressed_mint_inputs
            .merkle_context
            .merkle_tree_pubkey_index;
        input_compressed_account.merkle_context.queue_pubkey_index =
            compressed_mint_inputs.merkle_context.queue_pubkey_index;
        input_compressed_account
            .merkle_context
            .leaf_index
            .set(compressed_mint_inputs.merkle_context.leaf_index.get());
        input_compressed_account.merkle_context.prove_by_index =
            compressed_mint_inputs.merkle_context.prove_by_index;
        input_compressed_account
            .root_index
            .set(compressed_mint_inputs.root_index.get());

        input_compressed_account
            .address
            .as_mut()
            .ok_or(ProgramError::InvalidAccountData)?
            .copy_from_slice(compressed_mint_inputs.address.as_ref());
    }

    // 2. Extract and validate compressed mint data
    let compressed_mint_input = &compressed_mint_inputs.compressed_mint_input;

    // // Create the expected CompressedMint structure for validation
    // let compressed_mint = CompressedMint {
    //     spl_mint: compressed_mint_input.spl_mint,
    //     supply: compressed_mint_input.supply.get(),
    //     decimals: compressed_mint_input.decimals,
    //     is_decompressed: compressed_mint_input.is_decompressed(),
    //     mint_authority: None, // Will be set based on validation
    //     freeze_authority: if compressed_mint_input.freeze_authority_is_set() {
    //         Some(compressed_mint_input.freeze_authority)
    //     } else {
    //         None
    //     },
    //     num_extensions: compressed_mint_input.num_extensions,
    // };

    // 3. Compute data hash using TokenContext for caching
    {
        let hashed_spl_mint = context.get_or_hash_mint(compressed_mint_input.spl_mint.into())?;
        let mut supply_bytes = [0u8; 32];
        supply_bytes[24..]
            .copy_from_slice(compressed_mint_input.supply.get().to_be_bytes().as_slice());

        let hashed_freeze_authority = if compressed_mint_input.freeze_authority_is_set() {
            Some(context.get_or_hash_pubkey(&compressed_mint_input.freeze_authority.into()))
        } else {
            None
        };

        // Compute the data hash using the CompressedMint hash function
        let data_hash = CompressedMint::hash_with_hashed_values(
            &hashed_spl_mint,
            &supply_bytes,
            compressed_mint_input.decimals,
            compressed_mint_input.is_decompressed(),
            &Some(hashed_mint_authority), // pre-hashed mint_authority from signer
            &hashed_freeze_authority.as_ref(),
            compressed_mint_input.num_extensions,
        )
        .map_err(|_| ProgramError::InvalidAccountData)?;

        input_compressed_account.data_hash = data_hash;
    }

    Ok(())
}

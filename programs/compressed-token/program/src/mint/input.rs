use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::instruction_data::with_readonly::ZInAccountMut;
use light_hasher::{Hasher, Poseidon};

use crate::{
    constants::COMPRESSED_MINT_DISCRIMINATOR,
    mint::{instructions::ZUpdateCompressedMintInstructionData, state::CompressedMint},
    shared::context::TokenContext,
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
    let compressed_mint_input = &compressed_mint_inputs.mint;

    // 3. Compute data hash using TokenContext for caching
    {
        let hashed_spl_mint = context.get_or_hash_mint(&compressed_mint_input.spl_mint.into())?;
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

        let extension_hashchain = if let Some(extensions) =
            compressed_mint_inputs.mint.extensions.as_ref()
        {
            let mut extension_hashchain = [0u8; 32];
            for extension in extensions {
                let extension_hash = extension.hash::<Poseidon>(&hashed_spl_mint, context)?;
                extension_hashchain =
                    Poseidon::hashv(&[extension_hashchain.as_slice(), extension_hash.as_slice()])?;
            }
            Some(extension_hashchain)
        } else {
            None
        };
        input_compressed_account.data_hash = if let Some(extension_hashchain) = extension_hashchain
        {
            Poseidon::hashv(&[data_hash.as_slice(), extension_hashchain.as_slice()])?
        } else {
            data_hash
        };
    }

    Ok(())
}

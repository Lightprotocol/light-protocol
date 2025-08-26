use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::instruction_data::with_readonly::ZInAccountMut;
use light_ctoken_types::{
    hash_cache::HashCache, instructions::mint_action::ZMintActionCompressedInstructionData,
    state::compute_compressed_mint_hash_from_values,
};
use light_sdk::instruction::PackedMerkleContext;
use zerocopy::IntoBytes;

use crate::constants::COMPRESSED_MINT_DISCRIMINATOR;
/// Creates and validates an input compressed mint account.
/// This function follows the same pattern as create_output_compressed_mint_account
/// but processes existing compressed mint accounts as inputs.
///
/// Steps:
/// 1. Set InAccount fields (discriminator, merkle hash_cache, address)
/// 2. Validate the compressed mint data matches expected values
/// 3. Compute data hash using HashCache for caching
/// 4. Return validated CompressedMint data for output processing
pub fn create_input_compressed_mint_account(
    input_compressed_account: &mut ZInAccountMut,
    hash_cache: &mut HashCache,
    mint_instruction_data: &ZMintActionCompressedInstructionData,
    merkle_context: PackedMerkleContext,
) -> Result<(), ProgramError> {
    let mint = &mint_instruction_data.mint;

    // 1. Compute data hash using unified function
    let data_hash = compute_compressed_mint_hash_from_values(
        mint.spl_mint,
        mint.supply.as_bytes(),
        mint.decimals,
        mint.is_decompressed(),
        mint.mint_authority.map(|x| *x),
        mint.freeze_authority.map(|x| *x),
        mint.version,
        mint_instruction_data.mint.extensions.as_deref(),
        hash_cache,
    )
    .map_err(ProgramError::from)?;

    // 2. Set InAccount fields
    input_compressed_account.set(
        COMPRESSED_MINT_DISCRIMINATOR,
        data_hash,
        &merkle_context,
        mint_instruction_data.root_index,
        0,
        Some(mint_instruction_data.compressed_address.as_ref()),
    )?;

    Ok(())
}

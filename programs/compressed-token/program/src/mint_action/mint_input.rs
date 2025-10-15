use anchor_lang::solana_program::program_error::ProgramError;
use borsh::BorshSerialize;
use light_compressed_account::instruction_data::with_readonly::ZInAccountMut;
use light_ctoken_types::{
    instructions::mint_action::ZMintActionCompressedInstructionData, state::CompressedMint,
};
use light_hasher::{sha256::Sha256BE, Hasher};
use light_program_profiler::profile;
use light_sdk::instruction::PackedMerkleContext;

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
#[profile]
pub fn create_input_compressed_mint_account(
    input_compressed_account: &mut ZInAccountMut,
    mint_instruction_data: &ZMintActionCompressedInstructionData,
    merkle_context: PackedMerkleContext,
) -> Result<CompressedMint, ProgramError> {
    let compressed_mint = CompressedMint::try_from(&mint_instruction_data.mint)?;
    let bytes = compressed_mint
        .try_to_vec()
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;
    let input_data_hash = Sha256BE::hash(bytes.as_slice())?;

    // 2. Set InAccount fields
    input_compressed_account.set(
        COMPRESSED_MINT_DISCRIMINATOR,
        input_data_hash,
        &merkle_context,
        mint_instruction_data.root_index,
        0,
        Some(mint_instruction_data.compressed_address.as_ref()),
    )?;

    Ok(compressed_mint)
}

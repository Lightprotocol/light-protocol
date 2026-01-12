use anchor_lang::solana_program::program_error::ProgramError;
use borsh::BorshSerialize;
use light_compressed_account::instruction_data::with_readonly::ZInAccountMut;
use light_token_interface::state::CompressedMint;
use light_hasher::{sha256::Sha256BE, Hasher};
use light_program_profiler::profile;
use light_sdk::instruction::PackedMerkleContext;
use light_zero_copy::U16;

use crate::{
    compressed_token::mint_action::accounts::AccountsConfig,
    constants::COMPRESSED_MINT_DISCRIMINATOR,
};

/// Creates and validates an input compressed mint account.
/// This function follows the same pattern as create_output_compressed_mint_account
/// but processes existing compressed mint accounts as inputs.
///
/// Steps:
/// 1. Determine if CMint is decompressed (use zero values) or data from instruction
/// 2. Set InAccount fields (discriminator, merkle hash, address)
#[profile]
pub fn create_input_compressed_mint_account(
    input_compressed_account: &mut ZInAccountMut,
    root_index: U16,
    merkle_context: PackedMerkleContext,
    accounts_config: &AccountsConfig,
    compressed_mint: &CompressedMint,
) -> Result<(), ProgramError> {
    // When CMint was decompressed (input state BEFORE actions), use zero values
    let (discriminator, input_data_hash) = if accounts_config.cmint_decompressed {
        ([0u8; 8], [0u8; 32])
    } else {
        // Data from instruction - compute hash
        let bytes = compressed_mint
            .try_to_vec()
            .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;
        (
            COMPRESSED_MINT_DISCRIMINATOR,
            Sha256BE::hash(bytes.as_slice())?,
        )
    };

    // Set InAccount fields
    input_compressed_account.set(
        discriminator,
        input_data_hash,
        &merkle_context,
        root_index,
        0,
        Some(compressed_mint.metadata.compressed_address.as_ref()),
    )?;

    Ok(())
}

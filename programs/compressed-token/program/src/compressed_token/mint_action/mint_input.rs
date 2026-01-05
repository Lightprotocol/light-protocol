use anchor_lang::solana_program::program_error::ProgramError;
use borsh::BorshSerialize;
use light_compressed_account::instruction_data::with_readonly::ZInAccountMut;
use light_ctoken_interface::{
    instructions::mint_action::ZMintActionCompressedInstructionData, state::CompressedMint,
};
use light_hasher::{sha256::Sha256BE, Hasher};
use light_program_profiler::profile;
use light_sdk::instruction::PackedMerkleContext;

use crate::{compressed_token::mint_action::accounts::AccountsConfig, constants::COMPRESSED_MINT_DISCRIMINATOR};

/// Creates and validates an input compressed mint account.
/// This function follows the same pattern as create_output_compressed_mint_account
/// but processes existing compressed mint accounts as inputs.
///
/// Steps:
/// 1. Determine if CMint is source of truth (use zero values) or data from instruction
/// 2. Set InAccount fields (discriminator, merkle hash, address)
#[profile]
pub fn create_input_compressed_mint_account(
    input_compressed_account: &mut ZInAccountMut,
    mint_instruction_data: &ZMintActionCompressedInstructionData,
    merkle_context: PackedMerkleContext,
    accounts_config: &AccountsConfig,
) -> Result<(), ProgramError> {
    // When CMint was source of truth (input state BEFORE actions), use zero sentinel values
    // Use cmint_decompressed directly, not cmint_is_source_of_truth(), because:
    // - cmint_is_source_of_truth() tells us the OUTPUT state (after actions)
    // - cmint_decompressed tells us the INPUT state (before actions)
    // For CompressAndCloseCMint: input has zero values (was decompressed), output has real data
    let (discriminator, input_data_hash) = if accounts_config.cmint_decompressed {
        ([0u8; 8], [0u8; 32])
    } else {
        // Data from instruction - compute hash
        let mint_data = mint_instruction_data
            .mint
            .as_ref()
            .ok_or(ProgramError::InvalidInstructionData)?;
        // Return it so that we dont deserialize it twice.
        let compressed_mint = CompressedMint::try_from(mint_data)?;
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
        mint_instruction_data.root_index,
        0,
        Some(mint_instruction_data.compressed_address.as_ref()),
    )?;

    Ok(())
}

use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::{
    instruction_data::data::ZOutputCompressedAccountWithPackedContextMut, Pubkey,
};
use light_ctoken_types::{
    context::TokenContext,
    instructions::{
        extensions::ZExtensionInstructionData, mint_to_compressed::ZCompressedMintInputs,
    },
    state::{CompressedMint, CompressedMintConfig},
};
use light_hasher::Poseidon;
use light_zero_copy::ZeroCopyNew;
use zerocopy::little_endian::U64;

use crate::{
    constants::COMPRESSED_MINT_DISCRIMINATOR,
    extensions::processor::{
        create_extension_hash_chain, extensions_state_in_output_compressed_account,
    },
};

/// Input struct for create_output_compressed_mint_account function
/// Consolidates all parameters needed to create an output compressed mint account
pub struct CreateOutputCompressedMintAccountInputs<'a, 'b> {
    /// The mint PDA address
    pub mint_pda: Pubkey,
    /// Number of decimal places
    pub decimals: u8,
    /// Optional freeze authority
    pub freeze_authority: Option<Pubkey>,
    /// Optional mint authority
    pub mint_authority: Option<Pubkey>,
    /// Token supply
    pub supply: U64,
    /// Mint configuration for zero-copy
    pub mint_config: CompressedMintConfig,
    /// Compressed account address
    pub compressed_account_address: [u8; 32],
    /// Merkle tree index
    pub merkle_tree_index: u8,
    /// Version for upgradability
    pub version: u8,
    /// Whether the mint is decompressed
    pub is_decompressed: bool,
    pub compressed_mint_input: ZCompressedMintInputs<'a>,
    /// Optional extensions
    pub extensions: Option<&'a [ZExtensionInstructionData<'b>]>,
}

// TODO: pass in struct
#[allow(clippy::too_many_arguments)]
pub fn create_output_compressed_mint_account(
    output_compressed_account: &mut ZOutputCompressedAccountWithPackedContextMut<'_>,
    mint_pda: Pubkey,
    decimals: u8,
    freeze_authority: Option<Pubkey>,
    mint_authority: Option<Pubkey>,
    supply: U64,
    mint_config: CompressedMintConfig,
    compressed_account_address: [u8; 32],
    merkle_tree_index: u8,
    version: u8,
    is_decompressed: bool,
    extensions: Option<&[ZExtensionInstructionData<'_>]>,
    context: &mut TokenContext,
) -> Result<(), ProgramError> {
    // 1. Set CompressedMint account data & compute hash
    let data_hash = {
        let compressed_account_data = output_compressed_account
            .compressed_account
            .data
            .as_mut()
            .ok_or(ProgramError::InvalidAccountData)?;

        let (mut compressed_mint, _) =
            CompressedMint::new_zero_copy(compressed_account_data.data, mint_config)
                .map_err(ProgramError::from)?;
        compressed_mint.set(
            version,
            mint_pda,
            supply,
            decimals,
            is_decompressed,
            mint_authority,
            freeze_authority,
        )?;

        // Process extensions if provided and populate the zero-copy extension data
        let extension_hash = if let Some(extensions) = extensions.as_ref() {
            let z_extensions = compressed_mint
                .extensions
                .as_mut()
                .ok_or(ProgramError::AccountAlreadyInitialized)?;

            extensions_state_in_output_compressed_account(
                extensions,
                z_extensions.as_mut_slice(),
                mint_pda,
            )?;
            let hashed_spl_mint = context.get_or_hash_mint(&mint_pda.into())?;

            Some(create_extension_hash_chain::<Poseidon>(
                extensions,
                &hashed_spl_mint,
                context,
            )?)
        } else {
            None
        };
        // Compute final hash with extensions
        compressed_mint
            .hash(extension_hash, context)
            .map_err(|_| ProgramError::InvalidAccountData)?
    };

    // 2. Set output compressed account
    output_compressed_account.set(
        crate::LIGHT_CPI_SIGNER.program_id.into(),
        0,
        Some(compressed_account_address),
        merkle_tree_index,
        COMPRESSED_MINT_DISCRIMINATOR,
        data_hash,
    )?;

    Ok(())
}

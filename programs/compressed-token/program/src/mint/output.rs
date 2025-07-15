use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::{
    instruction_data::data::ZOutputCompressedAccountWithPackedContextMut, Pubkey,
};
use light_zero_copy::ZeroCopyNew;
use zerocopy::little_endian::U64;

use crate::{
    constants::COMPRESSED_MINT_DISCRIMINATOR,
    extensions::ZExtensionInstructionData,
    mint::state::{CompressedMint, CompressedMintConfig},
};
// TODO: pass in struct
#[allow(clippy::too_many_arguments)]
pub fn create_output_compressed_mint_account(
    output_compressed_account: &mut ZOutputCompressedAccountWithPackedContextMut<'_>,
    mint_pda: Pubkey,
    decimals: u8,
    freeze_authority: Option<Pubkey>,
    mint_authority: Option<Pubkey>,
    supply: U64,
    program_id: &Pubkey,
    mint_config: CompressedMintConfig,
    compressed_account_address: [u8; 32],
    merkle_tree_index: u8,
    version: u8,
    is_decompressed: bool,
    extensions: Option<&[ZExtensionInstructionData<'_>]>,
) -> Result<(), ProgramError> {
    // 1. Create output compressed account
    {
        // TODO: create helper to assign output_compressed_account
        output_compressed_account.compressed_account.owner = *program_id;

        if let Some(address) = output_compressed_account
            .compressed_account
            .address
            .as_deref_mut()
        {
            *address = compressed_account_address;
        } else {
            panic!("Compressed account address is required");
        }
        *output_compressed_account.merkle_tree_index = merkle_tree_index;
    }
    // 2. Create CompressedMint account data & compute hash

    // TODO: create helper to assign compressed account data
    let compressed_account_data = output_compressed_account
        .compressed_account
        .data
        .as_mut()
        .ok_or(ProgramError::InvalidAccountData)?;

    compressed_account_data.discriminator = COMPRESSED_MINT_DISCRIMINATOR;

    let (mut compressed_mint, _) =
        CompressedMint::new_zero_copy(compressed_account_data.data, mint_config)
            .map_err(ProgramError::from)?;
    compressed_mint.spl_mint = mint_pda;
    compressed_mint.decimals = decimals;
    compressed_mint.supply = supply;
    if let Some(freeze_auth) = freeze_authority {
        if let Some(z_freeze_authority) = compressed_mint.freeze_authority.as_deref_mut() {
            *z_freeze_authority = freeze_auth;
        }
    }
    if let Some(mint_auth) = mint_authority {
        if let Some(z_mint_authority) = compressed_mint.mint_authority.as_deref_mut() {
            *z_mint_authority = mint_auth;
        }
    }
    compressed_mint.version = version;
    compressed_mint.is_decompressed = if is_decompressed { 1 } else { 0 };

    // Process extensions if provided and populate the zero-copy extension data
    if let Some(extensions) = extensions.as_ref() {
        if let Some(z_extensions) = compressed_mint.extensions.as_mut() {
            // Now we can directly populate the extension data using the updated process_create_extensions
            use light_hasher::Poseidon;

            use crate::extensions::processor::process_create_extensions;
            let extension_hash = process_create_extensions::<Poseidon>(
                extensions,
                z_extensions.as_mut_slice(),
                mint_pda,
            )?;
            // Compute final hash with extensions
            *compressed_account_data.data_hash = compressed_mint
                .hash(Some(extension_hash.as_slice()))
                .map_err(|_| ProgramError::InvalidAccountData)?;
        };
    } else {
        *compressed_account_data.data_hash = compressed_mint
            .hash(None)
            .map_err(|_| ProgramError::InvalidAccountData)?;
    };

    Ok(())
}

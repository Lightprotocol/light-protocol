use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::{
    instruction_data::data::ZOutputCompressedAccountWithPackedContextMut, Pubkey,
};

use light_hasher::Poseidon;
use light_zero_copy::ZeroCopyNew;
use zerocopy::little_endian::U64;

use crate::{
    constants::COMPRESSED_MINT_DISCRIMINATOR,
    extensions::{processor::process_create_extensions, ZExtensionInstructionData},
    mint::state::{CompressedMint, CompressedMintConfig},
};
// TODO: pass in struct
#[allow(clippy::too_many_arguments)]
pub fn create_output_compressed_mint_account<'a>(
    output_compressed_account: &mut ZOutputCompressedAccountWithPackedContextMut<'a>,
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
    extensions: Option<&'a [ZExtensionInstructionData<'a>]>,
    base_mint_len: usize,
) -> Result<(), ProgramError> {
    // 3. Create output compressed account
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
    // 4. Create CompressedMint account data & compute hash

    // 5. Process extensions if provided first
    let extension_hash = if let Some(extensions) = extensions {
        Some(process_create_extensions::<Poseidon>(
            extensions,
            output_compressed_account,
            base_mint_len,
        )?)
    } else {
        None
    };

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

    // Compute final hash with extensions
    *compressed_account_data.data_hash = compressed_mint
        .hash(extension_hash.as_ref().map(|h| h.as_slice()))
        .map_err(|_| ProgramError::InvalidAccountData)?;

    Ok(())
}

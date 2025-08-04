use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::instruction_data::with_readonly::ZInAccountMut;
use light_ctoken_types::{
    hash_cache::HashCache, instructions::mint_actions::ZMintActionCompressedInstructionData,
    state::CompressedMint, CTokenError,
};
use light_hasher::{Hasher, Poseidon, Sha256};
use light_sdk::instruction::PackedMerkleContext;

use crate::{
    constants::COMPRESSED_MINT_DISCRIMINATOR, extensions::processor::create_extension_hash_chain,
};

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
    // 1. Compute data hash using HashCache for caching
    let data_hash = {
        let hashed_spl_mint = hash_cache
            .get_or_hash_mint(&mint.spl_mint.into())
            .map_err(ProgramError::from)?;
        let mut supply_bytes = [0u8; 32];
        supply_bytes[24..].copy_from_slice(mint.supply.get().to_be_bytes().as_slice());

        let hashed_mint_authority = mint
            .mint_authority
            .map(|pubkey| hash_cache.get_or_hash_pubkey(&pubkey.to_bytes()));
        let hashed_freeze_authority = mint
            .freeze_authority
            .map(|pubkey| hash_cache.get_or_hash_pubkey(&pubkey.to_bytes()));

        // Compute the data hash using the CompressedMint hash function
        let data_hash = CompressedMint::hash_with_hashed_values(
            &hashed_spl_mint,
            &supply_bytes,
            mint.decimals,
            mint.is_decompressed(),
            &hashed_mint_authority.as_ref(),
            &hashed_freeze_authority.as_ref(),
            mint.version,
        )?;

        let extension_hashchain =
            mint_instruction_data
                .mint
                .extensions
                .as_ref()
                .map(|extensions| {
                    create_extension_hash_chain(
                        extensions,
                        &hashed_spl_mint,
                        hash_cache,
                        mint.version,
                    )
                });
        if let Some(extension_hashchain) = extension_hashchain {
            if mint.version == 0 {
                Poseidon::hashv(&[data_hash.as_slice(), extension_hashchain?.as_slice()])?
            } else if mint.version == 1 {
                let mut hash =
                    Sha256::hashv(&[data_hash.as_slice(), extension_hashchain?.as_slice()])?;
                hash[0] = 0;
                hash
            } else {
                return Err(ProgramError::from(CTokenError::InvalidTokenDataVersion));
            }
        } else if mint.version == 0 {
            data_hash
        } else if mint.version == 1 {
            let mut hash = data_hash;
            hash[0] = 0;
            hash
        } else {
            return Err(ProgramError::from(CTokenError::InvalidTokenDataVersion));
        }
    };

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

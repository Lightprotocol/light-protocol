use anchor_lang::solana_program::program_error::ProgramError;
use borsh::BorshSerialize;
use light_compressed_account::instruction_data::with_readonly::ZInAccountMut;
use light_ctoken_types::{
    instructions::{
        extensions::ZExtensionInstructionData, mint_action::ZMintActionCompressedInstructionData,
    },
    state::{
        AdditionalMetadata, BaseCompressedMint, BaseCompressedMintConfig, CompressedMint,
        CompressedMintConfig, ExtensionStruct, Metadata, TokenMetadata,
    },
    CTokenError,
};
use light_hasher::{sha256::Sha256BE, Hasher};
use light_profiler::profile;
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
) -> Result<(), ProgramError> {
    // Build extensions if present
    let mut extensions_vec = None;
    if let Some(extensions) = mint_instruction_data.mint.extensions.as_deref() {
        let mut ext_structs = Vec::new();
        for ext in extensions {
            match ext {
                ZExtensionInstructionData::TokenMetadata(metadata_ix) => {
                    let metadata = Metadata {
                        name: metadata_ix.metadata.name.to_vec(),
                        symbol: metadata_ix.metadata.symbol.to_vec(),
                        uri: metadata_ix.metadata.uri.to_vec(),
                    };

                    let additional_metadata = metadata_ix
                        .additional_metadata
                        .as_ref()
                        .map(|v| {
                            v.iter()
                                .map(|e| AdditionalMetadata {
                                    key: e.key.to_vec(),
                                    value: e.value.to_vec(),
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    let token_metadata = TokenMetadata {
                        update_authority: metadata_ix.update_authority.map(|x| *x),
                        mint: mint_instruction_data.mint.base.spl_mint,
                        metadata,
                        additional_metadata,
                        version: metadata_ix.version,
                    };

                    ext_structs.push(ExtensionStruct::TokenMetadata(token_metadata));
                }
                _ => {
                    // Handle other extension types as needed
                }
            }
        }

        if !ext_structs.is_empty() {
            extensions_vec = Some(ext_structs);
        }
    }

    let compressed_mint = CompressedMint {
        base: BaseCompressedMint {
            version: mint_instruction_data.mint.base.version,
            spl_mint: mint_instruction_data.mint.base.spl_mint,
            supply: mint_instruction_data.mint.base.supply.into(),
            decimals: mint_instruction_data.mint.base.decimals,
            is_decompressed: mint_instruction_data.mint.base.is_decompressed(),
            mint_authority: mint_instruction_data.mint.base.mint_authority.map(|x| *x),
            freeze_authority: mint_instruction_data.mint.base.freeze_authority.map(|x| *x),
        },
        extensions: extensions_vec,
    };
    let input_data_hash = Sha256BE::hash(compressed_mint.try_to_vec().unwrap().as_slice())?;

    // 2. Set InAccount fields
    input_compressed_account.set(
        COMPRESSED_MINT_DISCRIMINATOR,
        input_data_hash,
        &merkle_context,
        mint_instruction_data.root_index,
        0,
        Some(mint_instruction_data.compressed_address.as_ref()),
    )?;

    Ok(())
}
#[inline(always)]
pub fn get_zero_copy_config(
    parsed_instruction_data: &ZMintActionCompressedInstructionData,
) -> Result<CompressedMintConfig, CTokenError> {
    // Calculate final authority states and modify output config without touching instruction data
    let final_mint_authority = parsed_instruction_data.mint.base.mint_authority.is_some();
    let final_freeze_authority = parsed_instruction_data.mint.base.freeze_authority.is_some();
    let (_, output_extensions_config, _) =
        crate::extensions::process_extensions_config_with_actions(
            parsed_instruction_data.mint.extensions.as_ref(),
            &parsed_instruction_data.actions,
        )?;

    Ok(CompressedMintConfig {
        base: BaseCompressedMintConfig {
            mint_authority: (final_mint_authority, ()),
            freeze_authority: (final_freeze_authority, ()),
        },
        extensions: (
            !output_extensions_config.is_empty(),
            output_extensions_config,
        ),
    })
}

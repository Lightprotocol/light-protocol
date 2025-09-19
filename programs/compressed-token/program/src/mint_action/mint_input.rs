use anchor_lang::solana_program::program_error::ProgramError;
use arrayvec::ArrayVec;
use borsh::BorshSerialize;
use light_compressed_account::{instruction_data::with_readonly::ZInAccountMut, Pubkey};
use light_ctoken_types::{
    instructions::{
        extensions::ZExtensionInstructionData, mint_action::ZMintActionCompressedInstructionData,
    },
    state::{
        AdditionalMetadata, BaseMint, CompressedMint, CompressedMintConfig, CompressedMintMetadata,
        ExtensionStruct, TokenMetadata,
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
    let mut extensions_vec = None;
    if let Some(extensions) = mint_instruction_data.mint.extensions.as_deref() {
        let mut ext_structs = Vec::new();
        for ext in extensions {
            match ext {
                ZExtensionInstructionData::TokenMetadata(metadata_ix) => {
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
                        update_authority: metadata_ix
                            .update_authority
                            .as_ref()
                            .map(|data| **data)
                            .unwrap_or_else(|| Pubkey::new_from_array([0u8; 32])),
                        mint: mint_instruction_data.mint.metadata.spl_mint,
                        name: metadata_ix.name.to_vec(),
                        symbol: metadata_ix.symbol.to_vec(),
                        uri: metadata_ix.uri.to_vec(),
                        additional_metadata,
                    };

                    ext_structs.push(ExtensionStruct::TokenMetadata(token_metadata));
                }
                _ => {
                    return Err(CTokenError::UnsupportedExtension.into());
                }
            }
        }

        if !ext_structs.is_empty() {
            extensions_vec = Some(ext_structs);
        }
    }

    let compressed_mint = CompressedMint {
        base: BaseMint {
            mint_authority: mint_instruction_data.mint.mint_authority.map(|x| *x),
            supply: mint_instruction_data.mint.supply.into(),
            decimals: mint_instruction_data.mint.decimals,
            is_initialized: true,
            freeze_authority: mint_instruction_data.mint.freeze_authority.map(|x| *x),
        },
        metadata: CompressedMintMetadata {
            version: mint_instruction_data.mint.metadata.version,
            spl_mint: mint_instruction_data.mint.metadata.spl_mint,
            spl_mint_initialized: mint_instruction_data.mint.metadata.spl_mint_initialized(),
        },
        extensions: extensions_vec,
    };
    let mut bytes = ArrayVec::<u8, 1024>::new();
    compressed_mint.serialize(&mut bytes)?;
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

    Ok(())
}

#[inline(always)]
pub fn get_zero_copy_config(
    parsed_instruction_data: &ZMintActionCompressedInstructionData,
) -> Result<CompressedMintConfig, CTokenError> {
    let (_, output_extensions_config, _) =
        crate::extensions::process_extensions_config_with_actions(
            parsed_instruction_data.mint.extensions.as_ref(),
            &parsed_instruction_data.actions,
        )?;

    Ok(CompressedMintConfig {
        base: (),
        metadata: (),
        extensions: (
            !output_extensions_config.is_empty(),
            output_extensions_config,
        ),
    })
}

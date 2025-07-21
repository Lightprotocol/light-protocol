use light_compressed_account::compressed_account::PackedMerkleContext;
use light_compressed_account::{instruction_data::compressed_proof::CompressedProof, Pubkey};
use light_zero_copy::ZeroCopy;

use crate::CTokenError;
use crate::{
    instructions::extensions::ExtensionInstructionData,
    state::{CompressedMint, ExtensionStruct},
    AnchorDeserialize, AnchorSerialize,
};

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct CreateCompressedMintInstructionData {
    pub decimals: u8,
    pub mint_authority: Pubkey,
    pub proof: CompressedProof,
    pub mint_bump: u8,
    pub address_merkle_tree_root_index: u16,
    // compressed address TODO: make a type CompressedAddress (not straight forward because of AnchorSerialize)
    pub mint_address: [u8; 32],
    pub freeze_authority: Option<Pubkey>,
    pub version: u8,
    pub extensions: Option<Vec<ExtensionInstructionData>>,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct UpdateCompressedMintInstructionData {
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
    pub address: [u8; 32],
    pub proof: Option<CompressedProof>,
    pub mint: CompressedMintInstructionData,
}

#[derive(Debug, PartialEq, Eq, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct CompressedMintInstructionData {
    /// Version for upgradability
    pub version: u8,
    /// Pda with seed address of compressed mint
    pub spl_mint: Pubkey,
    /// Total supply of tokens.
    pub supply: u64,
    /// Number of base 10 digits to the right of the decimal place.
    pub decimals: u8,
    /// Extension, necessary for mint to.
    pub is_decompressed: bool,
    /// Optional authority used to mint new tokens. The mint authority may only
    /// be provided during mint creation. If no mint authority is present
    /// then the mint has a fixed supply and no further tokens may be
    /// minted.
    pub mint_authority: Option<Pubkey>,
    /// Optional authority to freeze token accounts.
    pub freeze_authority: Option<Pubkey>,
    pub extensions: Option<Vec<ExtensionInstructionData>>,
}
impl TryFrom<CompressedMint> for CompressedMintInstructionData {
    type Error = CTokenError;

    fn try_from(mint: CompressedMint) -> Result<Self, Self::Error> {
        let extensions = match mint.extensions {
            Some(exts) => {
                let converted_exts: Result<Vec<_>, Self::Error> = exts
                    .into_iter()
                    .map(|ext| match ext {
                       /* ExtensionStruct::MetadataPointer(metadata_pointer) => {
                            Ok(ExtensionInstructionData::MetadataPointer(
                                crate::instructions::extensions::metadata_pointer::InitMetadataPointer {
                                    authority: metadata_pointer.authority,
                                    metadata_address: metadata_pointer.metadata_address,
                                },
                            ))
                        }*/
                        ExtensionStruct::TokenMetadata(token_metadata) => {
                            Ok(ExtensionInstructionData::TokenMetadata(
                                crate::instructions::extensions::token_metadata::TokenMetadataInstructionData {
                                    update_authority: token_metadata.update_authority,
                                    metadata: token_metadata.metadata,
                                    additional_metadata: Some(token_metadata.additional_metadata),
                                    version: token_metadata.version,
                                },
                            ))
                        }
                        _ => {
                            Err(CTokenError::UnsupportedExtension)
                        }
                    })
                    .collect();
                Some(converted_exts?)
            }
            None => None,
        };

        Ok(Self {
            version: mint.version,
            spl_mint: mint.spl_mint,
            supply: mint.supply,
            decimals: mint.decimals,
            is_decompressed: mint.is_decompressed,
            mint_authority: mint.mint_authority,
            freeze_authority: mint.freeze_authority,
            extensions,
        })
    }
}

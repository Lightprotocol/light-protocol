use light_compressed_account::{instruction_data::compressed_proof::CompressedProof, Pubkey};
use light_zero_copy::ZeroCopy;

use super::{
    CpiContext, CreateSplMintAction, MintToAction, MintToDecompressedAction,
    RemoveMetadataKeyAction, UpdateAuthority, UpdateMetadataAuthorityAction,
    UpdateMetadataFieldAction,
};
use crate::{
    instructions::extensions::ExtensionInstructionData,
    state::{CompressedMint, CompressedMintMetadata, ExtensionStruct},
    AnchorDeserialize, AnchorSerialize, CTokenError,
};

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub enum Action {
    /// Mint compressed tokens to compressed accounts.
    MintTo(MintToAction),
    /// Update mint authority of a compressed mint account.
    UpdateMintAuthority(UpdateAuthority),
    /// Update freeze authority of a compressed mint account.
    UpdateFreezeAuthority(UpdateAuthority),
    /// Create an spl mint for a cmint.
    /// - existing supply is minted to a token pool account.
    /// - mint and freeze authority are a ctoken pda.
    /// - is an spl-token-2022 mint account.
    CreateSplMint(CreateSplMintAction),
    /// Mint ctokens from a cmint to a ctoken solana account
    /// (tokens are not compressed but not spl tokens).
    MintToDecompressed(MintToDecompressedAction),
    UpdateMetadataField(UpdateMetadataFieldAction),
    UpdateMetadataAuthority(UpdateMetadataAuthorityAction),
    RemoveMetadataKey(RemoveMetadataKeyAction),
}

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct MintActionCompressedInstructionData {
    pub create_mint: bool,
    /// Only used if create mint
    pub mint_bump: u8,
    /// Only set if mint already exists
    pub leaf_index: u32,
    /// Only set if mint already exists
    pub prove_by_index: bool,
    /// If create mint, root index of address proof
    /// If mint already exists, root index of validity proof
    /// If proof by index not used.
    pub root_index: u16,
    pub compressed_address: [u8; 32],
    pub token_pool_bump: u8,
    pub token_pool_index: u8,
    pub actions: Vec<Action>,
    pub proof: Option<CompressedProof>,
    pub cpi_context: Option<CpiContext>,
    /// If some -> no input because we create mint
    pub mint: CompressedMintInstructionData,
}

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy, PartialEq)]
pub struct CompressedMintWithContext {
    pub leaf_index: u32,
    pub prove_by_index: bool,
    pub root_index: u16,
    pub address: [u8; 32],
    pub mint: CompressedMintInstructionData,
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct CompressedMintInstructionData {
    /// Total supply of tokens.
    pub supply: u64,
    /// Number of base 10 digits to the right of the decimal place.
    pub decimals: u8,
    /// Light Protocol-specific metadata
    pub metadata: CompressedMintMetadata,
    /// Optional authority used to mint new tokens. The mint authority may only
    /// be provided during mint creation. If no mint authority is present
    /// then the mint has a fixed supply and no further tokens may be
    /// minted.
    pub mint_authority: Option<Pubkey>,
    /// Optional authority to freeze token accounts.
    pub freeze_authority: Option<Pubkey>,
    /// Extensions for additional functionality
    pub extensions: Option<Vec<ExtensionInstructionData>>,
}

// TODO: add functional test
impl TryFrom<CompressedMint> for CompressedMintInstructionData {
    type Error = CTokenError;

    fn try_from(mint: CompressedMint) -> Result<Self, Self::Error> {
        let extensions = match mint.extensions {
            Some(exts) => {
                let converted_exts: Result<Vec<_>, Self::Error> = exts
                    .into_iter()
                    .map(|ext| match ext {
                        ExtensionStruct::TokenMetadata(token_metadata) => {
                            Ok(ExtensionInstructionData::TokenMetadata(
                                crate::instructions::extensions::token_metadata::TokenMetadataInstructionData {
                                    update_authority: if token_metadata.update_authority == [0u8;32] {None}else {Some(token_metadata.update_authority)},
                                    name: token_metadata.name,
                                    symbol: token_metadata.symbol,
                                    uri: token_metadata.uri,
                                    additional_metadata: Some(token_metadata.additional_metadata),

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
            supply: mint.base.supply,
            decimals: mint.base.decimals,
            metadata: mint.metadata,
            mint_authority: mint.base.mint_authority,
            freeze_authority: mint.base.freeze_authority,
            extensions,
        })
    }
}

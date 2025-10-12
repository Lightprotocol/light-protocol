use light_compressed_account::{instruction_data::compressed_proof::CompressedProof, Pubkey};
use light_zero_copy::ZeroCopy;

use super::{
    CpiContext, CreateSplMintAction, MintToCTokenAction, MintToCompressedAction,
    RemoveMetadataKeyAction, UpdateAuthority, UpdateMetadataAuthorityAction,
    UpdateMetadataFieldAction,
};
use crate::{
    instructions::extensions::{ExtensionInstructionData, ZExtensionInstructionData},
    state::{
        AdditionalMetadata, BaseMint, CompressedMint, CompressedMintMetadata, ExtensionStruct,
        TokenMetadata,
    },
    AnchorDeserialize, AnchorSerialize, CTokenError,
};

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub enum Action {
    /// Mint compressed tokens to compressed accounts.
    MintToCompressed(MintToCompressedAction),
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
    MintToCToken(MintToCTokenAction),
    UpdateMetadataField(UpdateMetadataFieldAction),
    UpdateMetadataAuthority(UpdateMetadataAuthorityAction),
    RemoveMetadataKey(RemoveMetadataKeyAction),
}

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct MintActionCompressedInstructionData {
    /// Only set if mint already exists
    pub leaf_index: u32,
    /// Only set if mint already exists
    pub prove_by_index: bool,
    /// If create mint, root index of address proof
    /// If mint already exists, root index of validity proof
    /// If proof by index not used.
    pub root_index: u16,
    /// Address of the compressed account the mint is stored in.
    /// Derived from the associated spl mint pubkey.
    pub compressed_address: [u8; 32],
    /// Used to check token pool derivation.
    /// Only required if associated spl mint exists and actions contain mint actions.
    pub token_pool_bump: u8,
    /// Used to check token pool derivation.
    /// Only required if associated spl mint exists and actions contain mint actions.
    pub token_pool_index: u8,
    pub create_mint: Option<CreateMint>,
    pub actions: Vec<Action>,
    pub proof: Option<CompressedProof>,
    pub cpi_context: Option<CpiContext>,
    pub mint: CompressedMintInstructionData,
}

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, Default, AnchorDeserialize, ZeroCopy)]
pub struct CreateMint {
    /// Only used if create mint
    pub mint_bump: u8,
    /// Placeholder to enable cmints in multiple address trees.
    /// Currently set to 0.
    pub read_only_address_trees: [u8; 4],
    /// Placeholder to enable cmints in multiple address trees.
    /// Currently set to 0.
    pub read_only_address_tree_root_indices: [u16; 4],
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

impl<'a> TryFrom<&ZCompressedMintInstructionData<'a>> for CompressedMint {
    type Error = CTokenError;

    fn try_from(
        instruction_data: &ZCompressedMintInstructionData<'a>,
    ) -> Result<Self, Self::Error> {
        let extensions = match &instruction_data.extensions {
            Some(exts) => {
                let converted_exts: Result<Vec<_>, Self::Error> = exts
                    .iter()
                    .map(|ext| match ext {
                        ZExtensionInstructionData::TokenMetadata(token_metadata_data) => {
                            Ok(ExtensionStruct::TokenMetadata(TokenMetadata {
                                update_authority: token_metadata_data
                                    .update_authority
                                    .map(|p| *p)
                                    .unwrap_or_else(|| Pubkey::from([0u8; 32])),
                                mint: instruction_data.metadata.mint, // Use the mint from metadata
                                name: token_metadata_data.name.to_vec(),
                                symbol: token_metadata_data.symbol.to_vec(),
                                uri: token_metadata_data.uri.to_vec(),
                                additional_metadata: token_metadata_data
                                    .additional_metadata
                                    .as_ref()
                                    .map(|ams| {
                                        ams.iter()
                                            .map(|am| AdditionalMetadata {
                                                key: am.key.to_vec(),
                                                value: am.value.to_vec(),
                                            })
                                            .collect()
                                    })
                                    .unwrap_or_else(Vec::new),
                            }))
                        }
                        _ => Err(CTokenError::UnsupportedExtension),
                    })
                    .collect();
                Some(converted_exts?)
            }
            None => None,
        };

        Ok(Self {
            base: BaseMint {
                mint_authority: instruction_data.mint_authority.map(|p| *p),
                supply: instruction_data.supply.into(),
                decimals: instruction_data.decimals,
                is_initialized: true, // Always true for compressed mints
                freeze_authority: instruction_data.freeze_authority.map(|p| *p),
            },
            metadata: CompressedMintMetadata {
                version: instruction_data.metadata.version,
                spl_mint_initialized: instruction_data.metadata.spl_mint_initialized(),
                mint: instruction_data.metadata.mint,
            },
            extensions,
        })
    }
}

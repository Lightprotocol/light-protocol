use light_compressed_account::{instruction_data::compressed_proof::CompressedProof, Pubkey};
use light_zero_copy::ZeroCopy;

use super::{
    CpiContext, CreateSplMintAction, MintToAction, MintToDecompressedAction,
    RemoveMetadataKeyAction, UpdateAuthority, UpdateMetadataAuthorityAction,
    UpdateMetadataFieldAction,
};
use crate::{
    instructions::extensions::ExtensionInstructionData,
    state::{CompressedMint, ExtensionStruct},
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
    /// If some -> no input because we create mint
    pub mint: CompressedMintInstructionData,
    pub token_pool_bump: u8,
    pub token_pool_index: u8,
    pub actions: Vec<Action>,
    pub proof: Option<CompressedProof>,
    pub cpi_context: Option<CpiContext>,
}

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct CompressedMintWithContext {
    pub leaf_index: u32,
    pub prove_by_index: bool,
    pub root_index: u16,
    pub address: [u8; 32],
    pub mint: CompressedMintInstructionData,
}

impl CompressedMintWithContext {
    pub fn new(
        compressed_address: [u8; 32],
        root_index: u16,
        decimals: u8,
        mint_authority: Option<Pubkey>,
        freeze_authority: Option<Pubkey>,
        spl_mint: Pubkey,
    ) -> Self {
        Self {
            leaf_index: 0,
            prove_by_index: false,
            root_index,
            address: compressed_address,
            mint: CompressedMintInstructionData {
                version: 0,
                spl_mint,
                supply: 0, // TODO: dynamic?
                decimals,
                is_decompressed: false,
                mint_authority,
                freeze_authority,
                extensions: None,
            },
        }
    }

    pub fn new_with_extensions(
        compressed_address: [u8; 32],
        root_index: u16,
        decimals: u8,
        mint_authority: Option<Pubkey>,
        freeze_authority: Option<Pubkey>,
        spl_mint: Pubkey,
        extensions: Option<Vec<ExtensionInstructionData>>,
    ) -> Self {
        Self {
            leaf_index: 0,
            prove_by_index: false,
            root_index,
            address: compressed_address,
            mint: CompressedMintInstructionData {
                version: 0,
                spl_mint,
                supply: 0,
                decimals,
                is_decompressed: false,
                mint_authority,
                freeze_authority,
                extensions,
            },
        }
    }
}
#[repr(C)]
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
            mint_authority: mint.mint_authority,
            is_decompressed: mint.is_decompressed,
            freeze_authority: mint.freeze_authority,
            extensions,
        })
    }
}

use anchor_lang::solana_program::program_error::ProgramError;
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::Hasher;

use crate::extensions::{
    metadata_pointer::{InitMetadataPointer, ZInitMetadataPointer},
    token_metadata::{TokenMetadataInstructionData, ZTokenMetadataInstructionData},
};
use crate::shared::context::TokenContext;

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum ExtensionInstructionData {
    // TODO: insert 18 placeholders to get consistent enum layout
    MetadataPointer(InitMetadataPointer),
    // TokenMetadata = 19,
    TokenMetadata(TokenMetadataInstructionData),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ZExtensionInstructionData<'a> {
    // TODO: insert 18 placeholders to get consistent enum layout
    MetadataPointer(ZInitMetadataPointer<'a>),
    // TokenMetadata = 19,
    TokenMetadata(ZTokenMetadataInstructionData<'a>),
}

impl ExtensionInstructionData {
    pub fn hash<H: Hasher>(
        &self,
        mint: light_compressed_account::Pubkey,
        context: &mut TokenContext,
    ) -> Result<[u8; 32], ProgramError> {
        match self {
            ExtensionInstructionData::MetadataPointer(metadata_pointer) => {
                metadata_pointer.hash_metadata_pointer::<H>(context)
            }
            ExtensionInstructionData::TokenMetadata(token_metadata) => {
                token_metadata.hash_token_metadata::<H>(mint, context)
            }
        }
    }
}

impl<'a> ZExtensionInstructionData<'a> {
    pub fn hash<H: Hasher>(
        &self,
        hashed_mint: &[u8; 32],
        context: &mut TokenContext,
    ) -> Result<[u8; 32], ProgramError> {
        match self {
            ZExtensionInstructionData::MetadataPointer(metadata_pointer) => {
                metadata_pointer.hash_metadata_pointer::<H>(context)
            }
            ZExtensionInstructionData::TokenMetadata(token_metadata) => {
                token_metadata.hash_token_metadata::<H>(hashed_mint, context)
            }
        }
    }
}

// Manual implementation of zero-copy traits for ExtensionInstructionData
impl<'a> light_zero_copy::borsh::Deserialize<'a> for ExtensionInstructionData {
    type Output = ZExtensionInstructionData<'a>;

    fn zero_copy_at(
        data: &'a [u8],
    ) -> Result<(Self::Output, &'a [u8]), light_zero_copy::errors::ZeroCopyError> {
        // Read discriminant (first 1 byte for borsh enum)
        if data.is_empty() {
            return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                1,
                data.len(),
            ));
        }

        let discriminant = data[0];
        let remaining_data = &data[1..];

        match discriminant {
            0 => {
                let (metadata_pointer, remaining_bytes) =
                    InitMetadataPointer::zero_copy_at(remaining_data)?;
                Ok((
                    ZExtensionInstructionData::MetadataPointer(metadata_pointer),
                    remaining_bytes,
                ))
            }
            1 => {
                let (token_metadata, remaining_bytes) =
                    TokenMetadataInstructionData::zero_copy_at(remaining_data)?;
                Ok((
                    ZExtensionInstructionData::TokenMetadata(token_metadata),
                    remaining_bytes,
                ))
            }
            _ => Err(light_zero_copy::errors::ZeroCopyError::InvalidConversion),
        }
    }
}

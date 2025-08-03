use light_compressed_account::{instruction_data::compressed_proof::CompressedProof, Pubkey};
use light_zero_copy::{borsh::Deserialize, ZeroCopy};

use crate::{
    instructions::{
        create_compressed_mint::{CompressedMintWithContext, ZCompressedMintWithContext},
        update_compressed_mint::UpdateMintCpiContext,
    },
    AnchorDeserialize, AnchorSerialize,
};

/// Authority types for compressed mint updates, following SPL Token-2022 pattern
#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub enum MetadataUpdate {
    UpdateAuthority(UpdateAuthority),
    UpdateKey(UpdateKey),
    RemoveKey(RemoveKey),
}

#[repr(u8)]
#[derive(Debug, Clone, PartialEq)]
pub enum ZMetadataUpdate<'a> {
    UpdateAuthority(ZUpdateAuthority<'a>),
    UpdateKey(ZUpdateKey<'a>),
    RemoveKey(ZRemoveKey<'a>),
}

#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct UpdateKey {
    pub extension_index: u8,
    pub key_index: u8,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct RemoveKey {
    pub extension_index: u8,
    pub key_index: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct UpdateAuthority {
    pub extension_index: u8,
    pub new_authority: Pubkey,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct UpdateMetadataInstructionData {
    pub mint: CompressedMintWithContext,
    pub updates: Vec<MetadataUpdate>,
    pub proof: Option<CompressedProof>,
    pub cpi_context: Option<UpdateMintCpiContext>,
}

pub struct ZUpdateMetadataInstructionData<'a> {
    pub mint: ZCompressedMintWithContext<'a>,
    pub updates: Vec<ZMetadataUpdate<'a>>,
    pub proof: <Option<CompressedProof> as Deserialize<'a>>::Output,
    pub cpi_context: <Option<UpdateMintCpiContext> as Deserialize<'a>>::Output,
}

impl<'a> Deserialize<'a> for UpdateMetadataInstructionData {
    type Output = ZUpdateMetadataInstructionData<'a>;
    fn zero_copy_at(
        bytes: &'a [u8],
    ) -> Result<(Self::Output, &'a [u8]), light_zero_copy::errors::ZeroCopyError> {
        let (mint, bytes) = CompressedMintWithContext::zero_copy_at(bytes)?;
        let (updates, bytes) = Vec::<MetadataUpdate>::zero_copy_at(bytes)?;
        let (proof, bytes) = <Option<CompressedProof> as Deserialize<'a>>::zero_copy_at(bytes)?;
        let (cpi_context, bytes) =
            <Option<UpdateMintCpiContext> as Deserialize<'a>>::zero_copy_at(bytes)?;
        Ok((
            ZUpdateMetadataInstructionData {
                mint,
                updates,
                proof,
                cpi_context,
            },
            bytes,
        ))
    }
}

impl<'a> Deserialize<'a> for MetadataUpdate {
    type Output = ZMetadataUpdate<'a>;
    fn zero_copy_at(
        bytes: &'a [u8],
    ) -> Result<(Self::Output, &'a [u8]), light_zero_copy::errors::ZeroCopyError> {
        let (enum_bytes, bytes) = bytes.split_at(1);
        match enum_bytes[0] {
            0 => {
                let (authority, bytes) = UpdateAuthority::zero_copy_at(bytes)?;
                Ok((ZMetadataUpdate::UpdateAuthority(authority), bytes))
            }
            1 => {
                let (update_key, bytes) = UpdateKey::zero_copy_at(bytes)?;
                Ok((ZMetadataUpdate::UpdateKey(update_key), bytes))
            }
            2 => {
                let (remove_key, bytes) = RemoveKey::zero_copy_at(bytes)?;
                Ok((ZMetadataUpdate::RemoveKey(remove_key), bytes))
            }
            _ => Err(light_zero_copy::errors::ZeroCopyError::InvalidEnumValue),
        }
    }
}

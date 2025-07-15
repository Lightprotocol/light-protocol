use light_compressed_account::{
    instruction_data::data::ZOutputCompressedAccountWithPackedContextMut, Pubkey,
};
use light_hasher::{
    hash_to_field_size::hashv_to_bn254_field_size_be_const_array, DataHasher, Hasher, HasherError,
};
use light_zero_copy::{ZeroCopy, ZeroCopyMut, ZeroCopyNew};

use crate::{context::TokenContext, AnchorDeserialize, AnchorSerialize, CTokenError, ExtensionType};

/// Metadata pointer extension data for compressed mints.
#[derive(
    Debug, Clone, PartialEq, Eq, AnchorSerialize, ZeroCopy, AnchorDeserialize, ZeroCopyMut,
)]
pub struct MetadataPointer {
    /// Authority that can set the metadata address
    pub authority: Option<Pubkey>,
    /// (Compressed) address that holds the metadata (in token 22)
    pub metadata_address: Option<Pubkey>,
}

impl DataHasher for MetadataPointer {
    fn hash<H: Hasher>(&self) -> Result<[u8; 32], HasherError> {
        let mut discriminator = [0u8; 32];
        discriminator[31] = ExtensionType::MetadataPointer as u8;
        let hashed_metadata_address = if let Some(metadata_address) = self.metadata_address {
            hashv_to_bn254_field_size_be_const_array::<2>(&[metadata_address.as_ref()])?
        } else {
            [0u8; 32]
        };
        let hashed_authority = if let Some(authority) = self.authority {
            hashv_to_bn254_field_size_be_const_array::<2>(&[authority.as_ref()])?
        } else {
            [0u8; 32]
        };
        H::hashv(&[
            discriminator.as_slice(),
            hashed_metadata_address.as_slice(),
            hashed_authority.as_slice(),
        ])
    }
}

/// Instruction data for initializing metadata pointer
#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct InitMetadataPointer {
    /// The authority that can set the metadata address
    pub authority: Option<Pubkey>,
    /// The account address that holds the metadata
    pub metadata_address: Option<Pubkey>,
}

impl InitMetadataPointer {
    pub fn hash_metadata_pointer<H: Hasher>(
        &self,
        context: &mut TokenContext,
    ) -> Result<[u8; 32], CTokenError> {
        let mut discriminator = [0u8; 32];
        discriminator[31] = ExtensionType::MetadataPointer as u8;

        let hashed_metadata_address = if let Some(metadata_address) = self.metadata_address {
            context.get_or_hash_pubkey(&metadata_address.into())
        } else {
            [0u8; 32]
        };

        let hashed_authority = if let Some(authority) = self.authority {
            context.get_or_hash_pubkey(&authority.into())
        } else {
            [0u8; 32]
        };

        H::hashv(&[
            discriminator.as_slice(),
            hashed_metadata_address.as_slice(),
            hashed_authority.as_slice(),
        ])
        .map_err(CTokenError::from)
    }
}

impl ZInitMetadataPointer<'_> {
    pub fn hash_metadata_pointer<H: Hasher>(
        &self,
        context: &mut TokenContext,
    ) -> Result<[u8; 32], CTokenError> {
        let mut discriminator = [0u8; 32];
        discriminator[31] = ExtensionType::MetadataPointer as u8;

        let hashed_metadata_address = if let Some(metadata_address) = self.metadata_address {
            context.get_or_hash_pubkey(&(*metadata_address).into())
        } else {
            [0u8; 32]
        };

        let hashed_authority = if let Some(authority) = self.authority {
            context.get_or_hash_pubkey(&(*authority).into())
        } else {
            [0u8; 32]
        };

        H::hashv(&[
            discriminator.as_slice(),
            hashed_metadata_address.as_slice(),
            hashed_authority.as_slice(),
        ])
        .map_err(CTokenError::from)
    }
}

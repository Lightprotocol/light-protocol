use anchor_compressed_token::ErrorCode;
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::ZeroCopy;

use crate::extensions::metadata_pointer::{
    MetadataPointer, MetadataPointerConfig, ZMetadataPointer, ZMetadataPointerMut,
};

pub mod metadata_pointer;
pub mod processor;
pub mod token_metadata;

#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(u16)]
pub enum ExtensionType {
    /// Mint contains a pointer to another account (or the same account) that
    /// holds metadata
    MetadataPointer = 18,
    TokenMetadata = 19,
}
// use spl_token_2022::extension::ExtensionType SplExtensionType;

impl TryFrom<u16> for ExtensionType {
    type Error = ErrorCode;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            18 => Ok(ExtensionType::MetadataPointer),
            19 => Ok(ExtensionType::TokenMetadata),
            _ => Err(ErrorCode::InvalidExtensionType),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum ExtensionStruct {
    /// Mint contains a pointer to another account (or the same account) that
    /// holds metadata
    MetadataPointer(MetadataPointer),
    // TokenMetadata = 19,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ZExtensionStruct<'a> {
    /// Mint contains a pointer to another account (or the same account) that
    /// holds metadata
    MetadataPointer(ZMetadataPointer<'a>),
    // TokenMetadata = 19,
}

#[derive(Debug)]
pub enum ZExtensionStructMut<'a> {
    /// Mint contains a pointer to another account (or the same account) that
    /// holds metadata
    MetadataPointer(ZMetadataPointerMut<'a>),
    // TokenMetadata = 19,
}

// Manual implementation of zero-copy traits for ExtensionStruct
impl<'a> light_zero_copy::borsh::Deserialize<'a> for ExtensionStruct {
    type Output = ZExtensionStruct<'a>;

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
                // MetadataPointer variant
                let (metadata_pointer, remaining_bytes) =
                    MetadataPointer::zero_copy_at(remaining_data)?;
                Ok((
                    ZExtensionStruct::MetadataPointer(metadata_pointer),
                    remaining_bytes,
                ))
            }
            _ => Err(light_zero_copy::errors::ZeroCopyError::InvalidConversion),
        }
    }
}

impl<'a> light_zero_copy::borsh_mut::DeserializeMut<'a> for ExtensionStruct {
    type Output = ZExtensionStructMut<'a>;

    fn zero_copy_at_mut(
        data: &'a mut [u8],
    ) -> Result<(Self::Output, &'a mut [u8]), light_zero_copy::errors::ZeroCopyError> {
        // Read discriminant (first 1 byte for borsh enum)
        if data.is_empty() {
            return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                1,
                data.len(),
            ));
        }

        let discriminant = data[0];
        let remaining_data = &mut data[1..];

        match discriminant {
            0 => {
                // MetadataPointer variant
                let (metadata_pointer, remaining_bytes) =
                    MetadataPointer::zero_copy_at_mut(remaining_data)?;
                Ok((
                    ZExtensionStructMut::MetadataPointer(metadata_pointer),
                    remaining_bytes,
                ))
            }
            _ => Err(light_zero_copy::errors::ZeroCopyError::InvalidConversion),
        }
    }
}

impl<'a> light_zero_copy::ZeroCopyNew<'a> for ExtensionStruct {
    type ZeroCopyConfig = ExtensionStructConfig;
    type Output = ZExtensionStructMut<'a>;

    fn byte_len(config: &Self::ZeroCopyConfig) -> usize {
        match config {
            ExtensionStructConfig::MetadataPointer(metadata_config) => {
                // 1 byte for discriminant + MetadataPointer size
                1 + MetadataPointer::byte_len(metadata_config)
            }
        }
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), light_zero_copy::errors::ZeroCopyError> {
        match config {
            ExtensionStructConfig::MetadataPointer(metadata_config) => {
                // Write discriminant (0 for MetadataPointer)
                if bytes.is_empty() {
                    return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1,
                        bytes.len(),
                    ));
                }
                bytes[0] = 0u8;

                // Create MetadataPointer at offset 1
                let (metadata_pointer, remaining_bytes) =
                    MetadataPointer::new_zero_copy(&mut bytes[1..], metadata_config)?;
                Ok((
                    ZExtensionStructMut::MetadataPointer(metadata_pointer),
                    remaining_bytes,
                ))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExtensionStructConfig {
    MetadataPointer(MetadataPointerConfig),
}


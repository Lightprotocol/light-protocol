use light_hasher::Hasher;
use spl_pod::solana_msg::msg;

use crate::{
    state::{
        extensions::{
            CompressibleExtension, TokenMetadata, TokenMetadataConfig, ZTokenMetadata,
            ZTokenMetadataMut,
        },
        CompressibleExtensionConfig,
    },
    AnchorDeserialize, AnchorSerialize, CTokenError,
};

#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub enum ExtensionStruct {
    Placeholder0,
    Placeholder1,
    Placeholder2,
    Placeholder3,
    Placeholder4,
    Placeholder5,
    Placeholder6,
    Placeholder7,
    Placeholder8,
    Placeholder9,
    Placeholder10,
    Placeholder11,
    Placeholder12,
    Placeholder13,
    Placeholder14,
    Placeholder15,
    Placeholder16,
    Placeholder17,
    Placeholder18, // MetadataPointer(MetadataPointer),
    TokenMetadata(TokenMetadata),
    Placeholder20,
    Placeholder21,
    Placeholder22,
    Placeholder23,
    Placeholder24,
    Placeholder25,
    /// Account contains compressible timing data and rent authority
    Compressible(CompressibleExtension),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ZExtensionStruct<'a> {
    Placeholder0,
    Placeholder1,
    Placeholder2,
    Placeholder3,
    Placeholder4,
    Placeholder5,
    Placeholder6,
    Placeholder7,
    Placeholder8,
    Placeholder9,
    Placeholder10,
    Placeholder11,
    Placeholder12,
    Placeholder13,
    Placeholder14,
    Placeholder15,
    Placeholder16,
    Placeholder17,
    Placeholder18, // MetadataPointer(ZMetadataPointer<'a>),
    TokenMetadata(ZTokenMetadata<'a>),
    Placeholder20,
    Placeholder21,
    Placeholder22,
    Placeholder23,
    Placeholder24,
    Placeholder25,
    /// Account contains compressible timing data and rent authority
    Compressible(<CompressibleExtension as light_zero_copy::borsh::Deserialize<'a>>::Output),
}

#[derive(Debug)]
pub enum ZExtensionStructMut<'a> {
    Placeholder0,
    Placeholder1,
    Placeholder2,
    Placeholder3,
    Placeholder4,
    Placeholder5,
    Placeholder6,
    Placeholder7,
    Placeholder8,
    Placeholder9,
    Placeholder10,
    Placeholder11,
    Placeholder12,
    Placeholder13,
    Placeholder14,
    Placeholder15,
    Placeholder16,
    Placeholder17,
    Placeholder18, //  MetadataPointer(ZMetadataPointerMut<'a>),
    TokenMetadata(ZTokenMetadataMut<'a>),
    Placeholder20,
    Placeholder21,
    Placeholder22,
    Placeholder23,
    Placeholder24,
    Placeholder25,
    /// Account contains compressible timing data and rent authority
    Compressible(<CompressibleExtension as light_zero_copy::borsh_mut::DeserializeMut<'a>>::Output),
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
            /* 18 => {
                // MetadataPointer variant
                let (metadata_pointer, remaining_bytes) =
                    MetadataPointer::zero_copy_at(remaining_data)?;
                Ok((
                    ZExtensionStruct::MetadataPointer(metadata_pointer),
                    remaining_bytes,
                ))
            }*/
            19 => {
                let (token_metadata, remaining_bytes) =
                    TokenMetadata::zero_copy_at(remaining_data)?;
                Ok((
                    ZExtensionStruct::TokenMetadata(token_metadata),
                    remaining_bytes,
                ))
            }
            26 => {
                // Compressible variant
                let (compressible_ext, remaining_bytes) =
                    CompressibleExtension::zero_copy_at(remaining_data)?;
                Ok((
                    ZExtensionStruct::Compressible(compressible_ext),
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
            /* 18 => {
                // MetadataPointer variant
                let (metadata_pointer, remaining_bytes) =
                    MetadataPointer::zero_copy_at_mut(remaining_data)?;
                Ok((
                    ZExtensionStructMut::MetadataPointer(metadata_pointer),
                    remaining_bytes,
                ))
            }*/
            19 => {
                let (token_metadata, remaining_bytes) =
                    TokenMetadata::zero_copy_at_mut(remaining_data)?;
                Ok((
                    ZExtensionStructMut::TokenMetadata(token_metadata),
                    remaining_bytes,
                ))
            }
            26 => {
                // Compressible variant
                let (compressible_ext, remaining_bytes) =
                    CompressibleExtension::zero_copy_at_mut(remaining_data)?;
                Ok((
                    ZExtensionStructMut::Compressible(compressible_ext),
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
    // TODO: return Result
    fn byte_len(config: &Self::ZeroCopyConfig) -> usize {
        match config {
            /* ExtensionStructConfig::MetadataPointer(metadata_config) => {
                // 1 byte for discriminant + MetadataPointer size
                1 + MetadataPointer::byte_len(metadata_config)
            } */
            ExtensionStructConfig::TokenMetadata(token_metadata_config) => {
                // 1 byte for discriminant + TokenMetadata size
                1 + TokenMetadata::byte_len(token_metadata_config)
            }
            ExtensionStructConfig::Compressible => {
                // 1 byte for discriminant + CompressibleExtension size
                1 + std::mem::size_of::<CompressibleExtension>()
            }
            _ => {
                msg!("Invalid extension type returning 0");
                0
            }
        }
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), light_zero_copy::errors::ZeroCopyError> {
        match config {
            /* ExtensionStructConfig::MetadataPointer(metadata_config) => {
                // Write discriminant (18 for MetadataPointer)
                if bytes.is_empty() {
                    return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1,
                        bytes.len(),
                    ));
                }
                bytes[0] = 18u8;

                // Create MetadataPointer at offset 1
                let (metadata_pointer, remaining_bytes) =
                    MetadataPointer::new_zero_copy(&mut bytes[1..], metadata_config)?;
                Ok((
                    ZExtensionStructMut::MetadataPointer(metadata_pointer),
                    remaining_bytes,
                ))
            } */
            ExtensionStructConfig::TokenMetadata(config) => {
                // Write discriminant (19 for TokenMetadata)
                if bytes.is_empty() {
                    return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1,
                        bytes.len(),
                    ));
                }
                bytes[0] = 19u8;

                let (token_metadata, remaining_bytes) =
                    TokenMetadata::new_zero_copy(&mut bytes[1..], config)?;
                Ok((
                    ZExtensionStructMut::TokenMetadata(token_metadata),
                    remaining_bytes,
                ))
            }
            ExtensionStructConfig::Compressible => {
                // Write discriminant (26 for Compressible)
                if bytes.is_empty() {
                    return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1,
                        bytes.len(),
                    ));
                }
                bytes[0] = 26u8;

                let (compressible_ext, remaining_bytes) = CompressibleExtension::new_zero_copy(
                    &mut bytes[1..],
                    CompressibleExtensionConfig {},
                )?;
                Ok((
                    ZExtensionStructMut::Compressible(compressible_ext),
                    remaining_bytes,
                ))
            }
            _ => Err(light_zero_copy::errors::ZeroCopyError::InvalidConversion),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExtensionStructConfig {
    Placeholder0,
    Placeholder1,
    Placeholder2,
    Placeholder3,
    Placeholder4,
    Placeholder5,
    Placeholder6,
    Placeholder7,
    Placeholder8,
    Placeholder9,
    Placeholder10,
    Placeholder11,
    Placeholder12,
    Placeholder13,
    Placeholder14,
    Placeholder15,
    Placeholder16,
    Placeholder17,
    Placeholder18, // MetadataPointer(MetadataPointerConfig),
    TokenMetadata(TokenMetadataConfig),
    Placeholder20,
    Placeholder21,
    Placeholder22,
    Placeholder23,
    Placeholder24,
    Placeholder25,
    Compressible,
}

impl ExtensionStruct {
    pub fn hash<H: Hasher>(&self) -> Result<[u8; 32], CTokenError> {
        match self {
            // ExtensionStruct::MetadataPointer(metadata_pointer) => Ok(metadata_pointer.hash::<H>()?),
            ExtensionStruct::TokenMetadata(token_metadata) => {
                // hash function is defined on the metadata level
                Ok(token_metadata.hash()?)
            }
            _ => Err(CTokenError::UnsupportedExtension),
        }
    }
}

use light_zero_copy::ZeroCopy;
use spl_pod::solana_msg::msg;

use crate::{
    state::{
        extensions::{CompressionInfo, TokenMetadata, TokenMetadataConfig, ZTokenMetadataMut},
        CompressionInfoConfig,
    },
    AnchorDeserialize, AnchorSerialize,
};

#[derive(Debug, Clone, Hash, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
#[repr(C)]
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
    Placeholder18,
    TokenMetadata(TokenMetadata),
    Placeholder20,
    Placeholder21,
    Placeholder22,
    Placeholder23,
    Placeholder24,
    Placeholder25,
    /// Account contains compressible timing data and rent authority
    Compressible(CompressionInfo),
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
    Placeholder18,
    TokenMetadata(ZTokenMetadataMut<'a>),
    Placeholder20,
    Placeholder21,
    Placeholder22,
    Placeholder23,
    Placeholder24,
    Placeholder25,
    /// Account contains compressible timing data and rent authority
    Compressible(<CompressionInfo as light_zero_copy::traits::ZeroCopyAtMut<'a>>::ZeroCopyAtMut),
}

impl<'a> light_zero_copy::traits::ZeroCopyAtMut<'a> for ExtensionStruct {
    type ZeroCopyAtMut = ZExtensionStructMut<'a>;

    fn zero_copy_at_mut(
        data: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), light_zero_copy::errors::ZeroCopyError> {
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
                    CompressionInfo::zero_copy_at_mut(remaining_data)?;
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

    fn byte_len(
        config: &Self::ZeroCopyConfig,
    ) -> Result<usize, light_zero_copy::errors::ZeroCopyError> {
        Ok(match config {
            ExtensionStructConfig::TokenMetadata(token_metadata_config) => {
                // 1 byte for discriminant + TokenMetadata size
                1 + TokenMetadata::byte_len(token_metadata_config)?
            }
            ExtensionStructConfig::Compressible(config) => {
                // 1 byte for discriminant + CompressionInfo size
                1 + CompressionInfo::byte_len(config)?
            }
            _ => {
                msg!("Invalid extension type returning");
                return Err(light_zero_copy::errors::ZeroCopyError::InvalidConversion);
            }
        })
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), light_zero_copy::errors::ZeroCopyError> {
        match config {
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
            ExtensionStructConfig::Compressible(config) => {
                // Write discriminant (26 for Compressible)
                if bytes.is_empty() {
                    return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1,
                        bytes.len(),
                    ));
                }
                bytes[0] = 26u8;

                let (compressible_ext, remaining_bytes) =
                    CompressionInfo::new_zero_copy(&mut bytes[1..], config)?;
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
    Compressible(CompressionInfoConfig),
}

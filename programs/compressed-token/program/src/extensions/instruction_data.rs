use borsh::{BorshDeserialize, BorshSerialize};

use crate::extensions::{
    metadata_pointer::{InitMetadataPointer, ZInitMetadataPointer},
    token_metadata::{TokenMetadataInstructionData, ZTokenMetadataInstructionData},
};

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
                // MetadataPointer variant
                let (metadata_pointer, remaining_bytes) =
                    InitMetadataPointer::zero_copy_at(remaining_data)?;
                Ok((
                    ZExtensionInstructionData::MetadataPointer(metadata_pointer),
                    remaining_bytes,
                ))
            }
            _ => Err(light_zero_copy::errors::ZeroCopyError::InvalidConversion),
        }
    }
}

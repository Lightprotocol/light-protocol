use anchor_compressed_token::ErrorCode;

pub mod metadata_pointer;
pub mod processor;
pub mod token_metadata;

pub enum ExtensionType {
    /// Mint contains a pointer to another account (or the same account) that
    /// holds metadata
    MetadataPointer,
    /// Mint contains token-metadata
    TokenMetadata,
}
// use spl_token_2022::extension::ExtensionType SplExtensionType;

impl TryFrom<u8> for ExtensionType {
    type Error = ErrorCode;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            18 => Ok(ExtensionType::MetadataPointer),
            19 => Ok(ExtensionType::TokenMetadata),
            _ => Err(ErrorCode::InvalidExtensionType),
        }
    }
}

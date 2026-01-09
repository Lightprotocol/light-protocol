pub mod compressed_only;
pub mod compressible;
pub mod token_metadata;
pub use compressed_only::{
    CompressedOnlyExtensionInstructionData, ZCompressedOnlyExtensionInstructionData,
};
pub use compressible::{CompressToPubkey, CompressibleExtensionInstructionData};
use light_compressible::compression_info::CompressionInfo;
use light_zero_copy::ZeroCopy;
pub use token_metadata::{TokenMetadataInstructionData, ZTokenMetadataInstructionData};

use crate::{AnchorDeserialize, AnchorSerialize};

#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
#[repr(C)]
pub enum ExtensionInstructionData {
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
    TokenMetadata(TokenMetadataInstructionData),
    Placeholder20,
    Placeholder21,
    Placeholder22,
    Placeholder23,
    Placeholder24,
    Placeholder25,
    Placeholder26,
    /// Reserved for PausableAccount extension
    Placeholder27,
    /// Reserved for PermanentDelegateAccount extension
    Placeholder28,
    Placeholder29,
    Placeholder30,
    /// CompressedOnly extension for compressed token accounts
    CompressedOnly(CompressedOnlyExtensionInstructionData),
    /// Compressible extension - reuses CompressionInfo from light_compressible
    /// Position 32 matches ExtensionStruct::Compressible
    Compressible(CompressionInfo),
}

/// Find the CompressedOnly extension from a TLV slice.
#[inline(always)]
pub fn find_compressed_only<'a>(
    tlv: &'a [ZExtensionInstructionData<'a>],
) -> Option<&'a ZCompressedOnlyExtensionInstructionData<'a>> {
    tlv.iter().find_map(|ext| match ext {
        ZExtensionInstructionData::CompressedOnly(data) => Some(data),
        _ => None,
    })
}

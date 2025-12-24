pub mod compressed_only;
pub mod pausable;
pub mod permanent_delegate;
pub mod token_metadata;
pub use compressed_only::CompressedOnlyExtensionInstructionData;
use light_zero_copy::ZeroCopy;
pub use pausable::PausableExtensionInstructionData;
pub use permanent_delegate::PermanentDelegateExtensionInstructionData;
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
    PausableAccount(PausableExtensionInstructionData),
    PermanentDelegateAccount(PermanentDelegateExtensionInstructionData),
    Placeholder29,
    Placeholder30,
    /// CompressedOnly extension for compressed token accounts
    CompressedOnly(CompressedOnlyExtensionInstructionData),
}

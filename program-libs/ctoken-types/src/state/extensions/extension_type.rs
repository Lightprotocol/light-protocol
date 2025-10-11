use crate::{AnchorDeserialize, AnchorSerialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorDeserialize, AnchorSerialize)]
#[repr(u8)] // Note: token22 uses u16
pub enum ExtensionType {
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
    /// Mint contains token-metadata.
    /// Unlike token22 there is no metadata pointer.
    TokenMetadata = 19,
    Placeholder20,
    Placeholder21,
    Placeholder22,
    Placeholder23,
    Placeholder24,
    Placeholder25,
    /// Account contains compressible timing data and rent authority
    Compressible = 26,
}

impl TryFrom<u8> for ExtensionType {
    type Error = crate::CTokenError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            19 => Ok(ExtensionType::TokenMetadata),
            26 => Ok(ExtensionType::Compressible),
            _ => Err(crate::CTokenError::UnsupportedExtension),
        }
    }
}

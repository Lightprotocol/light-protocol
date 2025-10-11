use crate::{AnchorDeserialize, AnchorSerialize};

/// Authority types for compressed mint updates, following SPL Token-2022 pattern
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub enum CompressedMintAuthorityType {
    /// Authority to mint new tokens
    MintTokens = 0,
    /// Authority to freeze token accounts
    FreezeAccount = 1,
}

impl TryFrom<u8> for CompressedMintAuthorityType {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(CompressedMintAuthorityType::MintTokens),
            1 => Ok(CompressedMintAuthorityType::FreezeAccount),
            _ => Err("Invalid authority type"),
        }
    }
}

impl From<CompressedMintAuthorityType> for u8 {
    fn from(authority_type: CompressedMintAuthorityType) -> u8 {
        authority_type as u8
    }
}

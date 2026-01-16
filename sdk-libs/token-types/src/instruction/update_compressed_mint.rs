use crate::{AnchorDeserialize, AnchorSerialize};

/// Authority types for compressed mint updates
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub enum MintAuthorityType {
    /// Authority to mint new tokens
    MintTokens = 0,
    /// Authority to freeze token accounts
    FreezeAccount = 1,
}

impl TryFrom<u8> for MintAuthorityType {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MintAuthorityType::MintTokens),
            1 => Ok(MintAuthorityType::FreezeAccount),
            _ => Err("Invalid authority type"),
        }
    }
}

impl From<MintAuthorityType> for u8 {
    fn from(authority_type: MintAuthorityType) -> u8 {
        authority_type as u8
    }
}

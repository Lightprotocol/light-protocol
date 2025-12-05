use crate::CTokenError;

/// TokenDataVersion is recorded in the token account discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TokenDataVersion {
    V1 = 1u8,
    V2 = 2u8,
    ShaFlat = 3u8,
}

impl TokenDataVersion {
    pub fn discriminator(&self) -> [u8; 8] {
        match self {
            TokenDataVersion::V1 => [2, 0, 0, 0, 0, 0, 0, 0], // 2 le
            TokenDataVersion::V2 => [0, 0, 0, 0, 0, 0, 0, 3], // 3 be
            TokenDataVersion::ShaFlat => [0, 0, 0, 0, 0, 0, 0, 4], // 4 be
        }
    }

    pub fn from_discriminator(discriminator: [u8; 8]) -> Result<Self, CTokenError> {
        match discriminator {
            [2, 0, 0, 0, 0, 0, 0, 0] => Ok(TokenDataVersion::V1), // 2 le
            [0, 0, 0, 0, 0, 0, 0, 3] => Ok(TokenDataVersion::V2), // 3 be
            [0, 0, 0, 0, 0, 0, 0, 4] => Ok(TokenDataVersion::ShaFlat), // 4 be
            _ => Err(CTokenError::InvalidTokenDataVersion),
        }
    }

    /// Serializes amount to bytes using version-specific endianness
    /// V1: little-endian, V2: big-endian
    pub fn serialize_amount_bytes(&self, amount: u64) -> Result<[u8; 32], CTokenError> {
        let mut amount_bytes = [0u8; 32];
        match self {
            TokenDataVersion::V1 => {
                amount_bytes[24..].copy_from_slice(&amount.to_le_bytes());
            }
            TokenDataVersion::V2 => {
                amount_bytes[24..].copy_from_slice(&amount.to_be_bytes());
            }
            _ => {
                return Err(CTokenError::InvalidTokenDataVersion);
            }
        }
        Ok(amount_bytes)
    }
}

impl TryFrom<u8> for TokenDataVersion {
    type Error = crate::CTokenError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(TokenDataVersion::V1),
            2 => Ok(TokenDataVersion::V2),
            3 => Ok(TokenDataVersion::ShaFlat),
            _ => Err(crate::CTokenError::InvalidTokenDataVersion),
        }
    }
}

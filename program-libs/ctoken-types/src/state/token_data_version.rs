/// TokenDataVersion is recorded in the token account discriminator.
#[repr(u8)]
pub enum TokenDataVersion {
    V1 = 1u8,
    V2 = 2u8,
}

impl TokenDataVersion {
    pub fn discriminator(&self) -> [u8; 8] {
        match self {
            TokenDataVersion::V1 => [2, 0, 0, 0, 0, 0, 0, 0], // 2 le
            TokenDataVersion::V2 => [0, 0, 0, 0, 0, 0, 0, 3], // 3 be
        }
    }

    /// Serializes amount to bytes using version-specific endianness
    /// V1: little-endian, V2: big-endian
    pub fn serialize_amount_bytes(&self, amount: u64) -> [u8; 32] {
        let mut amount_bytes = [0u8; 32];
        match self {
            TokenDataVersion::V1 => {
                amount_bytes[24..].copy_from_slice(&amount.to_le_bytes());
            }
            TokenDataVersion::V2 => {
                amount_bytes[24..].copy_from_slice(&amount.to_be_bytes());
            }
        }
        amount_bytes
    }
}

impl TryFrom<u8> for TokenDataVersion {
    type Error = crate::CTokenError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(TokenDataVersion::V1),
            2 => Ok(TokenDataVersion::V2),
            _ => Err(crate::CTokenError::InvalidTokenDataVersion),
        }
    }
}

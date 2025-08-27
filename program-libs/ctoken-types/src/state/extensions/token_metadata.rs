use light_compressed_account::Pubkey;
use light_hasher::HasherError;
use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize};

// TODO: decide whether to keep Shaflat
pub enum Version {
    Poseidon,
    Sha256,
    Keccak256,
    Sha256Flat,
}

impl TryFrom<u8> for Version {
    type Error = HasherError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            //0 => Ok(Version::Poseidon),
            //1 => Ok(Version::Sha256),
            // 2 => Ok(Version::Keccak256),
            3 => Ok(Version::Sha256Flat),
            // TODO: use real error
            _ => Err(HasherError::InvalidInputLength(value as usize, 3)),
        }
    }
}

// TODO: test deserialization equivalence
/// Used for onchain serialization
#[repr(C)]
#[derive(
    Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
pub struct TokenMetadata {
    // TODO: decide whether to move down for more efficient zero copy. Or impl manual zero copy.
    /// The authority that can sign to update the metadata
    pub update_authority: Option<Pubkey>,
    // TODO: decide whether to keep this.
    /// The associated mint, used to counter spoofing to be sure that metadata
    /// belongs to a particular mint
    pub mint: Pubkey,
    pub metadata: Metadata,
    /// Any additional metadata about the token as key-value pairs. The program
    /// must avoid storing the same key twice.
    pub additional_metadata: Vec<AdditionalMetadata>,
    // TODO: add check that if token is ShaFlat this also must be ShaFlat, right now we only allow shaflat
    /// 0: Poseidon, 1: Sha256, 2: Keccak256, 3: Sha256Flat
    pub version: u8,
}

// TODO: if version 0 we check all string len for less than 31 bytes
#[repr(C)]
#[derive(
    Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
pub struct Metadata {
    /// The longer name of the token
    pub name: Vec<u8>,
    /// The shortened symbol for the token
    pub symbol: Vec<u8>,
    /// The URI pointing to richer metadata
    pub uri: Vec<u8>,
}

#[repr(C)]
#[derive(
    Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
pub struct AdditionalMetadata {
    /// The key of the metadata
    pub key: Vec<u8>,
    /// The value of the metadata
    pub value: Vec<u8>,
}

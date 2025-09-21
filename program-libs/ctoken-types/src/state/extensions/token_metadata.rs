use light_compressed_account::Pubkey;
use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize};

/// Used for onchain serialization
#[repr(C)]
#[derive(
    Debug, Clone, Hash, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
pub struct TokenMetadata {
    /// The authority that can sign to update the metadata
    /// None if zero
    pub update_authority: Pubkey,
    /// The associated mint, used to counter spoofing to be sure that metadata
    /// belongs to a particular mint
    pub mint: Pubkey,
    /// The longer name of the token
    pub name: Vec<u8>,
    /// The shortened symbol for the token
    pub symbol: Vec<u8>,
    /// The URI pointing to richer metadata
    pub uri: Vec<u8>,
    /// Any additional metadata about the token as key-value pairs. The program
    /// must avoid storing the same key twice.
    pub additional_metadata: Vec<AdditionalMetadata>,
}

#[repr(C)]
#[derive(
    Debug, Clone, Hash, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
pub struct AdditionalMetadata {
    /// The key of the metadata
    pub key: Vec<u8>,
    /// The value of the metadata
    pub value: Vec<u8>,
}

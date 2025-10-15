use light_compressed_account::Pubkey;
use light_zero_copy::ZeroCopy;

use crate::{AnchorDeserialize, AnchorSerialize};

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct UpdateMetadataFieldAction {
    pub extension_index: u8, // Index of the TokenMetadata extension in the extensions array
    pub field_type: u8,      // 0=Name, 1=Symbol, 2=Uri, 3=Custom key
    pub key: Vec<u8>,        // Empty for Name/Symbol/Uri, key string for custom fields
    pub value: Vec<u8>,      // UTF-8 encoded value
}

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct UpdateMetadataAuthorityAction {
    pub extension_index: u8, // Index of the TokenMetadata extension in the extensions array
    pub new_authority: Pubkey, // Use zero bytes to set to None
}

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct RemoveMetadataKeyAction {
    pub extension_index: u8, // Index of the TokenMetadata extension in the extensions array
    pub key: Vec<u8>,        // UTF-8 encoded key to remove
    pub idempotent: u8,      // 0=false, 1=true - don't error if key doesn't exist
}

/// Authority types for compressed mint updates, following SPL Token-2022 pattern
#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
#[repr(C, u8)]
pub enum MetadataUpdate {
    UpdateAuthority(UpdateMetadataAuthority),
    UpdateKey(UpdateKey),
    RemoveKey(RemoveKey),
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct UpdateKey {
    pub extension_index: u8,
    pub key_index: u8,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct RemoveKey {
    pub extension_index: u8,
    pub key_index: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct UpdateMetadataAuthority {
    pub extension_index: u8,
    pub new_authority: Pubkey,
}

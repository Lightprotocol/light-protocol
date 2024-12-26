use solana_sdk::{bs58, pubkey::Pubkey};

use super::PhotonClientError;

pub type Address = [u8; 32];
pub type Hash = [u8; 32];

pub struct AddressWithTree {
    pub address: Address,
    pub tree: Pubkey,
}

pub trait Base58Conversions {
    fn to_base58(&self) -> String;
    fn from_base58(s: &str) -> Result<Self, PhotonClientError>
    where
        Self: Sized;
    fn to_bytes(&self) -> [u8; 32];
    fn from_bytes(bytes: &[u8]) -> Result<Self, PhotonClientError>
    where
        Self: Sized;
}

impl Base58Conversions for [u8; 32] {
    fn to_base58(&self) -> String {
        bs58::encode(self).into_string()
    }

    fn from_base58(s: &str) -> Result<Self, PhotonClientError> {
        let bytes = bs58::decode(s)
            .into_vec()
            .map_err(|e| PhotonClientError::DecodeError(e.to_string()))?;

        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(arr)
    }

    fn to_bytes(&self) -> [u8; 32] {
        *self
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, PhotonClientError> {
        let mut arr = [0u8; 32];
        arr.copy_from_slice(bytes);
        Ok(arr)
    }
}

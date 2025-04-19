use solana_sdk::bs58;

use crate::indexer::error::IndexerError;

pub trait Base58Conversions {
    fn to_base58(&self) -> String;
    fn from_base58(s: &str) -> Result<Self, IndexerError>
    where
        Self: Sized;
    fn to_bytes(&self) -> [u8; 32];
    fn from_bytes(bytes: &[u8]) -> Result<Self, IndexerError>
    where
        Self: Sized;
}

impl Base58Conversions for [u8; 32] {
    fn to_base58(&self) -> String {
        bs58::encode(self).into_string()
    }

    fn from_base58(s: &str) -> Result<Self, IndexerError> {
        // TODO: remove vec conversion.
        let result = bs58::decode(s)
            .into_vec()
            .map_err(|e| IndexerError::base58_decode_error(s, e))?
            .try_into()
            .map_err(|_| IndexerError::ApiError("Try into failed.".to_string()))?;

        Ok(result)
    }

    fn to_bytes(&self) -> [u8; 32] {
        *self
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, IndexerError> {
        let mut arr = [0u8; 32];
        arr.copy_from_slice(bytes);
        Ok(arr)
    }
}

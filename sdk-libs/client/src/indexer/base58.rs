use bs58;
use solana_pubkey::Pubkey;

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
        decode_base58_to_fixed_array(s)
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

pub fn decode_base58_to_fixed_array<const N: usize>(input: &str) -> Result<[u8; N], IndexerError> {
    let mut buffer = [0u8; N];
    let decoded_len = bs58::decode(input)
        .onto(&mut buffer)
        .map_err(|_| IndexerError::InvalidResponseData)?;

    if decoded_len != N {
        return Err(IndexerError::InvalidResponseData);
    }

    Ok(buffer)
}

pub fn decode_base58_option_to_pubkey(
    value: &Option<String>,
) -> Result<Option<Pubkey>, IndexerError> {
    value
        .as_ref()
        .map(|ctx| decode_base58_to_fixed_array(ctx).map(Pubkey::new_from_array))
        .transpose()
}

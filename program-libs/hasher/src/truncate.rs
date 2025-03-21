use solana_program::{keccak::hashv, pubkey::Pubkey};

use crate::{bytes::ToByteArray, HasherError};

// TODO: rename to HashTruncate
pub trait Truncate {
    fn truncate(&self) -> Result<[u8; 32], HasherError>;
}

impl<const N: usize> Truncate for [u8; N] {
    fn truncate(&self) -> Result<[u8; 32], HasherError> {
        Ok(hash_to_bn254_field_size_be(self.as_slice()))
    }
}

impl Truncate for String {
    fn truncate(&self) -> Result<[u8; 32], HasherError> {
        Ok(hash_to_bn254_field_size_be(self.as_bytes()))
    }
}

impl Truncate for Pubkey {
    fn truncate(&self) -> Result<[u8; 32], HasherError> {
        Ok(hash_to_bn254_field_size_be(&self.to_bytes()))
    }
}

impl<T> Truncate for Vec<T>
where
    T: ToByteArray,
{
    fn truncate(&self) -> Result<[u8; 32], HasherError> {
        let mut arrays = Vec::with_capacity(self.len());
        for item in self {
            let byte_array = item.to_byte_array()?;
            arrays.push(byte_array);
        }
        let mut slices = Vec::with_capacity(self.len() + 1);
        let bump_seed = [u8::MAX];
        slices.push(bump_seed.as_slice());
        arrays.iter().for_each(|x| slices.push(x.as_slice()));
        Ok(hashv(slices.as_slice()).to_bytes())
    }
}

pub fn hash_to_bn254_field_size_be(bytes: &[u8]) -> [u8; 32] {
    let bump_seed = [u8::MAX];
    let mut hashed_value: [u8; 32] = hashv(&[bytes, bump_seed.as_ref()]).to_bytes();
    // Truncates to 31 bytes so that value is less than bn254 Fr modulo
    // field size.
    hashed_value[0] = 0;
    hashed_value
}

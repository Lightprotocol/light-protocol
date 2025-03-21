use solana_program::{keccak::hashv, pubkey::Pubkey};

use crate::{bytes::ToByteArray, HasherError};

pub trait HashToFieldSize {
    fn hash_to_field_size(&self) -> Result<[u8; 32], HasherError>;
}

impl<const N: usize> HashToFieldSize for [u8; N] {
    fn hash_to_field_size(&self) -> Result<[u8; 32], HasherError> {
        Ok(hash_to_bn254_field_size_be(self.as_slice()))
    }
}

impl HashToFieldSize for String {
    fn hash_to_field_size(&self) -> Result<[u8; 32], HasherError> {
        Ok(hash_to_bn254_field_size_be(self.as_bytes()))
    }
}

impl HashToFieldSize for Pubkey {
    fn hash_to_field_size(&self) -> Result<[u8; 32], HasherError> {
        Ok(hash_to_bn254_field_size_be(&self.to_bytes()))
    }
}

impl<T> HashToFieldSize for Vec<T>
where
    T: ToByteArray,
{
    fn hash_to_field_size(&self) -> Result<[u8; 32], HasherError> {
        let mut arrays = Vec::with_capacity(self.len());
        for item in self {
            let byte_array = item.to_byte_array()?;
            arrays.push(byte_array);
        }
        let mut slices = Vec::with_capacity(self.len() + 1);
        arrays.iter().for_each(|x| slices.push(x.as_slice()));
        let bump_seed = [u8::MAX];
        slices.push(bump_seed.as_slice());
        Ok(hashv(slices.as_slice()).to_bytes())
    }
}

pub fn hashv_to_bn254_field_size_be(bytes: &[[u8; 32]]) -> [u8; 32] {
    // TODO: create a second version of this which uses ArrayVec.
    // - priority low since users use a Vec anyway.
    let mut slices = Vec::with_capacity(bytes.len() + 1);
    bytes.iter().for_each(|x| slices.push(x.as_slice()));
    let bump_seed = [u8::MAX];
    slices.push(bump_seed.as_slice());
    let mut hashed_value: [u8; 32] = hashv(&slices).to_bytes();
    // Truncates to 31 bytes so that value is less than bn254 Fr modulo
    // field size.
    hashed_value[0] = 0;
    hashed_value
}

pub fn hash_to_bn254_field_size_be(bytes: &[u8]) -> [u8; 32] {
    let bump_seed = [u8::MAX];
    let mut hashed_value: [u8; 32] = hashv(&[bytes, bump_seed.as_ref()]).to_bytes();
    // Truncates to 31 bytes so that value is less than bn254 Fr modulo
    // field size.
    hashed_value[0] = 0;
    hashed_value
}

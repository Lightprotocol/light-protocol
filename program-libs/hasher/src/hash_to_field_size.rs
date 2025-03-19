use arrayvec::ArrayVec;

use crate::{keccak::Keccak, to_byte_array::ToByteArray, Hasher, HasherError};

pub const HASH_TO_FIELD_SIZE_SEED: u8 = u8::MAX;

pub trait HashToFieldSize {
    fn hash_to_field_size(&self) -> Result<[u8; 32], HasherError>;
}

impl<const N: usize> HashToFieldSize for [u8; N] {
    fn hash_to_field_size(&self) -> Result<[u8; 32], HasherError> {
        hash_to_bn254_field_size_be(self.as_slice())
    }
}

impl HashToFieldSize for String {
    fn hash_to_field_size(&self) -> Result<[u8; 32], HasherError> {
        hash_to_bn254_field_size_be(self.as_bytes())
    }
}

#[cfg(feature = "solana")]
impl HashToFieldSize for solana_program::pubkey::Pubkey {
    fn hash_to_field_size(&self) -> Result<[u8; 32], HasherError> {
        hash_to_bn254_field_size_be(&self.to_bytes())
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
        let bump_seed = [HASH_TO_FIELD_SIZE_SEED];
        slices.push(bump_seed.as_slice());
        Keccak::hashv(slices.as_slice())
    }
}

pub fn hashv_to_bn254_field_size_be(bytes: &[&[u8]]) -> Result<[u8; 32], HasherError> {
    let mut slices = Vec::with_capacity(bytes.len() + 1);
    bytes.iter().for_each(|x| slices.push(*x));
    let bump_seed = [HASH_TO_FIELD_SIZE_SEED];
    slices.push(bump_seed.as_slice());
    let mut hashed_value: [u8; 32] = Keccak::hashv(&slices)?;
    // Truncates to 31 bytes so that value is less than bn254 Fr modulo
    // field size.
    hashed_value[0] = 0;
    Ok(hashed_value)
}

pub fn hashv_to_bn254_field_size_be_array(bytes: &[[u8; 32]]) -> Result<[u8; 32], HasherError> {
    let mut slices = Vec::with_capacity(bytes.len() + 1);
    bytes.iter().for_each(|x| slices.push(x.as_slice()));
    let bump_seed = [HASH_TO_FIELD_SIZE_SEED];
    slices.push(bump_seed.as_slice());
    let mut hashed_value: [u8; 32] = Keccak::hashv(&slices)?;
    // Truncates to 31 bytes so that value is less than bn254 Fr modulo
    // field size.
    hashed_value[0] = 0;
    Ok(hashed_value)
}

/// MAX_SLICES - 1 is usable.
pub fn hashv_to_bn254_field_size_be_const_array<const MAX_SLICES: usize>(
    bytes: &[&[u8]],
) -> Result<[u8; 32], HasherError> {
    let bump_seed = [HASH_TO_FIELD_SIZE_SEED];
    let mut slices = ArrayVec::<&[u8], MAX_SLICES>::new();
    if bytes.len() > MAX_SLICES - 1 {
        return Err(HasherError::InvalidInputLength(MAX_SLICES, bytes.len()));
    }
    bytes.iter().for_each(|x| slices.push(x));
    slices.push(bump_seed.as_slice());
    let mut hashed_value: [u8; 32] = Keccak::hashv(&slices)?;
    // Truncates to 31 bytes so that value is less than bn254 Fr modulo
    // field size.
    hashed_value[0] = 0;
    Ok(hashed_value)
}

/// Hashes the provided `bytes` with Keccak256 and ensures the result fits
/// in the BN254 field by truncating the resulting hash to 31 bytes.
///
/// # Examples
///
/// ```
/// use light_hasher::hash_to_field_size::hashv_to_bn254_field_size_be;
///
/// hashv_to_bn254_field_size_be(&[&[0u8;32][..]]);
/// ```
pub fn hash_to_bn254_field_size_be(bytes: &[u8]) -> Result<[u8; 32], HasherError> {
    let bump_seed = [HASH_TO_FIELD_SIZE_SEED];
    let mut hashed_value: [u8; 32] = Keccak::hashv(&[bytes, bump_seed.as_ref()])?;
    // Truncates to 31 bytes so that value is less than bn254 Fr modulo
    // field size.
    hashed_value[0] = 0;
    Ok(hashed_value)
}

#[cfg(not(target_os = "solana"))]
pub fn is_smaller_than_bn254_field_size_be(bytes: &[u8; 32]) -> bool {
    use ark_ff::PrimeField;
    use num_bigint::BigUint;
    let bigint = BigUint::from_bytes_be(bytes);
    bigint < ark_bn254::Fr::MODULUS.into()
}

#[cfg(test)]
mod tests {
    use ark_ff::PrimeField;
    use num_bigint::{BigUint, ToBigUint};

    use super::*;
    use crate::bigint::bigint_to_be_bytes_array;

    #[test]
    fn test_is_smaller_than_bn254_field_size_be() {
        let modulus: BigUint = ark_bn254::Fr::MODULUS.into();
        let modulus_bytes: [u8; 32] = bigint_to_be_bytes_array(&modulus).unwrap();
        assert!(!is_smaller_than_bn254_field_size_be(&modulus_bytes));

        let bigint = modulus.clone() - 1.to_biguint().unwrap();
        let bigint_bytes: [u8; 32] = bigint_to_be_bytes_array(&bigint).unwrap();
        assert!(is_smaller_than_bn254_field_size_be(&bigint_bytes));

        let bigint = modulus + 1.to_biguint().unwrap();
        let bigint_bytes: [u8; 32] = bigint_to_be_bytes_array(&bigint).unwrap();
        assert!(!is_smaller_than_bn254_field_size_be(&bigint_bytes));
    }

    #[cfg(feature = "solana")]
    #[test]
    fn test_hash_to_bn254_field_size_be() {
        use solana_program::pubkey::Pubkey;
        for _ in 0..10_000 {
            let input_bytes = Pubkey::new_unique().to_bytes(); // Sample input
            let hashed_value = hash_to_bn254_field_size_be(input_bytes.as_slice()).unwrap();
            assert!(
                is_smaller_than_bn254_field_size_be(&hashed_value),
                "Hashed value should be within BN254 field size"
            );
        }

        let max_input = [u8::MAX; 32];
        let hashed_value = hash_to_bn254_field_size_be(max_input.as_slice()).unwrap();
        assert!(
            is_smaller_than_bn254_field_size_be(&hashed_value),
            "Hashed value should be within BN254 field size"
        );
    }

    #[cfg(feature = "solana")]
    #[test]
    fn test_hashv_to_bn254_field_size_be() {
        use solana_program::pubkey::Pubkey;
        for _ in 0..10_000 {
            let input_bytes = [Pubkey::new_unique().to_bytes(); 4];
            let input_bytes = input_bytes.iter().map(|x| x.as_slice()).collect::<Vec<_>>();
            let hashed_value = hashv_to_bn254_field_size_be(input_bytes.as_slice()).unwrap();
            assert!(
                is_smaller_than_bn254_field_size_be(&hashed_value),
                "Hashed value should be within BN254 field size"
            );
        }

        let max_input = [[u8::MAX; 32]; 16];
        let max_input = max_input.iter().map(|x| x.as_slice()).collect::<Vec<_>>();
        let hashed_value = hashv_to_bn254_field_size_be(max_input.as_slice()).unwrap();
        assert!(
            is_smaller_than_bn254_field_size_be(&hashed_value),
            "Hashed value should be within BN254 field size"
        );
    }
}

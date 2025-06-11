use arrayvec::ArrayVec;
use borsh::BorshSerialize;

use crate::{keccak::Keccak, Hasher, HasherError};

pub const HASH_TO_FIELD_SIZE_SEED: u8 = u8::MAX;

pub trait HashToFieldSize {
    fn hash_to_field_size(&self) -> Result<[u8; 32], HasherError>;
}

impl<T> HashToFieldSize for T
where
    T: BorshSerialize,
{
    fn hash_to_field_size(&self) -> Result<[u8; 32], HasherError> {
        let borsh_vec = self.try_to_vec().map_err(|_| HasherError::BorshError)?;
        #[cfg(debug_assertions)]
        {
            if std::env::var("RUST_BACKTRACE").is_ok() {
                println!(
                    "#[hash] hash_to_field_size borsh try_to_vec {:?}",
                    borsh_vec
                );
            }
        }
        let bump_seed = [HASH_TO_FIELD_SIZE_SEED];
        let slices = [borsh_vec.as_slice(), bump_seed.as_slice()];
        // SAFETY: cannot panic Hasher::hashv returns an error because Poseidon can panic.
        let mut hashed_value = Keccak::hashv(slices.as_slice()).unwrap();
        // Truncates to 31 bytes so that value is less than bn254 Fr modulo
        // field size.
        hashed_value[0] = 0;
        Ok(hashed_value)
    }
}

pub fn hashv_to_bn254_field_size_be(bytes: &[&[u8]]) -> [u8; 32] {
    let mut slices = Vec::with_capacity(bytes.len() + 1);
    bytes.iter().for_each(|x| slices.push(*x));
    let bump_seed = [HASH_TO_FIELD_SIZE_SEED];
    slices.push(bump_seed.as_slice());
    // SAFETY: cannot panic Hasher::hashv returns an error because Poseidon can panic.
    let mut hashed_value: [u8; 32] = Keccak::hashv(&slices).unwrap();
    // Truncates to 31 bytes so that value is less than bn254 Fr modulo
    // field size.
    hashed_value[0] = 0;
    hashed_value
}

pub fn hashv_to_bn254_field_size_be_array(bytes: &[[u8; 32]]) -> [u8; 32] {
    let mut slices = Vec::with_capacity(bytes.len() + 1);
    bytes.iter().for_each(|x| slices.push(x.as_slice()));
    let bump_seed = [HASH_TO_FIELD_SIZE_SEED];
    slices.push(bump_seed.as_slice());
    let mut hashed_value: [u8; 32] = Keccak::hashv(&slices).unwrap();
    // Truncates to 31 bytes so that value is less than bn254 Fr modulo
    // field size.
    hashed_value[0] = 0;
    hashed_value
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
pub fn hash_to_bn254_field_size_be(bytes: &[u8]) -> [u8; 32] {
    hashv_to_bn254_field_size_be_const_array::<2>(&[bytes]).unwrap()
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
    use borsh::BorshDeserialize;
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

    #[test]
    fn hash_to_field_size_borsh() {
        #[derive(BorshSerialize, BorshDeserialize)]
        pub struct TestStruct {
            a: u32,
            b: u32,
            c: u64,
        }
        let test_struct = TestStruct { a: 1, b: 2, c: 3 };
        let serialized = test_struct.try_to_vec().unwrap();
        let hash = test_struct.hash_to_field_size().unwrap();
        let manual_hash = hash_to_bn254_field_size_be(&serialized);
        assert_eq!(hash, manual_hash);
    }

    #[cfg(feature = "solana")]
    #[test]
    fn test_hash_to_bn254_field_size_be() {
        use solana_pubkey::Pubkey;
        for _ in 0..10_000 {
            let input_bytes = Pubkey::new_unique().to_bytes(); // Sample input
            let hashed_value = hash_to_bn254_field_size_be(input_bytes.as_slice());
            assert!(
                is_smaller_than_bn254_field_size_be(&hashed_value),
                "Hashed value should be within BN254 field size"
            );
        }

        let max_input = [u8::MAX; 32];
        let hashed_value = hash_to_bn254_field_size_be(max_input.as_slice());
        assert!(
            is_smaller_than_bn254_field_size_be(&hashed_value),
            "Hashed value should be within BN254 field size"
        );
    }

    #[cfg(feature = "solana")]
    #[test]
    fn test_hashv_to_bn254_field_size_be() {
        use solana_pubkey::Pubkey;
        for _ in 0..10_000 {
            let input_bytes = [Pubkey::new_unique().to_bytes(); 4];
            let input_bytes = input_bytes.iter().map(|x| x.as_slice()).collect::<Vec<_>>();
            let hashed_value = hashv_to_bn254_field_size_be(input_bytes.as_slice());
            assert!(
                is_smaller_than_bn254_field_size_be(&hashed_value),
                "Hashed value should be within BN254 field size"
            );
        }

        let max_input = [[u8::MAX; 32]; 16];
        let max_input = max_input.iter().map(|x| x.as_slice()).collect::<Vec<_>>();
        let hashed_value = hashv_to_bn254_field_size_be(max_input.as_slice());
        assert!(
            is_smaller_than_bn254_field_size_be(&hashed_value),
            "Hashed value should be within BN254 field size"
        );
    }
}

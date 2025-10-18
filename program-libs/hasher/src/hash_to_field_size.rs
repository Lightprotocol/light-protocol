#[cfg(feature = "alloc")]
use borsh::BorshSerialize;
use tinyvec::ArrayVec;

#[cfg(feature = "alloc")]
use crate::Vec;
use crate::{keccak::Keccak, Hasher, HasherError};

pub const HASH_TO_FIELD_SIZE_SEED: u8 = u8::MAX;

pub trait HashToFieldSize {
    fn hash_to_field_size(&self) -> Result<[u8; 32], HasherError>;
}

#[cfg(feature = "alloc")]
impl<T> HashToFieldSize for T
where
    T: BorshSerialize,
{
    fn hash_to_field_size(&self) -> Result<[u8; 32], HasherError> {
        let borsh_vec = self.try_to_vec().map_err(|_| HasherError::BorshError)?;
        #[cfg(all(debug_assertions, feature = "std"))]
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
        // Keccak::hashv is fallible (trait-unified), propagate errors instead of panicking.
        let mut hashed_value = Keccak::hashv(slices.as_slice())?;
        // Truncates to 31 bytes so that value is less than bn254 Fr modulo
        // field size.
        hashed_value[0] = 0;
        Ok(hashed_value)
    }
}

#[cfg(feature = "alloc")]
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

#[cfg(feature = "alloc")]
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
    let mut slices = ArrayVec::<[&[u8]; MAX_SLICES]>::new();
    if bytes.len() > MAX_SLICES - 1 {
        return Err(HasherError::InvalidInputLength(MAX_SLICES, bytes.len()));
    }
    bytes.iter().for_each(|x| slices.push(*x));
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
/// # #[cfg(feature = "keccak")]
/// # use light_hasher::hash_to_field_size::hash_to_bn254_field_size_be;
/// #
/// # #[cfg(feature = "keccak")]
/// hash_to_bn254_field_size_be(&[0u8; 32]);
/// ```
pub fn hash_to_bn254_field_size_be(bytes: &[u8]) -> [u8; 32] {
    hashv_to_bn254_field_size_be_const_array::<2>(&[bytes]).unwrap()
}

#[cfg(all(not(target_os = "solana"), feature = "poseidon"))]
pub fn is_smaller_than_bn254_field_size_be(bytes: &[u8; 32]) -> bool {
    use ark_ff::PrimeField;
    use num_bigint::BigUint;
    let bigint = BigUint::from_bytes_be(bytes);
    bigint < ark_bn254::Fr::MODULUS.into()
}

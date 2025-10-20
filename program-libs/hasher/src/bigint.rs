use num_bigint::BigUint;

use crate::HasherError;

/// Converts the given [`num_bigint::BigUint`](num_bigint::BigUint) into a little-endian
/// byte array.
pub fn bigint_to_le_bytes_array<const BYTES_SIZE: usize>(
    bigint: &BigUint,
) -> Result<[u8; BYTES_SIZE], HasherError> {
    let mut array = [0u8; BYTES_SIZE];
    let bytes = bigint.to_bytes_le();

    if bytes.len() > BYTES_SIZE {
        return Err(HasherError::InvalidInputLength(BYTES_SIZE, bytes.len()));
    }

    array[..bytes.len()].copy_from_slice(bytes.as_slice());
    Ok(array)
}

/// Converts the given [`ark_ff::BigUint`](ark_ff::BigUint) into a big-endian
/// byte array.
pub fn bigint_to_be_bytes_array<const BYTES_SIZE: usize>(
    bigint: &BigUint,
) -> Result<[u8; BYTES_SIZE], HasherError> {
    let mut array = [0u8; BYTES_SIZE];
    let bytes = bigint.to_bytes_be();

    if bytes.len() > BYTES_SIZE {
        return Err(HasherError::InvalidInputLength(BYTES_SIZE, bytes.len()));
    }

    let start_pos = BYTES_SIZE - bytes.len();
    array[start_pos..].copy_from_slice(bytes.as_slice());
    Ok(array)
}

use num_bigint::BigUint;

use crate::UtilsError;

/// Converts the given [`num_bigint::BigUint`](num_bigint::BigUint) into a little-endian
/// byte array.
pub fn bigint_to_le_bytes_array<const BYTES_SIZE: usize>(
    bigint: &BigUint,
) -> Result<[u8; BYTES_SIZE], UtilsError> {
    let mut array = [0u8; BYTES_SIZE];
    let bytes = bigint.to_bytes_le();

    if bytes.len() > BYTES_SIZE {
        return Err(UtilsError::InputTooLarge(BYTES_SIZE));
    }

    array[..bytes.len()].copy_from_slice(bytes.as_slice());
    Ok(array)
}

/// Converts the given [`ark_ff::BigUint`](ark_ff::BigUint) into a big-endian
/// byte array.
pub fn bigint_to_be_bytes_array<const BYTES_SIZE: usize>(
    bigint: &BigUint,
) -> Result<[u8; BYTES_SIZE], UtilsError> {
    let mut array = [0u8; BYTES_SIZE];
    let bytes = bigint.to_bytes_be();

    if bytes.len() > BYTES_SIZE {
        return Err(UtilsError::InputTooLarge(BYTES_SIZE));
    }

    let start_pos = BYTES_SIZE - bytes.len();
    array[start_pos..].copy_from_slice(bytes.as_slice());
    Ok(array)
}

#[cfg(test)]
mod test {
    use num_bigint::{RandBigInt, ToBigUint};
    use rand::thread_rng;

    use super::*;

    const ITERATIONS: usize = 64;

    #[test]
    fn test_bigint_conversion_rand() {
        let mut rng = thread_rng();

        for _ in 0..ITERATIONS {
            let b64 = rng.gen_biguint(32);
            let b64_converted: [u8; 8] = bigint_to_be_bytes_array(&b64).unwrap();
            let b64_converted = BigUint::from_bytes_be(&b64_converted);
            assert_eq!(b64, b64_converted);
            let b64_converted: [u8; 8] = bigint_to_le_bytes_array(&b64).unwrap();
            let b64_converted = BigUint::from_bytes_le(&b64_converted);
            assert_eq!(b64, b64_converted);

            let b128 = rng.gen_biguint(128);
            let b128_converted: [u8; 16] = bigint_to_be_bytes_array(&b128).unwrap();
            let b128_converted = BigUint::from_bytes_be(&b128_converted);
            assert_eq!(b128, b128_converted);
            let b128_converted: [u8; 16] = bigint_to_le_bytes_array(&b128).unwrap();
            let b128_converted = BigUint::from_bytes_le(&b128_converted);
            assert_eq!(b128, b128_converted);

            let b256 = rng.gen_biguint(256);
            let b256_converted: [u8; 32] = bigint_to_be_bytes_array(&b256).unwrap();
            let b256_converted = BigUint::from_bytes_be(&b256_converted);
            assert_eq!(b256, b256_converted);
            let b256_converted: [u8; 32] = bigint_to_le_bytes_array(&b256).unwrap();
            let b256_converted = BigUint::from_bytes_le(&b256_converted);
            assert_eq!(b256, b256_converted);

            let b320 = rng.gen_biguint(320);
            let b320_converted: [u8; 40] = bigint_to_be_bytes_array(&b320).unwrap();
            let b320_converted = BigUint::from_bytes_be(&b320_converted);
            assert_eq!(b320, b320_converted);
            let b320_converted: [u8; 40] = bigint_to_le_bytes_array(&b320).unwrap();
            let b320_converted = BigUint::from_bytes_le(&b320_converted);
            assert_eq!(b320, b320_converted);

            let b384 = rng.gen_biguint(384);
            let b384_converted: [u8; 48] = bigint_to_be_bytes_array(&b384).unwrap();
            let b384_converted = BigUint::from_bytes_be(&b384_converted);
            assert_eq!(b384, b384_converted);
            let b384_converted: [u8; 48] = bigint_to_le_bytes_array(&b384).unwrap();
            let b384_converted = BigUint::from_bytes_le(&b384_converted);
            assert_eq!(b384, b384_converted);

            let b448 = rng.gen_biguint(448);
            let b448_converted: [u8; 56] = bigint_to_be_bytes_array(&b448).unwrap();
            let b448_converted = BigUint::from_bytes_be(&b448_converted);
            assert_eq!(b448, b448_converted);
            let b448_converted: [u8; 56] = bigint_to_le_bytes_array(&b448).unwrap();
            let b448_converted = BigUint::from_bytes_le(&b448_converted);
            assert_eq!(b448, b448_converted);

            let b768 = rng.gen_biguint(768);
            let b768_converted: [u8; 96] = bigint_to_be_bytes_array(&b768).unwrap();
            let b768_converted = BigUint::from_bytes_be(&b768_converted);
            assert_eq!(b768, b768_converted);
            let b768_converted: [u8; 96] = bigint_to_le_bytes_array(&b768).unwrap();
            let b768_converted = BigUint::from_bytes_le(&b768_converted);
            assert_eq!(b768, b768_converted);

            let b832 = rng.gen_biguint(832);
            let b832_converted: [u8; 104] = bigint_to_be_bytes_array(&b832).unwrap();
            let b832_converted = BigUint::from_bytes_be(&b832_converted);
            assert_eq!(b832, b832_converted);
            let b832_converted: [u8; 104] = bigint_to_le_bytes_array(&b832).unwrap();
            let b832_converted = BigUint::from_bytes_le(&b832_converted);
            assert_eq!(b832, b832_converted);
        }
    }

    #[test]
    fn test_bigint_conversion_zero() {
        let zero = 0_u32.to_biguint().unwrap();

        let b64_converted: [u8; 8] = bigint_to_be_bytes_array(&zero).unwrap();
        let b64_converted = BigUint::from_bytes_be(&b64_converted);
        assert_eq!(zero, b64_converted);
        let b64_converted: [u8; 8] = bigint_to_le_bytes_array(&zero).unwrap();
        let b64_converted = BigUint::from_bytes_le(&b64_converted);
        assert_eq!(zero, b64_converted);

        let b128_converted: [u8; 16] = bigint_to_be_bytes_array(&zero).unwrap();
        let b128_converted = BigUint::from_bytes_be(&b128_converted);
        assert_eq!(zero, b128_converted);
        let b128_converted: [u8; 16] = bigint_to_le_bytes_array(&zero).unwrap();
        let b128_converted = BigUint::from_bytes_le(&b128_converted);
        assert_eq!(zero, b128_converted);

        let b256_converted: [u8; 32] = bigint_to_be_bytes_array(&zero).unwrap();
        let b256_converted = BigUint::from_bytes_be(&b256_converted);
        assert_eq!(zero, b256_converted);
        let b256_converted: [u8; 32] = bigint_to_le_bytes_array(&zero).unwrap();
        let b256_converted = BigUint::from_bytes_le(&b256_converted);
        assert_eq!(zero, b256_converted);

        let b320_converted: [u8; 40] = bigint_to_be_bytes_array(&zero).unwrap();
        let b320_converted = BigUint::from_bytes_be(&b320_converted);
        assert_eq!(zero, b320_converted);
        let b320_converted: [u8; 40] = bigint_to_le_bytes_array(&zero).unwrap();
        let b320_converted = BigUint::from_bytes_le(&b320_converted);
        assert_eq!(zero, b320_converted);

        let b384_converted: [u8; 48] = bigint_to_be_bytes_array(&zero).unwrap();
        let b384_converted = BigUint::from_bytes_be(&b384_converted);
        assert_eq!(zero, b384_converted);
        let b384_converted: [u8; 48] = bigint_to_le_bytes_array(&zero).unwrap();
        let b384_converted = BigUint::from_bytes_le(&b384_converted);
        assert_eq!(zero, b384_converted);

        let b448_converted: [u8; 56] = bigint_to_be_bytes_array(&zero).unwrap();
        let b448_converted = BigUint::from_bytes_be(&b448_converted);
        assert_eq!(zero, b448_converted);
        let b448_converted: [u8; 56] = bigint_to_le_bytes_array(&zero).unwrap();
        let b448_converted = BigUint::from_bytes_le(&b448_converted);
        assert_eq!(zero, b448_converted);

        let b768_converted: [u8; 96] = bigint_to_be_bytes_array(&zero).unwrap();
        let b768_converted = BigUint::from_bytes_be(&b768_converted);
        assert_eq!(zero, b768_converted);
        let b768_converted: [u8; 96] = bigint_to_le_bytes_array(&zero).unwrap();
        let b768_converted = BigUint::from_bytes_le(&b768_converted);
        assert_eq!(zero, b768_converted);

        let b832_converted: [u8; 104] = bigint_to_be_bytes_array(&zero).unwrap();
        let b832_converted = BigUint::from_bytes_be(&b832_converted);
        assert_eq!(zero, b832_converted);
        let b832_converted: [u8; 104] = bigint_to_le_bytes_array(&zero).unwrap();
        let b832_converted = BigUint::from_bytes_le(&b832_converted);
        assert_eq!(zero, b832_converted);
    }

    #[test]
    fn test_bigint_conversion_one() {
        let one = 1_u32.to_biguint().unwrap();

        let b64_converted: [u8; 8] = bigint_to_be_bytes_array(&one).unwrap();
        let b64_converted = BigUint::from_bytes_be(&b64_converted);
        assert_eq!(one, b64_converted);
        let b64_converted: [u8; 8] = bigint_to_le_bytes_array(&one).unwrap();
        let b64_converted = BigUint::from_bytes_le(&b64_converted);
        assert_eq!(one, b64_converted);
        let b64 = BigUint::from_bytes_be(&[0, 0, 0, 0, 0, 0, 0, 1]);
        assert_eq!(one, b64);
        let b64 = BigUint::from_bytes_le(&[1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(one, b64);

        let b128_converted: [u8; 16] = bigint_to_be_bytes_array(&one).unwrap();
        let b128_converted = BigUint::from_bytes_be(&b128_converted);
        assert_eq!(one, b128_converted);
        let b128_converted: [u8; 16] = bigint_to_le_bytes_array(&one).unwrap();
        let b128_converted = BigUint::from_bytes_le(&b128_converted);
        assert_eq!(one, b128_converted);

        let b256_converted: [u8; 32] = bigint_to_be_bytes_array(&one).unwrap();
        let b256_converted = BigUint::from_bytes_be(&b256_converted);
        assert_eq!(one, b256_converted);
        let b256_converted: [u8; 32] = bigint_to_le_bytes_array(&one).unwrap();
        let b256_converted = BigUint::from_bytes_le(&b256_converted);
        assert_eq!(one, b256_converted);

        let b320_converted: [u8; 40] = bigint_to_be_bytes_array(&one).unwrap();
        let b320_converted = BigUint::from_bytes_be(&b320_converted);
        assert_eq!(one, b320_converted);
        let b320_converted: [u8; 40] = bigint_to_le_bytes_array(&one).unwrap();
        let b320_converted = BigUint::from_bytes_le(&b320_converted);
        assert_eq!(one, b320_converted);

        let b384_converted: [u8; 48] = bigint_to_be_bytes_array(&one).unwrap();
        let b384_converted = BigUint::from_bytes_be(&b384_converted);
        assert_eq!(one, b384_converted);
        let b384_converted: [u8; 48] = bigint_to_le_bytes_array(&one).unwrap();
        let b384_converted = BigUint::from_bytes_le(&b384_converted);
        assert_eq!(one, b384_converted);

        let b448_converted: [u8; 56] = bigint_to_be_bytes_array(&one).unwrap();
        let b448_converted = BigUint::from_bytes_be(&b448_converted);
        assert_eq!(one, b448_converted);
        let b448_converted: [u8; 56] = bigint_to_le_bytes_array(&one).unwrap();
        let b448_converted = BigUint::from_bytes_le(&b448_converted);
        assert_eq!(one, b448_converted);

        let b768_converted: [u8; 96] = bigint_to_be_bytes_array(&one).unwrap();
        let b768_converted = BigUint::from_bytes_be(&b768_converted);
        assert_eq!(one, b768_converted);
        let b768_converted: [u8; 96] = bigint_to_le_bytes_array(&one).unwrap();
        let b768_converted = BigUint::from_bytes_le(&b768_converted);
        assert_eq!(one, b768_converted);

        let b832_converted: [u8; 104] = bigint_to_be_bytes_array(&one).unwrap();
        let b832_converted = BigUint::from_bytes_be(&b832_converted);
        assert_eq!(one, b832_converted);
        let b832_converted: [u8; 104] = bigint_to_le_bytes_array(&one).unwrap();
        let b832_converted = BigUint::from_bytes_le(&b832_converted);
        assert_eq!(one, b832_converted);
    }

    #[test]
    fn test_bigint_conversion_invalid_size() {
        let mut rng = thread_rng();

        let b64 = rng.gen_biguint(64);
        let res: Result<[u8; 1], UtilsError> = bigint_to_be_bytes_array(&b64);
        assert!(matches!(res, Err(UtilsError::InputTooLarge(1))));
        let res: Result<[u8; 7], UtilsError> = bigint_to_be_bytes_array(&b64);
        assert!(matches!(res, Err(UtilsError::InputTooLarge(7))));
        let res: Result<[u8; 9], UtilsError> = bigint_to_be_bytes_array(&b64);
        assert!(res.is_ok());

        let b128 = rng.gen_biguint(128);
        let res: Result<[u8; 1], UtilsError> = bigint_to_be_bytes_array(&b128);
        assert!(matches!(res, Err(UtilsError::InputTooLarge(1))));
        let res: Result<[u8; 15], UtilsError> = bigint_to_be_bytes_array(&b128);
        assert!(matches!(res, Err(UtilsError::InputTooLarge(15))));
        let res: Result<[u8; 17], UtilsError> = bigint_to_be_bytes_array(&b128);
        assert!(res.is_ok());

        let b256 = rng.gen_biguint(256);
        let res: Result<[u8; 1], UtilsError> = bigint_to_be_bytes_array(&b256);
        assert!(matches!(res, Err(UtilsError::InputTooLarge(1))));
        let res: Result<[u8; 31], UtilsError> = bigint_to_be_bytes_array(&b256);
        assert!(matches!(res, Err(UtilsError::InputTooLarge(31))));
        let res: Result<[u8; 33], UtilsError> = bigint_to_be_bytes_array(&b256);
        assert!(res.is_ok());

        let b320 = rng.gen_biguint(320);
        let res: Result<[u8; 1], UtilsError> = bigint_to_be_bytes_array(&b320);
        assert!(matches!(res, Err(UtilsError::InputTooLarge(1))));
        let res: Result<[u8; 39], UtilsError> = bigint_to_be_bytes_array(&b320);
        assert!(matches!(res, Err(UtilsError::InputTooLarge(39))));
        let res: Result<[u8; 41], UtilsError> = bigint_to_be_bytes_array(&b320);
        assert!(res.is_ok());

        let b384 = rng.gen_biguint(384);
        let res: Result<[u8; 1], UtilsError> = bigint_to_be_bytes_array(&b384);
        assert!(matches!(res, Err(UtilsError::InputTooLarge(1))));
        let res: Result<[u8; 47], UtilsError> = bigint_to_be_bytes_array(&b384);
        assert!(matches!(res, Err(UtilsError::InputTooLarge(47))));
        let res: Result<[u8; 49], UtilsError> = bigint_to_be_bytes_array(&b384);
        assert!(res.is_ok());

        let b448 = rng.gen_biguint(448);
        let res: Result<[u8; 1], UtilsError> = bigint_to_be_bytes_array(&b448);
        assert!(matches!(res, Err(UtilsError::InputTooLarge(1))));
        let res: Result<[u8; 55], UtilsError> = bigint_to_be_bytes_array(&b448);
        assert!(matches!(res, Err(UtilsError::InputTooLarge(55))));
        let res: Result<[u8; 57], UtilsError> = bigint_to_be_bytes_array(&b448);
        assert!(res.is_ok());

        let b768 = rng.gen_biguint(768);
        let res: Result<[u8; 1], UtilsError> = bigint_to_be_bytes_array(&b768);
        assert!(matches!(res, Err(UtilsError::InputTooLarge(1))));
        let res: Result<[u8; 95], UtilsError> = bigint_to_be_bytes_array(&b768);
        assert!(matches!(res, Err(UtilsError::InputTooLarge(95))));
        let res: Result<[u8; 97], UtilsError> = bigint_to_be_bytes_array(&b768);
        assert!(res.is_ok());

        let b832 = rng.gen_biguint(832);
        let res: Result<[u8; 1], UtilsError> = bigint_to_be_bytes_array(&b832);
        assert!(matches!(res, Err(UtilsError::InputTooLarge(1))));
        let res: Result<[u8; 103], UtilsError> = bigint_to_be_bytes_array(&b832);
        assert!(matches!(res, Err(UtilsError::InputTooLarge(103))));
        let res: Result<[u8; 105], UtilsError> = bigint_to_be_bytes_array(&b832);
        assert!(res.is_ok());
    }
}

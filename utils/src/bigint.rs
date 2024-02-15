use std::mem;

use ark_ff::BigInt;

use crate::UtilsError;

/// Converts the given [`ark_ff::BigInt`](ark_ff::BigInt) into a big-endian
/// byte array.
pub fn bigint_to_be_bytes<const BYTES_SIZE: usize, const NUM_LIMBS: usize>(
    bigint: &BigInt<NUM_LIMBS>,
) -> Result<[u8; BYTES_SIZE], UtilsError> {
    let mut bytes = [0u8; BYTES_SIZE];
    let limb_size = mem::size_of::<u64>();

    if BYTES_SIZE != NUM_LIMBS * limb_size {
        return Err(UtilsError::InvalidInputSize(
            NUM_LIMBS * limb_size,
            BYTES_SIZE,
        ));
    }

    // Iterate over the limbs in reverse order - limbs are little-endian.
    for (i, limb) in bigint.0.iter().enumerate().rev() {
        let start_index = BYTES_SIZE - (i + 1) * limb_size;
        bytes[start_index..start_index + limb_size].copy_from_slice(&limb.to_be_bytes());
    }

    Ok(bytes)
}

/// Converts the given [`ark_ff::BigInt`](ark_ff::BigInt) into a little-endian
/// byte array.
pub fn bigint_to_le_bytes<const BYTES_SIZE: usize, const NUM_LIMBS: usize>(
    bigint: &BigInt<NUM_LIMBS>,
) -> Result<[u8; BYTES_SIZE], UtilsError> {
    let mut bytes = [0u8; BYTES_SIZE];
    let limb_size = mem::size_of::<u64>();

    if BYTES_SIZE != NUM_LIMBS * limb_size {
        return Err(UtilsError::InvalidInputSize(
            NUM_LIMBS * limb_size,
            BYTES_SIZE,
        ));
    }

    for (i, limb) in bigint.0.iter().enumerate() {
        bytes[i * limb_size..(i + 1) * limb_size].copy_from_slice(&limb.to_le_bytes());
    }

    Ok(bytes)
}

/// Converts the given big-endian byte slice into
/// [`ark_ff::BigInt`](`ark_ff::BigInt`).
pub fn be_bytes_to_bigint<const BYTES_SIZE: usize, const NUM_LIMBS: usize>(
    bytes: &[u8; BYTES_SIZE],
) -> Result<BigInt<NUM_LIMBS>, UtilsError> {
    let mut bytes = *bytes;
    bytes.reverse();
    le_bytes_to_bigint(&bytes)
}

/// Converts the given little-endian byte slice into
/// [`ark_ff::BigInt`](`ark_ff::BigInt`).
pub fn le_bytes_to_bigint<const BYTES_SIZE: usize, const NUM_LIMBS: usize>(
    bytes: &[u8; BYTES_SIZE],
) -> Result<BigInt<NUM_LIMBS>, UtilsError> {
    let expected_size = NUM_LIMBS * mem::size_of::<u64>();
    if BYTES_SIZE != expected_size {
        return Err(UtilsError::InvalidInputSize(expected_size, BYTES_SIZE));
    }

    let mut bigint: BigInt<NUM_LIMBS> = BigInt::zero();
    for (i, chunk) in bytes.chunks(mem::size_of::<u64>()).enumerate() {
        bigint.0[i] =
            u64::from_le_bytes(chunk.try_into().map_err(|_| UtilsError::InvalidChunkSize)?);
    }

    Ok(bigint)
}

#[cfg(test)]
mod test {
    use ark_ff::{
        BigInteger128, BigInteger256, BigInteger320, BigInteger384, BigInteger448, BigInteger64,
        BigInteger768, BigInteger832, UniformRand,
    };
    use rand::thread_rng;

    use super::*;

    const ITERATIONS: usize = 64;

    #[test]
    fn test_bigint_conversion_rand() {
        let mut rng = thread_rng();

        for _ in 0..ITERATIONS {
            let b64 = BigInteger64::rand(&mut rng);
            let b64_converted: [u8; 8] = bigint_to_be_bytes(&b64).unwrap();
            let b64_converted: BigInteger64 = be_bytes_to_bigint(&b64_converted).unwrap();
            assert_eq!(b64, b64_converted);
            let b64_converted: [u8; 8] = bigint_to_le_bytes(&b64).unwrap();
            let b64_converted: BigInteger64 = le_bytes_to_bigint(&b64_converted).unwrap();
            assert_eq!(b64, b64_converted);

            let b128 = BigInteger128::rand(&mut rng);
            let b128_converted: [u8; 16] = bigint_to_be_bytes(&b128).unwrap();
            let b128_converted: BigInteger128 = be_bytes_to_bigint(&b128_converted).unwrap();
            assert_eq!(b128, b128_converted);
            let b128_converted: [u8; 16] = bigint_to_le_bytes(&b128).unwrap();
            let b128_converted: BigInteger128 = le_bytes_to_bigint(&b128_converted).unwrap();
            assert_eq!(b128, b128_converted);

            let b256 = BigInteger256::rand(&mut rng);
            let b256_converted: [u8; 32] = bigint_to_be_bytes(&b256).unwrap();
            let b256_converted: BigInteger256 = be_bytes_to_bigint(&b256_converted).unwrap();
            assert_eq!(b256, b256_converted);
            let b256_converted: [u8; 32] = bigint_to_le_bytes(&b256).unwrap();
            let b256_converted: BigInteger256 = le_bytes_to_bigint(&b256_converted).unwrap();
            assert_eq!(b256, b256_converted);

            let b320 = BigInteger320::rand(&mut rng);
            let b320_converted: [u8; 40] = bigint_to_be_bytes(&b320).unwrap();
            let b320_converted: BigInteger320 = be_bytes_to_bigint(&b320_converted).unwrap();
            assert_eq!(b320, b320_converted);
            let b320_converted: [u8; 40] = bigint_to_le_bytes(&b320).unwrap();
            let b320_converted: BigInteger320 = le_bytes_to_bigint(&b320_converted).unwrap();
            assert_eq!(b320, b320_converted);

            let b384 = BigInteger384::rand(&mut rng);
            let b384_converted: [u8; 48] = bigint_to_be_bytes(&b384).unwrap();
            let b384_converted: BigInteger384 = be_bytes_to_bigint(&b384_converted).unwrap();
            assert_eq!(b384, b384_converted);
            let b384_converted: [u8; 48] = bigint_to_le_bytes(&b384).unwrap();
            let b384_converted: BigInteger384 = le_bytes_to_bigint(&b384_converted).unwrap();
            assert_eq!(b384, b384_converted);

            let b448 = BigInteger448::rand(&mut rng);
            let b448_converted: [u8; 56] = bigint_to_be_bytes(&b448).unwrap();
            let b448_converted: BigInteger448 = be_bytes_to_bigint(&b448_converted).unwrap();
            assert_eq!(b448, b448_converted);
            let b448_converted: [u8; 56] = bigint_to_le_bytes(&b448).unwrap();
            let b448_converted: BigInteger448 = le_bytes_to_bigint(&b448_converted).unwrap();
            assert_eq!(b448, b448_converted);

            let b768 = BigInteger768::rand(&mut rng);
            let b768_converted: [u8; 96] = bigint_to_be_bytes(&b768).unwrap();
            let b768_converted: BigInteger768 = be_bytes_to_bigint(&b768_converted).unwrap();
            assert_eq!(b768, b768_converted);
            let b768_converted: [u8; 96] = bigint_to_le_bytes(&b768).unwrap();
            let b768_converted: BigInteger768 = le_bytes_to_bigint(&b768_converted).unwrap();
            assert_eq!(b768, b768_converted);

            let b832 = BigInteger832::rand(&mut rng);
            let b832_converted: [u8; 104] = bigint_to_be_bytes(&b832).unwrap();
            let b832_converted: BigInteger832 = be_bytes_to_bigint(&b832_converted).unwrap();
            assert_eq!(b832, b832_converted);
            let b832_converted: [u8; 104] = bigint_to_le_bytes(&b832).unwrap();
            let b832_converted: BigInteger832 = le_bytes_to_bigint(&b832_converted).unwrap();
            assert_eq!(b832, b832_converted);
        }
    }

    #[test]
    fn test_bigint_conversion_zero() {
        let b64 = BigInteger64::zero();
        let b64_converted: [u8; 8] = bigint_to_be_bytes(&b64).unwrap();
        let b64_converted: BigInteger64 = be_bytes_to_bigint(&b64_converted).unwrap();
        assert_eq!(b64, b64_converted);
        let b64_converted: [u8; 8] = bigint_to_le_bytes(&b64).unwrap();
        let b64_converted: BigInteger64 = le_bytes_to_bigint(&b64_converted).unwrap();
        assert_eq!(b64, b64_converted);

        let b128 = BigInteger128::zero();
        let b128_converted: [u8; 16] = bigint_to_be_bytes(&b128).unwrap();
        let b128_converted: BigInteger128 = be_bytes_to_bigint(&b128_converted).unwrap();
        assert_eq!(b128, b128_converted);
        let b128_converted: [u8; 16] = bigint_to_le_bytes(&b128).unwrap();
        let b128_converted: BigInteger128 = le_bytes_to_bigint(&b128_converted).unwrap();
        assert_eq!(b128, b128_converted);

        let b256 = BigInteger256::zero();
        let b256_converted: [u8; 32] = bigint_to_be_bytes(&b256).unwrap();
        let b256_converted: BigInteger256 = be_bytes_to_bigint(&b256_converted).unwrap();
        assert_eq!(b256, b256_converted);
        let b256_converted: [u8; 32] = bigint_to_le_bytes(&b256).unwrap();
        let b256_converted: BigInteger256 = le_bytes_to_bigint(&b256_converted).unwrap();
        assert_eq!(b256, b256_converted);

        let b320 = BigInteger320::zero();
        let b320_converted: [u8; 40] = bigint_to_be_bytes(&b320).unwrap();
        let b320_converted: BigInteger320 = be_bytes_to_bigint(&b320_converted).unwrap();
        assert_eq!(b320, b320_converted);
        let b320_converted: [u8; 40] = bigint_to_le_bytes(&b320).unwrap();
        let b320_converted: BigInteger320 = le_bytes_to_bigint(&b320_converted).unwrap();
        assert_eq!(b320, b320_converted);

        let b384 = BigInteger384::zero();
        let b384_converted: [u8; 48] = bigint_to_be_bytes(&b384).unwrap();
        let b384_converted: BigInteger384 = be_bytes_to_bigint(&b384_converted).unwrap();
        assert_eq!(b384, b384_converted);
        let b384_converted: [u8; 48] = bigint_to_le_bytes(&b384).unwrap();
        let b384_converted: BigInteger384 = le_bytes_to_bigint(&b384_converted).unwrap();
        assert_eq!(b384, b384_converted);

        let b448 = BigInteger448::zero();
        let b448_converted: [u8; 56] = bigint_to_be_bytes(&b448).unwrap();
        let b448_converted: BigInteger448 = be_bytes_to_bigint(&b448_converted).unwrap();
        assert_eq!(b448, b448_converted);
        let b448_converted: [u8; 56] = bigint_to_le_bytes(&b448).unwrap();
        let b448_converted: BigInteger448 = le_bytes_to_bigint(&b448_converted).unwrap();
        assert_eq!(b448, b448_converted);

        let b768 = BigInteger768::zero();
        let b768_converted: [u8; 96] = bigint_to_be_bytes(&b768).unwrap();
        let b768_converted: BigInteger768 = be_bytes_to_bigint(&b768_converted).unwrap();
        assert_eq!(b768, b768_converted);
        let b768_converted: [u8; 96] = bigint_to_le_bytes(&b768).unwrap();
        let b768_converted: BigInteger768 = le_bytes_to_bigint(&b768_converted).unwrap();
        assert_eq!(b768, b768_converted);

        let b832 = BigInteger832::zero();
        let b832_converted: [u8; 104] = bigint_to_be_bytes(&b832).unwrap();
        let b832_converted: BigInteger832 = be_bytes_to_bigint(&b832_converted).unwrap();
        assert_eq!(b832, b832_converted);
        let b832_converted: [u8; 104] = bigint_to_le_bytes(&b832).unwrap();
        let b832_converted: BigInteger832 = le_bytes_to_bigint(&b832_converted).unwrap();
        assert_eq!(b832, b832_converted);
    }

    #[test]
    fn test_bigint_conversion_one() {
        let b64 = BigInteger64::one();
        let b64_converted: [u8; 8] = bigint_to_be_bytes(&b64).unwrap();
        let b64_converted: BigInteger64 = be_bytes_to_bigint(&b64_converted).unwrap();
        assert_eq!(b64, b64_converted);
        let b64_converted: [u8; 8] = bigint_to_le_bytes(&b64).unwrap();
        let b64_converted: BigInteger64 = le_bytes_to_bigint(&b64_converted).unwrap();
        assert_eq!(b64, b64_converted);

        let b128 = BigInteger128::one();
        let b128_converted: [u8; 16] = bigint_to_be_bytes(&b128).unwrap();
        let b128_converted: BigInteger128 = be_bytes_to_bigint(&b128_converted).unwrap();
        assert_eq!(b128, b128_converted);
        let b128_converted: [u8; 16] = bigint_to_le_bytes(&b128).unwrap();
        let b128_converted: BigInteger128 = le_bytes_to_bigint(&b128_converted).unwrap();
        assert_eq!(b128, b128_converted);

        let b256 = BigInteger256::one();
        let b256_converted: [u8; 32] = bigint_to_be_bytes(&b256).unwrap();
        let b256_converted: BigInteger256 = be_bytes_to_bigint(&b256_converted).unwrap();
        assert_eq!(b256, b256_converted);
        let b256_converted: [u8; 32] = bigint_to_le_bytes(&b256).unwrap();
        let b256_converted: BigInteger256 = le_bytes_to_bigint(&b256_converted).unwrap();
        assert_eq!(b256, b256_converted);

        let b320 = BigInteger320::one();
        let b320_converted: [u8; 40] = bigint_to_be_bytes(&b320).unwrap();
        let b320_converted: BigInteger320 = be_bytes_to_bigint(&b320_converted).unwrap();
        assert_eq!(b320, b320_converted);
        let b320_converted: [u8; 40] = bigint_to_le_bytes(&b320).unwrap();
        let b320_converted: BigInteger320 = le_bytes_to_bigint(&b320_converted).unwrap();
        assert_eq!(b320, b320_converted);

        let b384 = BigInteger384::one();
        let b384_converted: [u8; 48] = bigint_to_be_bytes(&b384).unwrap();
        let b384_converted: BigInteger384 = be_bytes_to_bigint(&b384_converted).unwrap();
        assert_eq!(b384, b384_converted);
        let b384_converted: [u8; 48] = bigint_to_le_bytes(&b384).unwrap();
        let b384_converted: BigInteger384 = le_bytes_to_bigint(&b384_converted).unwrap();
        assert_eq!(b384, b384_converted);

        let b448 = BigInteger448::one();
        let b448_converted: [u8; 56] = bigint_to_be_bytes(&b448).unwrap();
        let b448_converted: BigInteger448 = be_bytes_to_bigint(&b448_converted).unwrap();
        assert_eq!(b448, b448_converted);
        let b448_converted: [u8; 56] = bigint_to_le_bytes(&b448).unwrap();
        let b448_converted: BigInteger448 = le_bytes_to_bigint(&b448_converted).unwrap();
        assert_eq!(b448, b448_converted);

        let b768 = BigInteger768::one();
        let b768_converted: [u8; 96] = bigint_to_be_bytes(&b768).unwrap();
        let b768_converted: BigInteger768 = be_bytes_to_bigint(&b768_converted).unwrap();
        assert_eq!(b768, b768_converted);
        let b768_converted: [u8; 96] = bigint_to_le_bytes(&b768).unwrap();
        let b768_converted: BigInteger768 = le_bytes_to_bigint(&b768_converted).unwrap();
        assert_eq!(b768, b768_converted);

        let b832 = BigInteger832::one();
        let b832_converted: [u8; 104] = bigint_to_be_bytes(&b832).unwrap();
        let b832_converted: BigInteger832 = be_bytes_to_bigint(&b832_converted).unwrap();
        assert_eq!(b832, b832_converted);
        let b832_converted: [u8; 104] = bigint_to_le_bytes(&b832).unwrap();
        let b832_converted: BigInteger832 = le_bytes_to_bigint(&b832_converted).unwrap();
        assert_eq!(b832, b832_converted);
    }

    #[test]
    fn test_bigint_conversion_max() {
        let b64 = BigInteger64::new([u64::MAX; 1]);
        let b64_converted: [u8; 8] = bigint_to_be_bytes(&b64).unwrap();
        let b64_converted: BigInteger64 = be_bytes_to_bigint(&b64_converted).unwrap();
        assert_eq!(b64, b64_converted);
        let b64_converted: [u8; 8] = bigint_to_le_bytes(&b64).unwrap();
        let b64_converted: BigInteger64 = le_bytes_to_bigint(&b64_converted).unwrap();
        assert_eq!(b64, b64_converted);

        let b128 = BigInteger128::new([u64::MAX; 2]);
        let b128_converted: [u8; 16] = bigint_to_be_bytes(&b128).unwrap();
        let b128_converted: BigInteger128 = be_bytes_to_bigint(&b128_converted).unwrap();
        assert_eq!(b128, b128_converted);
        let b128_converted: [u8; 16] = bigint_to_le_bytes(&b128).unwrap();
        let b128_converted: BigInteger128 = le_bytes_to_bigint(&b128_converted).unwrap();
        assert_eq!(b128, b128_converted);

        let b256 = BigInteger256::new([u64::MAX; 4]);
        let b256_converted: [u8; 32] = bigint_to_be_bytes(&b256).unwrap();
        let b256_converted: BigInteger256 = be_bytes_to_bigint(&b256_converted).unwrap();
        assert_eq!(b256, b256_converted);
        let b256_converted: [u8; 32] = bigint_to_le_bytes(&b256).unwrap();
        let b256_converted: BigInteger256 = le_bytes_to_bigint(&b256_converted).unwrap();
        assert_eq!(b256, b256_converted);

        let b320 = BigInteger320::new([u64::MAX; 5]);
        let b320_converted: [u8; 40] = bigint_to_be_bytes(&b320).unwrap();
        let b320_converted: BigInteger320 = be_bytes_to_bigint(&b320_converted).unwrap();
        assert_eq!(b320, b320_converted);
        let b320_converted: [u8; 40] = bigint_to_le_bytes(&b320).unwrap();
        let b320_converted: BigInteger320 = le_bytes_to_bigint(&b320_converted).unwrap();
        assert_eq!(b320, b320_converted);

        let b384 = BigInteger384::new([u64::MAX; 6]);
        let b384_converted: [u8; 48] = bigint_to_be_bytes(&b384).unwrap();
        let b384_converted: BigInteger384 = be_bytes_to_bigint(&b384_converted).unwrap();
        assert_eq!(b384, b384_converted);
        let b384_converted: [u8; 48] = bigint_to_le_bytes(&b384).unwrap();
        let b384_converted: BigInteger384 = le_bytes_to_bigint(&b384_converted).unwrap();
        assert_eq!(b384, b384_converted);

        let b448 = BigInteger448::new([u64::MAX; 7]);
        let b448_converted: [u8; 56] = bigint_to_be_bytes(&b448).unwrap();
        let b448_converted: BigInteger448 = be_bytes_to_bigint(&b448_converted).unwrap();
        assert_eq!(b448, b448_converted);
        let b448_converted: [u8; 56] = bigint_to_le_bytes(&b448).unwrap();
        let b448_converted: BigInteger448 = le_bytes_to_bigint(&b448_converted).unwrap();
        assert_eq!(b448, b448_converted);

        let b768 = BigInteger768::new([u64::MAX; 12]);
        let b768_converted: [u8; 96] = bigint_to_be_bytes(&b768).unwrap();
        let b768_converted: BigInteger768 = be_bytes_to_bigint(&b768_converted).unwrap();
        assert_eq!(b768, b768_converted);
        let b768_converted: [u8; 96] = bigint_to_le_bytes(&b768).unwrap();
        let b768_converted: BigInteger768 = le_bytes_to_bigint(&b768_converted).unwrap();
        assert_eq!(b768, b768_converted);

        let b832 = BigInteger832::new([u64::MAX; 13]);
        let b832_converted: [u8; 104] = bigint_to_be_bytes(&b832).unwrap();
        let b832_converted: BigInteger832 = be_bytes_to_bigint(&b832_converted).unwrap();
        assert_eq!(b832, b832_converted);
        let b832_converted: [u8; 104] = bigint_to_le_bytes(&b832).unwrap();
        let b832_converted: BigInteger832 = le_bytes_to_bigint(&b832_converted).unwrap();
        assert_eq!(b832, b832_converted);
    }

    #[test]
    fn test_bigint_conversion_invalid_size() {
        let b64 = BigInteger64::one();
        let res: Result<[u8; 1], UtilsError> = bigint_to_be_bytes(&b64);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(8, 1))));
        let res: Result<[u8; 7], UtilsError> = bigint_to_be_bytes(&b64);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(8, 7))));
        let res: Result<[u8; 9], UtilsError> = bigint_to_be_bytes(&b64);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(8, 9))));

        let b128 = BigInteger128::one();
        let res: Result<[u8; 1], UtilsError> = bigint_to_be_bytes(&b128);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(16, 1))));
        let res: Result<[u8; 15], UtilsError> = bigint_to_be_bytes(&b128);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(16, 15))));
        let res: Result<[u8; 17], UtilsError> = bigint_to_be_bytes(&b128);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(16, 17))));

        let b256 = BigInteger256::one();
        let res: Result<[u8; 1], UtilsError> = bigint_to_be_bytes(&b256);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(32, 1))));
        let res: Result<[u8; 31], UtilsError> = bigint_to_be_bytes(&b256);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(32, 31))));
        let res: Result<[u8; 33], UtilsError> = bigint_to_be_bytes(&b256);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(32, 33))));

        let b320 = BigInteger320::one();
        let res: Result<[u8; 1], UtilsError> = bigint_to_be_bytes(&b320);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(40, 1))));
        let res: Result<[u8; 39], UtilsError> = bigint_to_be_bytes(&b320);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(40, 39))));
        let res: Result<[u8; 41], UtilsError> = bigint_to_be_bytes(&b320);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(40, 41))));

        let b384 = BigInteger384::one();
        let res: Result<[u8; 1], UtilsError> = bigint_to_be_bytes(&b384);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(48, 1))));
        let res: Result<[u8; 47], UtilsError> = bigint_to_be_bytes(&b384);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(48, 47))));
        let res: Result<[u8; 49], UtilsError> = bigint_to_be_bytes(&b384);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(48, 49))));

        let b448 = BigInteger448::one();
        let res: Result<[u8; 1], UtilsError> = bigint_to_be_bytes(&b448);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(56, 1))));
        let res: Result<[u8; 55], UtilsError> = bigint_to_be_bytes(&b448);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(56, 55))));
        let res: Result<[u8; 57], UtilsError> = bigint_to_be_bytes(&b448);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(56, 57))));

        let b768 = BigInteger768::one();
        let res: Result<[u8; 1], UtilsError> = bigint_to_be_bytes(&b768);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(96, 1))));
        let res: Result<[u8; 95], UtilsError> = bigint_to_be_bytes(&b768);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(96, 95))));
        let res: Result<[u8; 97], UtilsError> = bigint_to_be_bytes(&b768);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(96, 97))));

        let b832 = BigInteger832::one();
        let res: Result<[u8; 1], UtilsError> = bigint_to_be_bytes(&b832);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(104, 1))));
        let res: Result<[u8; 103], UtilsError> = bigint_to_be_bytes(&b832);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(104, 103))));
        let res: Result<[u8; 105], UtilsError> = bigint_to_be_bytes(&b832);
        assert!(matches!(res, Err(UtilsError::InvalidInputSize(104, 105))));
    }
}

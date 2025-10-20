use light_hasher::{
    bigint::{bigint_to_be_bytes_array, bigint_to_le_bytes_array},
    HasherError,
};
use num_bigint::{BigUint, RandBigInt, ToBigUint};
use rand::thread_rng;

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
    let b8 = BigUint::from_bytes_be(&[1; 8]);
    let res: Result<[u8; 1], HasherError> = bigint_to_be_bytes_array(&b8);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(1, 8))));
    let res: Result<[u8; 7], HasherError> = bigint_to_be_bytes_array(&b8);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(7, 8))));
    let res: Result<[u8; 8], HasherError> = bigint_to_be_bytes_array(&b8);
    assert!(res.is_ok());

    let b8 = BigUint::from_bytes_le(&[1; 8]);
    let res: Result<[u8; 1], HasherError> = bigint_to_le_bytes_array(&b8);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(1, 8))));
    let res: Result<[u8; 7], HasherError> = bigint_to_le_bytes_array(&b8);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(7, 8))));
    let res: Result<[u8; 8], HasherError> = bigint_to_le_bytes_array(&b8);
    assert!(res.is_ok());

    let b16 = BigUint::from_bytes_be(&[1; 16]);
    let res: Result<[u8; 1], HasherError> = bigint_to_be_bytes_array(&b16);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(1, 16))));
    let res: Result<[u8; 15], HasherError> = bigint_to_be_bytes_array(&b16);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(15, 16))));
    let res: Result<[u8; 16], HasherError> = bigint_to_be_bytes_array(&b16);
    assert!(res.is_ok());

    let b16 = BigUint::from_bytes_le(&[1; 16]);
    let res: Result<[u8; 1], HasherError> = bigint_to_le_bytes_array(&b16);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(1, 16))));
    let res: Result<[u8; 15], HasherError> = bigint_to_le_bytes_array(&b16);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(15, 16))));
    let res: Result<[u8; 16], HasherError> = bigint_to_le_bytes_array(&b16);
    assert!(res.is_ok());

    let b32 = BigUint::from_bytes_be(&[1; 32]);
    let res: Result<[u8; 1], HasherError> = bigint_to_be_bytes_array(&b32);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(1, 32))));
    let res: Result<[u8; 31], HasherError> = bigint_to_be_bytes_array(&b32);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(31, 32))));
    let res: Result<[u8; 32], HasherError> = bigint_to_be_bytes_array(&b32);
    assert!(res.is_ok());

    let b32 = BigUint::from_bytes_le(&[1; 32]);
    let res: Result<[u8; 1], HasherError> = bigint_to_le_bytes_array(&b32);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(1, 32))));
    let res: Result<[u8; 31], HasherError> = bigint_to_le_bytes_array(&b32);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(31, 32))));
    let res: Result<[u8; 32], HasherError> = bigint_to_le_bytes_array(&b32);
    assert!(res.is_ok());

    let b64 = BigUint::from_bytes_be(&[1; 64]);
    let res: Result<[u8; 1], HasherError> = bigint_to_be_bytes_array(&b64);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(1, 64))));
    let res: Result<[u8; 63], HasherError> = bigint_to_be_bytes_array(&b64);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(63, 64))));
    let res: Result<[u8; 64], HasherError> = bigint_to_be_bytes_array(&b64);
    assert!(res.is_ok());

    let b64 = BigUint::from_bytes_le(&[1; 64]);
    let res: Result<[u8; 1], HasherError> = bigint_to_le_bytes_array(&b64);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(1, 64))));
    let res: Result<[u8; 63], HasherError> = bigint_to_le_bytes_array(&b64);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(63, 64))));
    let res: Result<[u8; 64], HasherError> = bigint_to_le_bytes_array(&b64);
    assert!(res.is_ok());

    let b128 = BigUint::from_bytes_be(&[1; 128]);
    let res: Result<[u8; 1], HasherError> = bigint_to_be_bytes_array(&b128);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(1, 128))));
    let res: Result<[u8; 127], HasherError> = bigint_to_be_bytes_array(&b128);
    assert!(matches!(
        res,
        Err(HasherError::InvalidInputLength(127, 128))
    ));
    let res: Result<[u8; 128], HasherError> = bigint_to_be_bytes_array(&b128);
    assert!(res.is_ok());

    let b128 = BigUint::from_bytes_le(&[1; 128]);
    let res: Result<[u8; 1], HasherError> = bigint_to_le_bytes_array(&b128);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(1, 128))));
    let res: Result<[u8; 127], HasherError> = bigint_to_le_bytes_array(&b128);
    assert!(matches!(
        res,
        Err(HasherError::InvalidInputLength(127, 128))
    ));
    let res: Result<[u8; 128], HasherError> = bigint_to_le_bytes_array(&b128);
    assert!(res.is_ok());

    let b256 = BigUint::from_bytes_be(&[1; 256]);
    let res: Result<[u8; 1], HasherError> = bigint_to_be_bytes_array(&b256);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(1, 256))));
    let res: Result<[u8; 255], HasherError> = bigint_to_be_bytes_array(&b256);
    assert!(matches!(
        res,
        Err(HasherError::InvalidInputLength(255, 256))
    ));
    let res: Result<[u8; 256], HasherError> = bigint_to_be_bytes_array(&b256);
    assert!(res.is_ok());

    let b256 = BigUint::from_bytes_le(&[1; 256]);
    let res: Result<[u8; 1], HasherError> = bigint_to_le_bytes_array(&b256);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(1, 256))));
    let res: Result<[u8; 255], HasherError> = bigint_to_le_bytes_array(&b256);
    assert!(matches!(
        res,
        Err(HasherError::InvalidInputLength(255, 256))
    ));
    let res: Result<[u8; 256], HasherError> = bigint_to_le_bytes_array(&b256);
    assert!(res.is_ok());

    let b512 = BigUint::from_bytes_be(&[1; 512]);
    let res: Result<[u8; 1], HasherError> = bigint_to_be_bytes_array(&b512);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(1, 512))));
    let res: Result<[u8; 511], HasherError> = bigint_to_be_bytes_array(&b512);
    assert!(matches!(
        res,
        Err(HasherError::InvalidInputLength(511, 512))
    ));
    let res: Result<[u8; 512], HasherError> = bigint_to_be_bytes_array(&b512);
    assert!(res.is_ok());

    let b512 = BigUint::from_bytes_le(&[1; 512]);
    let res: Result<[u8; 1], HasherError> = bigint_to_le_bytes_array(&b512);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(1, 512))));
    let res: Result<[u8; 511], HasherError> = bigint_to_le_bytes_array(&b512);
    assert!(matches!(
        res,
        Err(HasherError::InvalidInputLength(511, 512))
    ));
    let res: Result<[u8; 512], HasherError> = bigint_to_le_bytes_array(&b512);
    assert!(res.is_ok());

    let b768 = BigUint::from_bytes_be(&[1; 768]);
    let res: Result<[u8; 1], HasherError> = bigint_to_be_bytes_array(&b768);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(1, 768))));
    let res: Result<[u8; 767], HasherError> = bigint_to_be_bytes_array(&b768);
    assert!(matches!(
        res,
        Err(HasherError::InvalidInputLength(767, 768))
    ));
    let res: Result<[u8; 768], HasherError> = bigint_to_be_bytes_array(&b768);
    assert!(res.is_ok());

    let b768 = BigUint::from_bytes_le(&[1; 768]);
    let res: Result<[u8; 1], HasherError> = bigint_to_le_bytes_array(&b768);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(1, 768))));
    let res: Result<[u8; 767], HasherError> = bigint_to_le_bytes_array(&b768);
    assert!(matches!(
        res,
        Err(HasherError::InvalidInputLength(767, 768))
    ));
    let res: Result<[u8; 768], HasherError> = bigint_to_le_bytes_array(&b768);
    assert!(res.is_ok());

    let b1024 = BigUint::from_bytes_be(&[1; 1024]);
    let res: Result<[u8; 1], HasherError> = bigint_to_be_bytes_array(&b1024);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(1, 1024))));
    let res: Result<[u8; 1023], HasherError> = bigint_to_be_bytes_array(&b1024);
    assert!(matches!(
        res,
        Err(HasherError::InvalidInputLength(1023, 1024))
    ));
    let res: Result<[u8; 1024], HasherError> = bigint_to_be_bytes_array(&b1024);
    assert!(res.is_ok());

    let b1024 = BigUint::from_bytes_le(&[1; 1024]);
    let res: Result<[u8; 1], HasherError> = bigint_to_le_bytes_array(&b1024);
    assert!(matches!(res, Err(HasherError::InvalidInputLength(1, 1024))));
    let res: Result<[u8; 1023], HasherError> = bigint_to_le_bytes_array(&b1024);
    assert!(matches!(
        res,
        Err(HasherError::InvalidInputLength(1023, 1024))
    ));
    let res: Result<[u8; 1024], HasherError> = bigint_to_le_bytes_array(&b1024);
    assert!(res.is_ok());
}

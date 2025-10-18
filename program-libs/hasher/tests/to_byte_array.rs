use light_hasher::{to_byte_array::ToByteArray, HasherError};

#[test]
fn test_to_byte_array_integers() {
    // i8 tests
    let i8_min_result = i8::MIN.to_byte_array().unwrap();
    let mut expected_i8_min = [0u8; 32];
    expected_i8_min[31] = 128; // i8::MIN.to_be_bytes() = [128]
    assert_eq!(i8_min_result, expected_i8_min);

    let i8_max_result = i8::MAX.to_byte_array().unwrap();
    let mut expected_i8_max = [0u8; 32];
    expected_i8_max[31] = 127; // i8::MAX.to_be_bytes() = [127]
    assert_eq!(i8_max_result, expected_i8_max);

    // u8 tests
    let u8_min_result = u8::MIN.to_byte_array().unwrap();
    let mut expected_u8_min = [0u8; 32];
    expected_u8_min[31] = 0; // u8::MIN.to_be_bytes() = [0]
    assert_eq!(u8_min_result, expected_u8_min);

    let u8_max_result = u8::MAX.to_byte_array().unwrap();
    let mut expected_u8_max = [0u8; 32];
    expected_u8_max[31] = 255; // u8::MAX.to_be_bytes() = [255]
    assert_eq!(u8_max_result, expected_u8_max);

    // i16 tests
    let i16_min_result = i16::MIN.to_byte_array().unwrap();
    let mut expected_i16_min = [0u8; 32];
    expected_i16_min[30..32].copy_from_slice(&i16::MIN.to_be_bytes()); // [128, 0]
    assert_eq!(i16_min_result, expected_i16_min);

    let i16_max_result = i16::MAX.to_byte_array().unwrap();
    let mut expected_i16_max = [0u8; 32];
    expected_i16_max[30..32].copy_from_slice(&i16::MAX.to_be_bytes()); // [127, 255]
    assert_eq!(i16_max_result, expected_i16_max);

    // u16 tests
    let u16_min_result = u16::MIN.to_byte_array().unwrap();
    let mut expected_u16_min = [0u8; 32];
    expected_u16_min[30..32].copy_from_slice(&u16::MIN.to_be_bytes()); // [0, 0]
    assert_eq!(u16_min_result, expected_u16_min);

    let u16_max_result = u16::MAX.to_byte_array().unwrap();
    let mut expected_u16_max = [0u8; 32];
    expected_u16_max[30..32].copy_from_slice(&u16::MAX.to_be_bytes()); // [255, 255]
    assert_eq!(u16_max_result, expected_u16_max);

    // i32 tests
    let i32_min_result = i32::MIN.to_byte_array().unwrap();
    let mut expected_i32_min = [0u8; 32];
    expected_i32_min[28..32].copy_from_slice(&i32::MIN.to_be_bytes()); // [128, 0, 0, 0]
    assert_eq!(i32_min_result, expected_i32_min);

    let i32_max_result = i32::MAX.to_byte_array().unwrap();
    let mut expected_i32_max = [0u8; 32];
    expected_i32_max[28..32].copy_from_slice(&i32::MAX.to_be_bytes()); // [127, 255, 255, 255]
    assert_eq!(i32_max_result, expected_i32_max);

    // u32 tests
    let u32_min_result = u32::MIN.to_byte_array().unwrap();
    let mut expected_u32_min = [0u8; 32];
    expected_u32_min[28..32].copy_from_slice(&u32::MIN.to_be_bytes()); // [0, 0, 0, 0]
    assert_eq!(u32_min_result, expected_u32_min);

    let u32_max_result = u32::MAX.to_byte_array().unwrap();
    let mut expected_u32_max = [0u8; 32];
    expected_u32_max[28..32].copy_from_slice(&u32::MAX.to_be_bytes()); // [255, 255, 255, 255]
    assert_eq!(u32_max_result, expected_u32_max);

    // i64 tests
    let i64_min_result = i64::MIN.to_byte_array().unwrap();
    let mut expected_i64_min = [0u8; 32];
    expected_i64_min[24..32].copy_from_slice(&i64::MIN.to_be_bytes()); // [128, 0, 0, 0, 0, 0, 0, 0]
    assert_eq!(i64_min_result, expected_i64_min);

    let i64_max_result = i64::MAX.to_byte_array().unwrap();
    let mut expected_i64_max = [0u8; 32];
    expected_i64_max[24..32].copy_from_slice(&i64::MAX.to_be_bytes()); // [127, 255, 255, 255, 255, 255, 255, 255]
    assert_eq!(i64_max_result, expected_i64_max);

    // u64 tests
    let u64_min_result = u64::MIN.to_byte_array().unwrap();
    let mut expected_u64_min = [0u8; 32];
    expected_u64_min[24..32].copy_from_slice(&u64::MIN.to_be_bytes()); // [0, 0, 0, 0, 0, 0, 0, 0]
    assert_eq!(u64_min_result, expected_u64_min);

    let u64_max_result = u64::MAX.to_byte_array().unwrap();
    let mut expected_u64_max = [0u8; 32];
    expected_u64_max[24..32].copy_from_slice(&u64::MAX.to_be_bytes()); // [255, 255, 255, 255, 255, 255, 255, 255]
    assert_eq!(u64_max_result, expected_u64_max);

    // i128 tests
    let i128_min_result = i128::MIN.to_byte_array().unwrap();
    let mut expected_i128_min = [0u8; 32];
    expected_i128_min[16..32].copy_from_slice(&i128::MIN.to_be_bytes()); // [128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    assert_eq!(i128_min_result, expected_i128_min);

    let i128_max_result = i128::MAX.to_byte_array().unwrap();
    let mut expected_i128_max = [0u8; 32];
    expected_i128_max[16..32].copy_from_slice(&i128::MAX.to_be_bytes()); // [127, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255]
    assert_eq!(i128_max_result, expected_i128_max);

    // u128 tests
    let u128_min_result = u128::MIN.to_byte_array().unwrap();
    let mut expected_u128_min = [0u8; 32];
    expected_u128_min[16..32].copy_from_slice(&u128::MIN.to_be_bytes()); // [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    assert_eq!(u128_min_result, expected_u128_min);

    let u128_max_result = u128::MAX.to_byte_array().unwrap();
    let mut expected_u128_max = [0u8; 32];
    expected_u128_max[16..32].copy_from_slice(&u128::MAX.to_be_bytes()); // [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255]
    assert_eq!(u128_max_result, expected_u128_max);
}

#[test]
fn test_to_byte_array_primitives() {
    // Test bool::to_byte_array
    let bool_false_result = false.to_byte_array().unwrap();
    let mut expected_bool_false = [0u8; 32];
    expected_bool_false[31] = 0;
    assert_eq!(bool_false_result, expected_bool_false);

    let bool_true_result = true.to_byte_array().unwrap();
    let mut expected_bool_true = [0u8; 32];
    expected_bool_true[31] = 1;
    assert_eq!(bool_true_result, expected_bool_true);
}

#[test]
fn test_to_byte_array_option() {
    // Very important property - `None` and `Some(0)` always have to be
    // different and should produce different hashes!

    // Test Option<u8>::to_byte_array
    let u8_none: Option<u8> = None;
    let u8_none_result = u8_none.to_byte_array().unwrap();
    assert_eq!(u8_none_result, [0u8; 32]);

    let u8_some_zero: Option<u8> = Some(0);
    let u8_some_zero_result = u8_some_zero.to_byte_array().unwrap();
    let mut expected_u8_some_zero = [0u8; 32];
    expected_u8_some_zero[32 - std::mem::size_of::<u8>() - 1] = 1; // Mark as Some
    assert_eq!(u8_some_zero_result, expected_u8_some_zero);

    // Test Option<u16>::to_byte_array
    let u16_none: Option<u16> = None;
    let u16_none_result = u16_none.to_byte_array().unwrap();
    assert_eq!(u16_none_result, [0u8; 32]);

    let u16_some_zero: Option<u16> = Some(0);
    let u16_some_zero_result = u16_some_zero.to_byte_array().unwrap();
    let mut expected_u16_some_zero = [0u8; 32];
    expected_u16_some_zero[32 - std::mem::size_of::<u16>() - 1] = 1; // Mark as Some
    assert_eq!(u16_some_zero_result, expected_u16_some_zero);

    // Test Option<u32>::to_byte_array
    let u32_none: Option<u32> = None;
    let u32_none_result = u32_none.to_byte_array().unwrap();
    assert_eq!(u32_none_result, [0u8; 32]);

    let u32_some_zero: Option<u32> = Some(0);
    let u32_some_zero_result = u32_some_zero.to_byte_array().unwrap();
    let mut expected_u32_some_zero = [0u8; 32];
    expected_u32_some_zero[32 - std::mem::size_of::<u32>() - 1] = 1; // Mark as Some
    assert_eq!(u32_some_zero_result, expected_u32_some_zero);

    // Test Option<u64>::to_byte_array
    let u64_none: Option<u64> = None;
    let u64_none_result = u64_none.to_byte_array().unwrap();
    assert_eq!(u64_none_result, [0u8; 32]);

    let u64_some_zero: Option<u64> = Some(0);
    let u64_some_zero_result = u64_some_zero.to_byte_array().unwrap();
    let mut expected_u64_some_zero = [0u8; 32];
    expected_u64_some_zero[32 - std::mem::size_of::<u64>() - 1] = 1; // Mark as Some
    assert_eq!(u64_some_zero_result, expected_u64_some_zero);

    // Test Option<u128>::to_byte_array
    let u128_none: Option<u128> = None;
    let u128_none_result = u128_none.to_byte_array().unwrap();
    assert_eq!(u128_none_result, [0u8; 32]);

    let u128_some_zero: Option<u128> = Some(0);
    let u128_some_zero_result = u128_some_zero.to_byte_array().unwrap();
    let mut expected_u128_some_zero = [0u8; 32];
    expected_u128_some_zero[32 - std::mem::size_of::<u128>() - 1] = 1; // Mark as Some
    assert_eq!(u128_some_zero_result, expected_u128_some_zero);
}

#[test]
fn test_to_byte_array_u8_arrays() {
    // Test with single element array
    let single_element_arr: [u8; 1] = [255];
    let result = single_element_arr.to_byte_array().unwrap();
    let mut expected = [0u8; 32];
    expected[31] = 255;
    assert_eq!(result, expected);

    // Test with multi-element array
    let multi_element_arr: [u8; 4] = [1, 2, 3, 4];
    let result = multi_element_arr.to_byte_array().unwrap();
    let mut expected = [0u8; 32];
    expected[32 - 4..].copy_from_slice(&multi_element_arr);
    assert_eq!(result, expected);

    // Test with full 32-byte array
    let full_arr: [u8; 31] = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31,
    ];
    let result = full_arr.to_byte_array().unwrap();
    assert_eq!(result[0], 0);
    assert_eq!(&result[1..], full_arr.as_slice());
}

#[test]
fn test_to_byte_array_string() {
    // Test with empty string
    let empty_string = "".to_string();
    let result = empty_string.to_byte_array().unwrap();
    let expected = [0u8; 32];
    assert_eq!(result, expected);

    // Test with short string
    let short_string = "foobar".to_string();
    let result = short_string.to_byte_array().unwrap();
    let mut expected = [0u8; 32];
    expected[32 - 6..].copy_from_slice(b"foobar");
    assert_eq!(result, expected);

    // Test with longer string that gets truncated
    let long_string = "this is a string that is longer than 32 bytes and will be fail".to_string();
    let byte_len = long_string.len();
    let result = long_string.to_byte_array();
    assert_eq!(result, Err(HasherError::InvalidInputLength(31, byte_len)));
}

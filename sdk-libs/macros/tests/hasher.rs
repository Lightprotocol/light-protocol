use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::hash_to_bn254_field_size_be;
use light_hasher::{to_byte_array::ToByteArray, DataHasher, Hasher, Poseidon, Sha256};
use light_sdk_macros::LightHasher;
use solana_pubkey::Pubkey;

#[derive(LightHasher, Clone)]
pub struct MyAccount {
    pub a: bool,
    pub b: u64,
    pub c: MyNestedStruct,
    #[hash]
    pub d: [u8; 32],
    pub f: Option<u64>,
}

#[derive(LightHasher, Clone)]
pub struct TruncateVec {
    #[hash]
    pub d: Vec<u8>,
}

#[derive(LightHasher, Clone)]
pub struct MyNestedStruct {
    pub a: i32,
    pub b: u32,
    #[hash]
    pub c: String,
}

#[derive(Clone)]
pub struct MyNestedNonHashableStruct {
    pub a: PhantomData<()>,
    pub b: Rc<RefCell<usize>>,
}

#[test]
fn test_simple_hash() {
    let account = MyAccount {
        a: true,
        b: 42,
        c: MyNestedStruct {
            a: 100,
            b: 200,
            c: "test".to_string(),
        },
        d: [1u8; 32],
        f: Some(10),
    };

    // Simply test that hashing works
    let result = account.hash::<Poseidon>();
    assert!(result.is_ok());

    // Test ToByteArray and to_byte_arrays
    let bytes = account.to_byte_array();
    assert!(bytes.is_ok());
}
// #[cfg(test)]
// mod tests {

/// LightHasher Tests
///
/// 1. Basic Hashing (Success):
/// - test_byte_representation: assert_eq! nested struct hash matches manual hash
/// - test_zero_values: assert_eq! zero-value field hash matches manual hash
///
/// 2. Attribute Behavior:
///    a. HashToFieldSize (Success):
///   - test_array_truncation: assert_ne! between different array hashes
///   - test_truncation_longer_array: assert_ne! between different long string hashes
///   - test_multiple_truncates: assert_ne! between multiple truncated field hashes
///   - test_nested_with_truncate: assert_eq! nested + truncated field hash matches manual hash
///
///   b. Nested (Success):
///   - test_recursive_nesting: assert_eq! recursive nested struct hash matches manual hash
///   - test_nested_option: assert_eq! Option<NestedStruct> hash matches manual hash
///   - test_nested_field_count: assert!(is_ok()) with 12 nested fields
///
/// 3. Error Cases (Failure):
/// - test_empty_struct: assert!(is_err()) on empty struct
/// - test_poseidon_width_limits: assert!(is_err()) with >12 fields
/// - test_max_array_length: assert!(is_err()) on array exceeding max size
/// - test_option_array_error: assert!(is_err()) on Option<[u8;32]> without truncate
///
/// 4. Option Handling (Success):
/// - test_option_hashing_with_reference_values: assert_eq! against reference hashes
/// - test_basic_option_variants: assert_eq! basic type hashes match manual hash
/// - test_truncated_option_variants: assert_eq! truncated Option hash matches manual hash
/// - test_nested_option_variants: assert_eq! nested Option hash matches manual hash
/// - test_mixed_option_combinations: assert_eq! combined Option hash matches manual hash
/// - test_nested_struct_with_options: assert_eq! nested struct with options hash matches manual hash
///
/// 5. Option Uniqueness (Success):
/// - test_option_value_uniqueness: assert_ne! between None/Some(0)/Some(1) hashes
/// - test_field_order_uniqueness: assert_ne! between different field orders
/// - test_truncated_option_uniqueness: assert_ne! between None/Some truncated hashes
///
/// 6. Byte Representation (Success):
/// - test_truncate_byte_representation: assert_eq! truncated bytes match expected
/// - test_byte_representation_combinations: assert_eq! bytes match expected
///
mod fixtures {
    use super::*;

    pub fn create_nested_struct() -> MyNestedStruct {
        MyNestedStruct {
            a: i32::MIN,
            b: u32::MAX,
            c: "wao".to_string(),
        }
    }

    pub fn create_account(f: Option<u64>) -> MyAccount {
        MyAccount {
            a: true,
            b: u64::MAX,
            c: create_nested_struct(),
            d: [u8::MAX; 32],
            f,
        }
    }

    pub fn create_zero_nested() -> MyNestedStruct {
        MyNestedStruct {
            a: 0,
            b: 0,
            c: "".to_string(),
        }
    }
}

mod basic_hashing {
    use super::{fixtures::*, *};

    #[test]
    fn test_byte_representation() {
        let nested_struct = create_nested_struct();
        let account = create_account(Some(42));

        let manual_nested_bytes: Vec<Vec<u8>> = vec![
            nested_struct.a.to_be_bytes().to_vec(),
            nested_struct.b.to_be_bytes().to_vec(),
            light_compressed_account::hash_to_bn254_field_size_be(
                nested_struct.c.try_to_vec().unwrap().as_slice(),
            )
            .to_vec(),
        ];

        let nested_bytes: Vec<&[u8]> = manual_nested_bytes.iter().map(|v| v.as_slice()).collect();
        let manual_nested_hash = Poseidon::hashv(&nested_bytes).unwrap();

        let nested_reference_hash = [
            23, 168, 151, 171, 174, 194, 211, 73, 247, 130, 121, 180, 3, 103, 77, 84, 93, 124, 57,
            96, 100, 128, 168, 101, 212, 191, 249, 93, 115, 219, 37, 22,
        ];
        let nested_hash_result = nested_struct.hash::<Poseidon>().unwrap();

        assert_eq!(nested_hash_result, manual_nested_hash);
        assert_eq!(manual_nested_hash, nested_reference_hash);
        assert_eq!(nested_hash_result, manual_nested_hash);

        let manual_account_bytes: Vec<Vec<u8>> = vec![
            vec![u8::from(account.a)],
            account.b.to_be_bytes().to_vec(),
            account.c.hash::<Poseidon>().unwrap().to_vec(),
            light_compressed_account::hash_to_bn254_field_size_be(&account.d).to_vec(),
            {
                let mut bytes = vec![0; 32];
                bytes[24..].copy_from_slice(&account.f.unwrap().to_be_bytes());
                bytes[23] = 1; // Suffix with 1 for Some
                bytes
            },
        ];

        let account_bytes: Vec<&[u8]> = manual_account_bytes.iter().map(|v| v.as_slice()).collect();
        let manual_account_hash = Poseidon::hashv(&account_bytes).unwrap();

        let account_hash_result = account.hash::<Poseidon>().unwrap();

        assert_eq!(account_hash_result, manual_account_hash);
    }

    #[test]
    fn test_zero_values() {
        let nested = create_zero_nested();

        let zero_account = MyAccount {
            a: false,
            b: 0,
            c: nested,
            d: [0; 32],
            f: Some(0),
        };

        let manual_account_bytes = [
            [0u8; 32],
            [0u8; 32],
            zero_account.c.hash::<Poseidon>().unwrap(),
            light_compressed_account::hash_to_bn254_field_size_be(&zero_account.d),
            {
                let mut bytes = [0u8; 32];
                bytes[24..].copy_from_slice(&zero_account.f.unwrap().to_be_bytes());
                bytes[23] = 1; // Suffix with 1 for Some
                bytes
            },
        ];
        let account_bytes: Vec<&[u8]> = manual_account_bytes.iter().map(|v| v.as_slice()).collect();
        let manual_account_hash = Poseidon::hashv(&account_bytes).unwrap();
        let hash = zero_account.hash::<Poseidon>().unwrap();
        assert_eq!(hash, manual_account_hash);

        let expected_hash = [
            47, 62, 70, 12, 78, 227, 140, 201, 110, 213, 91, 205, 99, 218, 61, 163, 117, 26, 219,
            39, 235, 30, 172, 183, 161, 112, 98, 182, 145, 132, 9, 227,
        ];
        assert_eq!(hash, expected_hash);
    }
}

mod attribute_behavior {
    use super::{fixtures::*, *};

    mod truncate {
        use super::*;

        #[test]
        fn test_array_truncation() {
            #[derive(LightHasher)]
            struct TruncatedStruct {
                #[hash]
                data: [u8; 32],
            }

            let ones = TruncatedStruct { data: [1u8; 32] };
            let twos = TruncatedStruct { data: [2u8; 32] };
            let mixed = TruncatedStruct {
                data: {
                    let mut data = [1u8; 32];
                    data[0] = 2u8;
                    data
                },
            };

            let ones_hash = ones.hash::<Poseidon>().unwrap();
            let twos_hash = twos.hash::<Poseidon>().unwrap();
            let mixed_hash = mixed.hash::<Poseidon>().unwrap();

            assert_ne!(ones_hash, twos_hash);
            assert_ne!(ones_hash, mixed_hash);
            assert_ne!(twos_hash, mixed_hash);
        }

        #[test]
        fn test_truncation_longer_array() {
            #[derive(LightHasher)]
            struct LongTruncatedStruct {
                #[hash]
                data: String,
            }

            let large_data = "a".repeat(64);
            let truncated = LongTruncatedStruct {
                data: large_data.clone(),
            };

            let mut modified_data = large_data.clone();
            modified_data.push('b');
            let truncated2 = LongTruncatedStruct {
                data: modified_data,
            };

            let hash1 = truncated.hash::<Poseidon>().unwrap();
            let hash2 = truncated2.hash::<Poseidon>().unwrap();

            assert_ne!(hash1, hash2);
        }

        #[test]
        fn test_multiple_truncates() {
            #[derive(LightHasher)]
            struct MultiTruncate {
                #[hash]
                data1: String,
                #[hash]
                data2: String,
            }

            let test_struct = MultiTruncate {
                data1: "a".repeat(64),
                data2: "b".repeat(64),
            };

            let hash1 = test_struct.hash::<Poseidon>().unwrap();

            let test_struct2 = MultiTruncate {
                data1: "a".repeat(65),
                data2: "b".repeat(65),
            };

            let hash2 = test_struct2.hash::<Poseidon>().unwrap();
            assert_ne!(
                hash1, hash2,
                "Different data should produce different hashes"
            );
        }

        #[test]
        fn test_nested_with_truncate() {
            #[derive(LightHasher)]
            struct NestedTruncate {
                inner: MyNestedStruct,
                #[hash]
                data: String,
            }

            let nested = create_nested_struct();
            let test_struct = NestedTruncate {
                inner: nested,
                data: "test".to_string(),
            };

            let manual_hash = Poseidon::hashv(&[
                &test_struct.inner.hash::<Poseidon>().unwrap(),
                &light_compressed_account::hash_to_bn254_field_size_be(
                    test_struct.data.try_to_vec().unwrap().as_slice(),
                ),
            ])
            .unwrap();

            let hash = test_struct.hash::<Poseidon>().unwrap();

            // Updated reference hash for BE bytes
            let reference_hash = [
                23, 51, 46, 64, 164, 108, 180, 43, 103, 108, 36, 17, 191, 231, 210, 28, 178, 114,
                188, 37, 143, 15, 165, 109, 154, 241, 33, 210, 172, 108, 10, 33,
            ];

            assert_eq!(hash, manual_hash);
            assert_eq!(hash, reference_hash);
        }
    }

    mod nested {
        use super::*;

        #[test]
        fn test_recursive_nesting() {
            let nested_struct = create_nested_struct();

            #[derive(LightHasher)]
            struct TestNestedStruct {
                one: MyNestedStruct,

                two: MyNestedStruct,
            }

            let test_nested_struct = TestNestedStruct {
                one: nested_struct,
                two: create_nested_struct(),
            };

            let manual_hash = Poseidon::hashv(&[
                &test_nested_struct.one.hash::<Poseidon>().unwrap(),
                &test_nested_struct.two.hash::<Poseidon>().unwrap(),
            ])
            .unwrap();

            assert_eq!(test_nested_struct.hash::<Poseidon>().unwrap(), manual_hash);
        }

        #[test]
        fn test_nested_option() {
            #[derive(LightHasher)]
            struct NestedOption {
                opt: Option<MyNestedStruct>,
            }

            let with_some = NestedOption {
                opt: Some(create_nested_struct()),
            };
            let with_none = NestedOption { opt: None };

            let some_bytes =
                [
                    Poseidon::hash(
                        &with_some.opt.as_ref().unwrap().hash::<Poseidon>().unwrap()[..],
                    )
                    .unwrap(),
                ];
            let none_bytes = [[0u8; 32]];

            assert_eq!(with_some.to_byte_array().unwrap(), some_bytes[0]);
            println!("1");
            assert_eq!(with_none.to_byte_array().unwrap(), none_bytes[0]);
            println!("1");

            let some_hash = with_some.hash::<Poseidon>().unwrap();
            let none_hash = with_none.hash::<Poseidon>().unwrap();

            assert_ne!(some_hash, none_hash);
        }

        #[test]
        fn test_nested_field_count() {
            #[derive(LightHasher)]
            struct InnerMaxFields {
                f1: u64,
                f2: u64,
                f3: u64,
                f4: u64,
                f5: u64,
                f6: u64,
                f7: u64,
                f8: u64,
                f9: u64,
                f10: u64,
                f11: u64,
                f12: u64,
            }

            #[derive(LightHasher)]
            struct OuterWithNested {
                inner: InnerMaxFields,
                other: u64,
            }

            let inner = InnerMaxFields {
                f1: 1,
                f2: 2,
                f3: 3,
                f4: 4,
                f5: 5,
                f6: 6,
                f7: 7,
                f8: 8,
                f9: 9,
                f10: 10,
                f11: 11,
                f12: 12,
            };

            let outer = OuterWithNested { inner, other: 13 };

            assert!(outer.hash::<Poseidon>().is_ok());
        }
    }
}

#[test]
fn test_empty_struct() {
    #[derive(LightHasher)]
    struct EmptyStruct {}

    let empty = EmptyStruct {};
    let result = empty.hash::<Poseidon>();

    assert!(result.is_err(), "Empty struct should fail to hash");
}

#[test]
fn test_poseidon_width_limits() {
    #[derive(LightHasher)]
    struct MaxFields {
        f1: u64,
        f2: u64,
        f3: u64,
        f4: u64,
        f5: u64,
        f6: u64,
        f7: u64,
        f8: u64,
        f9: u64,
        f10: u64,
        f11: u64,
        f12: u64,
    }

    let max_fields = MaxFields {
        f1: 1,
        f2: 2,
        f3: 3,
        f4: 4,
        f5: 5,
        f6: 6,
        f7: 7,
        f8: 8,
        f9: 9,
        f10: 10,
        f11: 11,
        f12: 12,
    };

    assert!(max_fields.hash::<Poseidon>().is_ok());
    let expected_hash = Poseidon::hashv(&[
        1u64.to_be_bytes().as_ref(),
        2u64.to_be_bytes().as_ref(),
        3u64.to_be_bytes().as_ref(),
        4u64.to_be_bytes().as_ref(),
        5u64.to_be_bytes().as_ref(),
        6u64.to_be_bytes().as_ref(),
        7u64.to_be_bytes().as_ref(),
        8u64.to_be_bytes().as_ref(),
        9u64.to_be_bytes().as_ref(),
        10u64.to_be_bytes().as_ref(),
        11u64.to_be_bytes().as_ref(),
        12u64.to_be_bytes().as_ref(),
    ])
    .unwrap();
    assert_eq!(max_fields.hash::<Poseidon>().unwrap(), expected_hash);

    // Doesn't compile because it has too many fields.
    // #[derive(LightHasher)]
    // struct TooManyFields {
    //     f1: u64,
    //     f2: u64,
    //     f3: u64,
    //     f4: u64,
    //     f5: u64,
    //     f6: u64,
    //     f7: u64,
    //     f8: u64,
    //     f9: u64,
    //     f10: u64,
    //     f11: u64,
    //     f12: u64,
    //     f13: u64,
    // }
}

/// Byte arrays over length 31 bytes need to be truncated or a custom ToByteArray impl.
#[test]
fn test_32_array_length() {
    #[derive(LightHasher)]
    struct OversizedArray {
        #[hash]
        data: [u8; 32],
    }

    let test_struct = OversizedArray { data: [255u8; 32] };
    let expected_result =
        Poseidon::hash(&hash_to_bn254_field_size_be(test_struct.data.as_slice())).unwrap();
    let result = test_struct.hash::<Poseidon>().unwrap();
    assert_eq!(result, expected_result);
}

/// doesn't compile without truncate
#[test]
fn test_option_array() {
    #[derive(LightHasher)]
    struct OptionArray {
        #[hash]
        data: Option<[u8; 32]>,
    }

    let test_struct = OptionArray {
        data: Some([0u8; 32]),
    };

    let result = test_struct.hash::<Poseidon>().unwrap();
    assert_ne!(result, [0u8; 32]);
    let expected_result = Poseidon::hash(&hash_to_bn254_field_size_be(&[0u8; 32][..])).unwrap();
    assert_eq!(result, expected_result);
}

mod option_handling {
    use super::{fixtures::*, *};

    #[test]
    fn test_option_hashing_with_reference_values() {
        let account_none = create_account(None);
        let none_hash = account_none.hash::<Poseidon>().unwrap();

        let account_some = create_account(Some(0));
        let some_hash = account_some.hash::<Poseidon>().unwrap();

        // Verify that None and Some(0) have different hashes
        assert_ne!(
            none_hash, some_hash,
            "None and Some(0) should have different hashes"
        );
    }

    #[test]
    fn test_basic_option_variants() {
        #[allow(dead_code)]
        #[derive(LightHasher)]
        struct BasicOptions {
            small: Option<u32>,
            large: Option<u64>,
            #[hash]
            empty_str: Option<String>,
        }

        let test_struct = BasicOptions {
            small: Some(42),
            large: Some(u64::MAX),
            empty_str: Some("".to_string()),
        };

        let none_struct = BasicOptions {
            small: None,
            large: None,
            empty_str: None,
        };

        let manual_bytes = [
            {
                let mut bytes = [0u8; 32];
                bytes[28..].copy_from_slice(&42u32.to_be_bytes());
                bytes[27] = 1; // Prefix with 1 for Some
                bytes
            },
            {
                let mut bytes = [0u8; 32];
                bytes[24..].copy_from_slice(&u64::MAX.to_be_bytes());
                bytes[23] = 1; // Prefix with 1 for Some
                bytes
            },
            light_compressed_account::hash_to_bn254_field_size_be(
                "".try_to_vec().unwrap().as_slice(),
            ),
        ];

        assert_eq!(test_struct.hash::<Poseidon>(), test_struct.to_byte_array());
        let expected_hash = Poseidon::hashv(
            &manual_bytes
                .iter()
                .map(|x| x.as_slice())
                .collect::<Vec<_>>(),
        )
        .unwrap();
        assert_eq!(test_struct.hash::<Poseidon>().unwrap(), expected_hash);
        let test_hash = test_struct.hash::<Poseidon>();
        assert!(test_hash.is_ok());
        let none_hash = none_struct.hash::<Poseidon>().unwrap();

        // Verify that None and Some produce different hashes
        assert_ne!(
            test_hash.unwrap(),
            none_hash,
            "None and Some should have different hashes"
        );
    }

    #[test]
    fn test_truncated_option_variants() {
        #[derive(LightHasher)]
        struct TruncatedOptions {
            #[hash]
            empty_str: Option<String>,
            #[hash]
            short_str: Option<String>,
            #[hash]
            long_str: Option<String>,
            #[hash]
            large_array: Option<[u8; 64]>,
        }

        let test_struct = TruncatedOptions {
            empty_str: Some("".to_string()),
            short_str: Some("test".to_string()),
            long_str: Some("a".repeat(100)),
            large_array: Some([42u8; 64]),
        };

        let none_struct = TruncatedOptions {
            empty_str: None,
            short_str: None,
            long_str: None,
            large_array: None,
        };

        let manual_some_bytes = [
            light_compressed_account::hash_to_bn254_field_size_be(
                "".try_to_vec().unwrap().as_slice(),
            ),
            light_compressed_account::hash_to_bn254_field_size_be(
                "test".try_to_vec().unwrap().as_slice(),
            ),
            light_compressed_account::hash_to_bn254_field_size_be(
                "a".repeat(100).try_to_vec().unwrap().as_slice(),
            ),
            light_compressed_account::hash_to_bn254_field_size_be(
                &test_struct.large_array.unwrap(),
            ),
        ];

        let test_hash = test_struct.hash::<Poseidon>().unwrap();
        let none_hash = none_struct.hash::<Poseidon>().unwrap();
        let expeceted_some_hash = Poseidon::hashv(
            &manual_some_bytes
                .iter()
                .map(|x| x.as_slice())
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let expected_none_hash =
            Poseidon::hashv(&[&[0; 32], &[0; 32], &[0; 32], &[0; 32]]).unwrap();
        assert_eq!(test_hash, expeceted_some_hash);
        assert_eq!(none_hash, expected_none_hash);
        // Updated reference hash for BE bytes
        assert_eq!(
            test_hash,
            [
                26, 206, 86, 217, 69, 163, 110, 158, 101, 48, 167, 203, 138, 17, 126, 43, 203, 82,
                148, 165, 167, 144, 44, 120, 82, 49, 202, 62, 109, 206, 237, 190
            ]
        );
        // Updated reference hash for BE bytes
        assert_eq!(
            none_hash,
            [
                5, 50, 253, 67, 110, 25, 199, 14, 81, 32, 150, 148, 217, 194, 21, 37, 9, 55, 146,
                27, 139, 121, 6, 4, 136, 193, 32, 109, 183, 62, 153, 70
            ]
        );
    }

    #[test]
    fn test_nested_option_variants() {
        #[derive(LightHasher)]
        struct NestedOptions {
            empty_struct: Option<MyNestedStruct>,
            full_struct: Option<MyNestedStruct>,
        }

        let empty_nested = create_zero_nested();
        let full_nested = create_nested_struct();

        let test_struct = NestedOptions {
            empty_struct: Some(empty_nested),
            full_struct: Some(full_nested),
        };

        let none_struct = NestedOptions {
            empty_struct: None,
            full_struct: None,
        };

        let manual_bytes = [
            Poseidon::hash(
                &test_struct
                    .empty_struct
                    .as_ref()
                    .unwrap()
                    .hash::<Poseidon>()
                    .unwrap(),
            )
            .unwrap(),
            Poseidon::hash(
                &test_struct
                    .full_struct
                    .as_ref()
                    .unwrap()
                    .hash::<Poseidon>()
                    .unwrap(),
            )
            .unwrap(),
        ];

        let expected_hash =
            Poseidon::hashv(&manual_bytes.iter().map(|x| x.as_ref()).collect::<Vec<_>>()).unwrap();
        assert_eq!(test_struct.hash::<Poseidon>().unwrap(), expected_hash);
        // Updated reference hash for BE bytes
        assert_eq!(
            test_struct.hash::<Poseidon>().unwrap(),
            [
                38, 207, 53, 149, 51, 139, 156, 60, 155, 207, 232, 222, 177, 238, 31, 130, 136,
                224, 210, 74, 144, 46, 141, 195, 34, 135, 83, 198, 233, 159, 168, 143
            ]
        );
        // Updated reference hash for BE bytes
        assert_eq!(
            none_struct.hash::<Poseidon>().unwrap(),
            [
                32, 152, 245, 251, 158, 35, 158, 171, 60, 234, 195, 242, 123, 129, 228, 129, 220,
                49, 36, 213, 95, 254, 213, 35, 168, 57, 238, 132, 70, 182, 72, 100
            ]
        );
    }

    #[test]
    fn test_mixed_option_combinations() {
        #[derive(LightHasher)]
        struct MixedOptions {
            basic: Option<u32>,
            #[hash]
            truncated_small: Option<String>,
            #[hash]
            truncated_large: Option<[u8; 64]>,

            nested_empty: Option<MyNestedStruct>,

            nested_full: Option<MyNestedStruct>,
        }

        let test_struct = MixedOptions {
            basic: Some(42),
            truncated_small: Some("test".to_string()),
            truncated_large: Some([42u8; 64]),
            nested_empty: Some(MyNestedStruct {
                a: 0,
                b: 0,
                c: "".to_string(),
            }),
            nested_full: Some(create_nested_struct()),
        };

        let partial_struct = MixedOptions {
            basic: Some(42),
            truncated_small: None,
            truncated_large: Some([42u8; 64]),
            nested_empty: None,
            nested_full: Some(create_nested_struct()),
        };

        let none_struct = MixedOptions {
            basic: None,
            truncated_small: None,
            truncated_large: None,
            nested_empty: None,
            nested_full: None,
        };

        let manual_bytes = [
            {
                let mut bytes = [0u8; 32];
                bytes[28..].copy_from_slice(&42u32.to_be_bytes());
                bytes[27] = 1;
                bytes
            },
            light_compressed_account::hash_to_bn254_field_size_be(
                "test".try_to_vec().unwrap().as_slice(),
            ),
            light_compressed_account::hash_to_bn254_field_size_be(&[42u8; 64][..]),
            Poseidon::hash(
                &test_struct
                    .nested_empty
                    .as_ref()
                    .unwrap()
                    .hash::<Poseidon>()
                    .unwrap(),
            )
            .unwrap(),
            Poseidon::hash(
                &test_struct
                    .nested_full
                    .as_ref()
                    .unwrap()
                    .hash::<Poseidon>()
                    .unwrap(),
            )
            .unwrap(),
        ];

        let expected_hash =
            Poseidon::hashv(&manual_bytes.iter().map(|x| x.as_ref()).collect::<Vec<_>>()).unwrap();
        assert_eq!(test_struct.hash::<Poseidon>().unwrap(), expected_hash);
        assert_eq!(
            test_struct.hash::<Poseidon>().unwrap(),
            [
                11, 157, 253, 114, 25, 23, 79, 182, 68, 25, 62, 21, 54, 17, 133, 132, 46, 211, 241,
                153, 207, 76, 61, 164, 177, 148, 208, 53, 50, 179, 26, 213
            ]
        );
        // Updated reference hash for BE bytes
        assert_eq!(
            partial_struct.hash::<Poseidon>().unwrap(),
            [
                37, 131, 136, 26, 175, 106, 143, 121, 184, 59, 76, 126, 15, 134, 111, 55, 194, 38,
                166, 191, 109, 79, 125, 48, 141, 129, 166, 234, 210, 243, 93, 144
            ]
        );
        // Updated reference hash for BE bytes
        assert_eq!(
            none_struct.hash::<Poseidon>().unwrap(),
            [
                32, 102, 190, 65, 190, 190, 108, 175, 126, 7, 147, 96, 171, 225, 79, 191, 145, 24,
                198, 46, 171, 196, 46, 47, 231, 94, 52, 43, 22, 10, 149, 188
            ]
        );
    }

    #[test]
    fn test_nested_struct_with_options() {
        #[derive(LightHasher)]
        struct InnerWithOptions {
            basic: Option<u32>,
            #[hash]
            truncated: Option<String>,
        }

        #[derive(LightHasher)]
        struct OuterStruct {
            inner: InnerWithOptions,
            basic: Option<u64>,
        }

        let test_struct = OuterStruct {
            inner: InnerWithOptions {
                basic: Some(42),
                truncated: Some("test".to_string()),
            },
            basic: Some(u64::MAX),
        };

        let none_struct = OuterStruct {
            inner: InnerWithOptions {
                basic: None,
                truncated: None,
            },
            basic: None,
        };

        let manual_bytes = [test_struct.inner.hash::<Poseidon>().unwrap(), {
            let mut bytes = [0u8; 32];
            bytes[24..].copy_from_slice(&u64::MAX.to_be_bytes());
            bytes[23] = 1;
            bytes
        }];

        let expected_hash = Poseidon::hashv(
            manual_bytes
                .iter()
                .map(|x| x.as_slice())
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .unwrap();
        assert_eq!(test_struct.hash::<Poseidon>().unwrap(), expected_hash);
        assert_eq!(
            test_struct.hash::<Poseidon>().unwrap(),
            [
                12, 235, 222, 198, 73, 228, 229, 31, 235, 53, 206, 115, 238, 91, 183, 135, 185,
                105, 2, 255, 171, 222, 207, 6, 189, 151, 58, 172, 28, 183, 57, 92
            ]
        );
        // Updated reference hash for BE bytes
        assert_eq!(
            none_struct.hash::<Poseidon>().unwrap(),
            [
                23, 83, 82, 87, 94, 164, 86, 13, 119, 230, 225, 21, 182, 59, 41, 174, 42, 2, 191,
                189, 157, 234, 195, 122, 103, 142, 82, 137, 231, 49, 77, 106
            ]
        );
    }
}

mod option_uniqueness {
    use super::*;
    // TODO: split into multi tests to ensure ne is attributable
    #[test]
    fn test_option_value_uniqueness() {
        #[derive(LightHasher)]
        struct OptionTest {
            a: Option<u64>,
            b: Option<u64>,
            #[hash]
            c: Option<String>,

            d: Option<MyNestedStruct>,
        }

        // Test None vs Some(0) produce different hashes
        let none_struct = OptionTest {
            a: None,
            b: None,
            c: None,
            d: None,
        };

        let zero_struct = OptionTest {
            a: Some(0),
            b: Some(0),
            c: Some("".to_string()),
            d: Some(MyNestedStruct {
                a: 0,
                b: 0,
                c: "".to_string(),
            }),
        };

        assert_ne!(
            none_struct.hash::<Poseidon>().unwrap(),
            zero_struct.hash::<Poseidon>().unwrap(),
            "None should hash differently than Some(0)"
        );

        // Test different Some values produce different hashes
        let one_struct = OptionTest {
            a: Some(1),
            b: Some(1),
            c: Some("a".to_string()),
            d: Some(MyNestedStruct {
                a: 1,
                b: 1,
                c: "a".to_string(),
            }),
        };

        assert_ne!(
            zero_struct.hash::<Poseidon>().unwrap(),
            one_struct.hash::<Poseidon>().unwrap(),
            "Different Some values should hash differently"
        );

        // Test partial Some/None combinations
        let partial_struct = OptionTest {
            a: Some(1),
            b: None,
            c: Some("a".to_string()),
            d: None,
        };

        assert_ne!(
            none_struct.hash::<Poseidon>().unwrap(),
            partial_struct.hash::<Poseidon>().unwrap(),
            "Partial Some/None should hash differently than all None"
        );
        assert_ne!(
            one_struct.hash::<Poseidon>().unwrap(),
            partial_struct.hash::<Poseidon>().unwrap(),
            "Partial Some/None should hash differently than all Some"
        );
    }

    #[test]
    fn test_field_order_uniqueness() {
        // Test that field order matters for options
        #[derive(LightHasher)]
        struct OrderTestA {
            first: Option<u64>,
            second: Option<u64>,
        }

        #[derive(LightHasher)]
        struct OrderTestB {
            first: Option<u64>,
            second: Option<u64>,
        }

        let test_a = OrderTestA {
            first: Some(1),
            second: Some(2),
        };

        let test_b = OrderTestB {
            first: Some(2),
            second: Some(1),
        };

        assert_ne!(
            test_a.hash::<Poseidon>().unwrap(),
            test_b.hash::<Poseidon>().unwrap(),
            "Different field order should produce different hashes"
        );

        // Test nested option field order
        #[derive(LightHasher)]
        struct NestedOrderTestA {
            first: Option<MyNestedStruct>,
            second: Option<u64>,
        }

        #[derive(LightHasher)]
        struct NestedOrderTestB {
            first: Option<u64>,

            second: Option<MyNestedStruct>,
        }

        let nested_a = NestedOrderTestA {
            first: Some(MyNestedStruct {
                a: 1,
                b: 2,
                c: "test".to_string(),
            }),
            second: Some(42),
        };

        let nested_b = NestedOrderTestB {
            first: Some(42),
            second: Some(MyNestedStruct {
                a: 1,
                b: 2,
                c: "test".to_string(),
            }),
        };

        assert_ne!(
            nested_a.hash::<Poseidon>().unwrap(),
            nested_b.hash::<Poseidon>().unwrap(),
            "Different nested field order should produce different hashes"
        );
    }

    #[test]
    fn test_truncated_option_uniqueness() {
        #[derive(LightHasher)]
        struct TruncateTest {
            #[hash]
            a: Option<String>,
            #[hash]
            b: Option<[u8; 64]>,
        }

        // Test truncated None vs empty
        let none_struct = TruncateTest { a: None, b: None };

        let empty_struct = TruncateTest {
            a: Some("".to_string()),
            b: Some([0u8; 64]),
        };

        assert_ne!(
            none_struct.hash::<Poseidon>().unwrap(),
            empty_struct.hash::<Poseidon>().unwrap(),
            "Truncated None should hash differently than empty values"
        );

        // Test truncated different values
        let value_struct = TruncateTest {
            a: Some("test".to_string()),
            b: Some([1u8; 64]),
        };

        assert_ne!(
            empty_struct.hash::<Poseidon>().unwrap(),
            value_struct.hash::<Poseidon>().unwrap(),
            "Different truncated values should hash differently"
        );

        // Test truncated long values
        let long_struct = TruncateTest {
            a: Some("a".repeat(100)),
            b: Some([2u8; 64]),
        };

        assert_ne!(
            value_struct.hash::<Poseidon>().unwrap(),
            long_struct.hash::<Poseidon>().unwrap(),
            "Different length truncated values should hash differently"
        );
    }
}

#[test]
fn test_solana_program_pubkey() {
    // Pubkey field
    {
        #[derive(LightHasher)]
        pub struct PubkeyStruct {
            #[hash]
            pub pubkey: Pubkey,
        }
        let pubkey_struct = PubkeyStruct {
            pubkey: Pubkey::new_unique(),
        };

        let manual_hash = Poseidon::hash(
            light_compressed_account::hash_to_bn254_field_size_be(pubkey_struct.pubkey.as_ref())
                .as_slice(),
        )
        .unwrap();
        let manual_hash_borsh = Poseidon::hash(
            light_compressed_account::hash_to_bn254_field_size_be(
                pubkey_struct.pubkey.try_to_vec().unwrap().as_slice(),
            )
            .as_slice(),
        )
        .unwrap();
        let hash = pubkey_struct.hash::<Poseidon>().unwrap();
        assert_eq!(manual_hash, hash);
        assert_eq!(manual_hash_borsh, hash);
    }
    // Option<Pubkey>
    {
        #[derive(LightHasher)]
        pub struct PubkeyStruct {
            #[hash]
            pub pubkey: Option<Pubkey>,
        }
        // Some
        {
            let pubkey_struct = PubkeyStruct {
                pubkey: Some(Pubkey::new_unique()),
            };
            let manual_bytes = pubkey_struct.pubkey.unwrap().try_to_vec().unwrap();

            let manual_hash = Poseidon::hash(
                light_compressed_account::hash_to_bn254_field_size_be(manual_bytes.as_slice())
                    .as_slice(),
            )
            .unwrap();
            let hash = pubkey_struct.hash::<Poseidon>().unwrap();
            assert_eq!(manual_hash, hash);

            // Sha256
            let mut manual_hash = Sha256::hash(
                light_compressed_account::hash_to_bn254_field_size_be(manual_bytes.as_slice())
                    .as_slice(),
            )
            .unwrap();
            // Apply truncation for non-Poseidon hashers
            if Sha256::ID != 0 {
                manual_hash[0] = 0;
            }
            let hash = pubkey_struct.hash::<Sha256>().unwrap();
            assert_eq!(manual_hash, hash);
        }
        // None
        {
            let pubkey_struct = PubkeyStruct { pubkey: None };
            let manual_hash = Poseidon::hash([0u8; 32].as_slice()).unwrap();
            let hash = pubkey_struct.hash::<Poseidon>().unwrap();
            assert_eq!(manual_hash, hash);

            // Sha256
            let mut manual_hash = Sha256::hash([0u8; 32].as_slice()).unwrap();
            // Apply truncation for non-Poseidon hashers
            if Sha256::ID != 0 {
                manual_hash[0] = 0;
            }
            let hash = pubkey_struct.hash::<Sha256>().unwrap();
            assert_eq!(manual_hash, hash);
        }
    }
    // Vec<Pubkey>
    {
        #[derive(LightHasher)]
        pub struct PubkeyStruct {
            #[hash]
            pub pubkey: Vec<Pubkey>,
        }
        let pubkey_vec = (0..3).map(|_| Pubkey::new_unique()).collect::<Vec<_>>();
        let pubkey_struct = PubkeyStruct { pubkey: pubkey_vec };
        let manual_bytes = pubkey_struct.pubkey.try_to_vec().unwrap();

        let manual_hash = Poseidon::hash(
            light_compressed_account::hash_to_bn254_field_size_be(manual_bytes.as_slice())
                .as_slice(),
        )
        .unwrap();
        let hash = pubkey_struct.hash::<Poseidon>().unwrap();
        assert_eq!(manual_hash, hash);

        // Sha256
        let mut manual_hash = Sha256::hash(
            light_compressed_account::hash_to_bn254_field_size_be(manual_bytes.as_slice())
                .as_slice(),
        )
        .unwrap();
        // Apply truncation for non-Poseidon hashers
        if Sha256::ID != 0 {
            manual_hash[0] = 0;
        }
        let hash = pubkey_struct.hash::<Sha256>().unwrap();
        assert_eq!(manual_hash, hash);
    }
    // Vec<Option<Pubkey>>
    {
        #[derive(LightHasher)]
        pub struct PubkeyStruct {
            #[hash]
            pub pubkey: Vec<Option<Pubkey>>,
        }
        // Some
        {
            let pubkey_vec = (0..3)
                .map(|_| Some(Pubkey::new_unique()))
                .collect::<Vec<_>>();
            let pubkey_struct = PubkeyStruct { pubkey: pubkey_vec };
            let manual_bytes = pubkey_struct.pubkey.try_to_vec().unwrap();

            let manual_hash = Poseidon::hash(
                light_compressed_account::hash_to_bn254_field_size_be(manual_bytes.as_slice())
                    .as_slice(),
            )
            .unwrap();
            let hash = pubkey_struct.hash::<Poseidon>().unwrap();
            assert_eq!(manual_hash, hash);

            // Sha256
            let mut manual_hash = Sha256::hash(
                light_compressed_account::hash_to_bn254_field_size_be(manual_bytes.as_slice())
                    .as_slice(),
            )
            .unwrap();
            // Apply truncation for non-Poseidon hashers
            if Sha256::ID != 0 {
                manual_hash[0] = 0;
            }
            let hash = pubkey_struct.hash::<Sha256>().unwrap();
            assert_eq!(manual_hash, hash);
        }
        // None
        {
            let pubkey_vec = (0..3).map(|_| None).collect::<Vec<_>>();
            let pubkey_struct = PubkeyStruct { pubkey: pubkey_vec };
            let manual_bytes = pubkey_struct.pubkey.try_to_vec().unwrap();
            let manual_hash = Poseidon::hash(
                light_compressed_account::hash_to_bn254_field_size_be(manual_bytes.as_slice())
                    .as_slice(),
            )
            .unwrap();
            let hash = pubkey_struct.hash::<Poseidon>().unwrap();
            assert_eq!(manual_hash, hash);

            // Sha256
            let mut manual_hash = Sha256::hash(
                light_compressed_account::hash_to_bn254_field_size_be(manual_bytes.as_slice())
                    .as_slice(),
            )
            .unwrap();
            // Apply truncation for non-Poseidon hashers
            if Sha256::ID != 0 {
                manual_hash[0] = 0;
            }
            let hash = pubkey_struct.hash::<Sha256>().unwrap();
            assert_eq!(manual_hash, hash);
        }
    }
}

#[test]
fn test_light_hasher_sha_macro() {
    use light_sdk_macros::LightHasherSha;

    // Test struct with many fields that would exceed Poseidon's limit
    #[derive(LightHasherSha, BorshSerialize, BorshDeserialize, Clone)]
    struct LargeShaStruct {
        pub field1: u64,
        pub field2: u64,
        pub field3: u64,
        pub field4: u64,
        pub field5: u64,
        pub field6: u64,
        pub field7: u64,
        pub field8: u64,
        pub field9: u64,
        pub field10: u64,
        pub field11: u64,
        pub field12: u64,
        pub field13: u64,
        pub field14: u64,
        pub field15: u64,
        pub owner: Pubkey,
        pub authority: Pubkey,
    }

    let test_struct = LargeShaStruct {
        field1: 1,
        field2: 2,
        field3: 3,
        field4: 4,
        field5: 5,
        field6: 6,
        field7: 7,
        field8: 8,
        field9: 9,
        field10: 10,
        field11: 11,
        field12: 12,
        field13: 13,
        field14: 14,
        field15: 15,
        owner: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
    };

    // Verify the hash matches manual SHA256 hashing
    let bytes = test_struct.try_to_vec().unwrap();
    let mut ref_hash = Sha256::hash(bytes.as_slice()).unwrap();

    // Apply truncation for non-Poseidon hashers (ID != 0)
    if Sha256::ID != 0 {
        ref_hash[0] = 0;
    }

    // Test with SHA256 hasher
    let hash_result = test_struct.hash::<Sha256>().unwrap();
    assert_eq!(
        hash_result, ref_hash,
        "SHA256 hash should match manual hash"
    );

    // Test ToByteArray implementation
    let byte_array_result = test_struct.to_byte_array().unwrap();
    assert_eq!(
        byte_array_result, ref_hash,
        "ToByteArray should match SHA256 hash"
    );

    // Test another struct with different values
    let test_struct2 = LargeShaStruct {
        field1: 100,
        field2: 200,
        field3: 300,
        field4: 400,
        field5: 500,
        field6: 600,
        field7: 700,
        field8: 800,
        field9: 900,
        field10: 1000,
        field11: 1100,
        field12: 1200,
        field13: 1300,
        field14: 1400,
        field15: 1500,
        owner: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
    };

    let bytes2 = test_struct2.try_to_vec().unwrap();
    let mut ref_hash2 = Sha256::hash(bytes2.as_slice()).unwrap();

    if Sha256::ID != 0 {
        ref_hash2[0] = 0;
    }

    let hash_result2 = test_struct2.hash::<Sha256>().unwrap();
    assert_eq!(
        hash_result2, ref_hash2,
        "Second SHA256 hash should match manual hash"
    );

    // Ensure different structs produce different hashes
    assert_ne!(
        hash_result, hash_result2,
        "Different structs should produce different hashes"
    );
}

// Option<BorshStruct>
#[test]
fn test_borsh() {
    #[derive(BorshDeserialize, BorshSerialize)]
    pub struct BorshStruct {
        data: [u8; 34],
    }
    impl Default for BorshStruct {
        fn default() -> Self {
            Self { data: [1u8; 34] }
        }
    }
    // Option Borsh
    {
        #[derive(LightHasher)]
        pub struct PubkeyStruct {
            #[hash]
            pub pubkey: Option<BorshStruct>,
        }
        // Some
        {
            let pubkey_struct = PubkeyStruct {
                pubkey: Some(BorshStruct::default()),
            };
            let manual_bytes = pubkey_struct.pubkey.as_ref().unwrap().try_to_vec().unwrap();

            let manual_hash = Poseidon::hash(
                light_compressed_account::hash_to_bn254_field_size_be(manual_bytes.as_slice())
                    .as_slice(),
            )
            .unwrap();
            let hash = pubkey_struct.hash::<Poseidon>().unwrap();
            assert_eq!(manual_hash, hash);

            // Sha256
            let mut manual_hash = Sha256::hash(
                light_compressed_account::hash_to_bn254_field_size_be(manual_bytes.as_slice())
                    .as_slice(),
            )
            .unwrap();
            // Apply truncation for non-Poseidon hashers
            if Sha256::ID != 0 {
                manual_hash[0] = 0;
            }
            let hash = pubkey_struct.hash::<Sha256>().unwrap();
            assert_eq!(manual_hash, hash);
        }
        // None
        {
            let pubkey_struct = PubkeyStruct { pubkey: None };
            let manual_hash = Poseidon::hash([0u8; 32].as_slice()).unwrap();
            let hash = pubkey_struct.hash::<Poseidon>().unwrap();
            assert_eq!(manual_hash, hash);

            // Sha256
            let mut manual_hash = Sha256::hash([0u8; 32].as_slice()).unwrap();
            // Apply truncation for non-Poseidon hashers
            if Sha256::ID != 0 {
                manual_hash[0] = 0;
            }
            let hash = pubkey_struct.hash::<Sha256>().unwrap();
            assert_eq!(manual_hash, hash);
        }
    }
    // Borsh
    {
        #[derive(LightHasher)]
        pub struct PubkeyStruct {
            #[hash]
            pub pubkey: BorshStruct,
        }

        let pubkey_struct = PubkeyStruct {
            pubkey: BorshStruct::default(),
        };
        let manual_bytes = pubkey_struct.pubkey.try_to_vec().unwrap();

        let manual_hash = Poseidon::hash(
            light_compressed_account::hash_to_bn254_field_size_be(manual_bytes.as_slice())
                .as_slice(),
        )
        .unwrap();
        let hash = pubkey_struct.hash::<Poseidon>().unwrap();
        assert_eq!(manual_hash, hash);

        // Sha256
        let mut manual_hash = Sha256::hash(
            light_compressed_account::hash_to_bn254_field_size_be(manual_bytes.as_slice())
                .as_slice(),
        )
        .unwrap();
        // Apply truncation for non-Poseidon hashers
        if Sha256::ID != 0 {
            manual_hash[0] = 0;
        }
        let hash = pubkey_struct.hash::<Sha256>().unwrap();
        assert_eq!(manual_hash, hash);
    }
}

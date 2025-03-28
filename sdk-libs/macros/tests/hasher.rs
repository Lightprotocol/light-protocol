use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use light_compressed_account::hash_to_bn254_field_size_be;
use light_hasher::{to_byte_array::ToByteArray, DataHasher, Hasher, Poseidon};
use light_sdk_macros::LightHasher;

#[derive(LightHasher, Clone)]
pub struct MyAccount {
    pub a: bool,
    pub b: u64,
    pub c: MyNestedStruct,
    #[hash]
    pub d: [u8; 32],
    pub f: Option<usize>,
}

#[derive(LightHasher, Clone)]
pub struct TruncateVec {
    #[hash]
    pub d: Vec<u8>,
}

// TODO: test flatten twice
// TODO: fix #[flatten] pub d: Vec<u64>,
#[derive(LightHasher, Clone)]
pub struct FlatMyAccount {
    pub a: bool,
    pub b: u64,
    #[flatten]
    pub c: MyNestedStruct,
    pub d: [u8; 31],
    pub f: Option<usize>,
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

    let field_arrays = account.to_byte_arrays::<5>(); // MyAccount has 5 fields
    assert!(field_arrays.is_ok());
    let arrays = field_arrays.unwrap();
    assert_eq!(arrays.len(), 5);
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

    pub fn create_account(f: Option<usize>) -> MyAccount {
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
            light_compressed_account::hash_to_bn254_field_size_be(nested_struct.c.as_bytes())
                .to_vec(),
        ];

        let nested_bytes: Vec<&[u8]> = manual_nested_bytes.iter().map(|v| v.as_slice()).collect();
        let manual_nested_hash = Poseidon::hashv(&nested_bytes).unwrap();

        let nested_reference_hash = [
            29, 149, 180, 150, 84, 0, 186, 120, 253, 64, 185, 187, 93, 26, 252, 138, 85, 232, 255,
            144, 8, 35, 150, 142, 250, 235, 197, 57, 73, 205, 107, 35,
        ];
        let nested_hash_result = nested_struct.hash::<Poseidon>().unwrap();

        // assert_eq!(
        //     nested_struct.to_byte_array(),
        //     to_array_vec(manual_nested_bytes)
        // );
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

        // assert_eq!(account.to_byte_array(), to_array_vec(manual_account_bytes));
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
            3, 127, 219, 54, 250, 11, 162, 52, 88, 137, 5, 102, 150, 39, 131, 175, 28, 26, 59, 122,
            170, 39, 222, 21, 191, 172, 167, 36, 200, 218, 131, 43,
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
                &light_compressed_account::hash_to_bn254_field_size_be(test_struct.data.as_bytes()),
            ])
            .unwrap();

            let hash = test_struct.hash::<Poseidon>().unwrap();

            // Updated reference hash for BE bytes
            let reference_hash = [
                48, 9, 163, 28, 177, 59, 200, 170, 26, 181, 224, 191, 251, 157, 98, 198, 27, 195,
                113, 222, 10, 44, 14, 23, 96, 127, 53, 130, 93, 116, 101, 14,
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

            assert_eq!(with_some.to_byte_arrays().unwrap(), some_bytes);
            println!("1");
            assert_eq!(with_none.to_byte_arrays().unwrap(), none_bytes);
            println!("1");
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
    assert_ne!(result, [0u8; 32],);
    let expected_result = Poseidon::hash(&hash_to_bn254_field_size_be(&[0u8; 32])).unwrap();
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
                bytes[27] = 1; // Suffix with 1 for Some
                bytes
            },
            {
                let mut bytes = [0u8; 32];
                bytes[24..].copy_from_slice(&u64::MAX.to_be_bytes());
                bytes[23] = 1; // Suffix with 1 for Some
                bytes
            },
            light_compressed_account::hash_to_bn254_field_size_be("".as_bytes()),
        ];

        assert_eq!(test_struct.to_byte_arrays().unwrap(), manual_bytes);
        assert_eq!(test_struct.hash::<Poseidon>(), test_struct.to_byte_array());
        assert_eq!(none_struct.to_byte_arrays().unwrap(), [[0u8; 32]; 3]);
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
            light_compressed_account::hash_to_bn254_field_size_be("".as_bytes()),
            light_compressed_account::hash_to_bn254_field_size_be("test".as_bytes()),
            light_compressed_account::hash_to_bn254_field_size_be("a".repeat(100).as_bytes()),
            light_compressed_account::hash_to_bn254_field_size_be(
                &test_struct.large_array.unwrap(),
            ),
        ];

        assert_eq!(test_struct.to_byte_arrays().unwrap(), manual_some_bytes);
        assert_eq!(
            none_struct.to_byte_arrays().unwrap(),
            [[0; 32], [0; 32], [0; 32], [0; 32]]
        );

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
                37, 226, 47, 85, 30, 108, 236, 252, 82, 79, 97, 139, 68, 236, 199, 14, 159, 239,
                210, 122, 191, 200, 142, 120, 143, 34, 153, 144, 98, 192, 152, 24
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

        assert_eq!(test_struct.to_byte_arrays().unwrap(), manual_bytes);
        assert_eq!(none_struct.to_byte_arrays().unwrap(), [[0u8; 32]; 2]);
        let expected_hash =
            Poseidon::hashv(&manual_bytes.iter().map(|x| x.as_ref()).collect::<Vec<_>>()).unwrap();
        assert_eq!(test_struct.hash::<Poseidon>().unwrap(), expected_hash);
        // Updated reference hash for BE bytes
        assert_eq!(
            test_struct.hash::<Poseidon>().unwrap(),
            [
                30, 167, 186, 136, 126, 58, 22, 82, 142, 231, 244, 238, 4, 63, 122, 12, 143, 249,
                99, 99, 49, 50, 102, 134, 56, 152, 179, 15, 121, 133, 132, 250
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
            light_compressed_account::hash_to_bn254_field_size_be("test".as_bytes()),
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

        assert_eq!(test_struct.to_byte_arrays().unwrap(), manual_bytes);
        assert_eq!(none_struct.to_byte_arrays().unwrap(), [[0u8; 32]; 5]);
        let expected_hash =
            Poseidon::hashv(&manual_bytes.iter().map(|x| x.as_ref()).collect::<Vec<_>>()).unwrap();
        assert_eq!(test_struct.hash::<Poseidon>().unwrap(), expected_hash);
        assert_eq!(
            test_struct.hash::<Poseidon>().unwrap(),
            [
                45, 31, 51, 201, 30, 211, 34, 233, 167, 221, 208, 213, 70, 50, 209, 82, 124, 34,
                243, 207, 243, 56, 160, 65, 104, 158, 136, 230, 221, 244, 61, 162
            ]
        );
        // Updated reference hash for BE bytes
        assert_eq!(
            partial_struct.hash::<Poseidon>().unwrap(),
            [
                15, 120, 15, 160, 138, 178, 90, 138, 221, 251, 187, 167, 167, 54, 101, 131, 213,
                57, 166, 61, 53, 18, 139, 45, 97, 154, 198, 73, 4, 108, 63, 161
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

        assert_eq!(test_struct.to_byte_arrays().unwrap(), manual_bytes);
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
                13, 177, 80, 37, 34, 177, 88, 80, 222, 177, 167, 172, 26, 80, 208, 203, 235, 203,
                105, 234, 81, 170, 209, 7, 35, 90, 221, 103, 97, 78, 44, 101
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
fn test_truncate_byte_representation() {
    #[derive(LightHasher)]
    struct TruncateTest {
        #[hash]
        data: String,
        #[hash]
        array: [u8; 64],
    }

    let test_struct = TruncateTest {
        data: "test".to_string(),
        array: [42u8; 64],
    };

    let manual_bytes = [
        light_compressed_account::hash_to_bn254_field_size_be(test_struct.data.as_bytes()),
        light_compressed_account::hash_to_bn254_field_size_be(&test_struct.array),
    ];

    assert_eq!(test_struct.to_byte_arrays().unwrap(), manual_bytes);
}

#[test]
fn test_byte_representation_combinations() {
    #[derive(LightHasher)]
    struct BasicOption {
        opt: Option<u64>,
    }

    let with_some = BasicOption { opt: Some(42) };
    let with_none = BasicOption { opt: None };

    let manual_some = [{
        let mut bytes = [0u8; 32];
        bytes[24..].copy_from_slice(&42u64.to_be_bytes());
        bytes[23] = 1;
        bytes
    }];
    let manual_none = [[0u8; 32]];
    assert_eq!(with_some.to_byte_arrays().unwrap(), manual_some);
    assert_eq!(with_none.to_byte_arrays().unwrap(), manual_none);

    // Option + HashToFieldSize
    #[derive(LightHasher)]
    struct OptionTruncate {
        #[hash]
        opt: Option<String>,
    }

    let with_some = OptionTruncate {
        opt: Some("test".to_string()),
    };
    let with_none = OptionTruncate { opt: None };

    let manual_some = [light_compressed_account::hash_to_bn254_field_size_be(
        "test".as_bytes(),
    )];
    let manual_none = [[0u8; 32]];

    assert_eq!(with_some.to_byte_arrays().unwrap(), manual_some);
    assert_eq!(with_none.to_byte_arrays().unwrap(), manual_none);

    // Option + Nested
    #[derive(LightHasher)]
    struct OptionNested {
        opt: Option<MyNestedStruct>,
    }

    let nested = MyNestedStruct {
        a: 1,
        b: 2,
        c: "test".to_string(),
    };
    let with_some = OptionNested {
        opt: Some(nested.clone()),
    };
    let with_none = OptionNested { opt: None };

    let manual_some =
        [Poseidon::hash(&with_some.opt.as_ref().unwrap().hash::<Poseidon>().unwrap()).unwrap()];
    let manual_none = [[0u8; 32]];

    assert_eq!(with_some.to_byte_arrays().unwrap(), manual_some);
    assert_eq!(with_none.to_byte_arrays().unwrap(), manual_none);

    // All combined
    #[derive(LightHasher)]
    struct Combined {
        basic: Option<u64>,
        #[hash]
        trunc: Option<String>,

        nest: Option<MyNestedStruct>,
    }

    let with_some = Combined {
        basic: Some(42),
        trunc: Some("test".to_string()),
        nest: Some(nested),
    };
    let with_none = Combined {
        basic: None,
        trunc: None,
        nest: None,
    };

    let manual_some = [
        {
            let mut bytes = [0u8; 32];
            bytes[24..].copy_from_slice(&42u64.to_be_bytes());
            bytes[23] = 1;
            bytes
        },
        light_compressed_account::hash_to_bn254_field_size_be("test".as_bytes()),
        Poseidon::hash(&with_some.nest.as_ref().unwrap().hash::<Poseidon>().unwrap()).unwrap(),
    ];
    let manual_none = [[0u8; 32]; 3];

    assert_eq!(with_some.to_byte_arrays().unwrap(), manual_some);
    assert_eq!(with_none.to_byte_arrays().unwrap(), manual_none);
}

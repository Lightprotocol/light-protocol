use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use light_hasher::{bytes::AsByteVec, DataHasher, Hasher, Poseidon};
use light_sdk_macros::LightHasher;

#[derive(LightHasher)]
pub struct MyAccount {
    pub a: bool,
    pub b: u64,
    #[nested]
    pub c: MyNestedStruct,
    #[truncate]
    pub d: [u8; 32],
    #[skip]
    pub e: MyNestedNonHashableStruct,
    pub f: Option<usize>,
}

#[derive(LightHasher)]
pub struct MyNestedStruct {
    pub a: i32,
    pub b: u32,
    #[truncate]
    pub c: String,
}

pub struct MyNestedNonHashableStruct {
    pub a: PhantomData<()>,
    pub b: Rc<RefCell<usize>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn create_test_nested_struct() -> MyNestedStruct {
        MyNestedStruct {
            a: i32::MIN,
            b: u32::MAX,
            c: "wao".to_string(),
        }
    }

    fn create_test_account(f: Option<usize>) -> MyAccount {
        MyAccount {
            a: true,
            b: u64::MAX,
            c: create_test_nested_struct(),
            d: [u8::MAX; 32],
            e: MyNestedNonHashableStruct {
                a: PhantomData,
                b: Rc::new(RefCell::new(usize::MAX)),
            },
            f,
        }
    }

    #[test]
    fn test_byte_representation() {
        let nested_struct = create_test_nested_struct();
        let account = create_test_account(Some(42));

        let manual_nested_bytes: Vec<Vec<u8>> = vec![
            nested_struct.a.to_le_bytes().to_vec(),
            nested_struct.b.to_le_bytes().to_vec(),
            light_utils::hash_to_bn254_field_size_be(nested_struct.c.as_bytes())
                .unwrap()
                .0
                .to_vec(),
        ];

        let nested_bytes: Vec<&[u8]> = manual_nested_bytes.iter().map(|v| v.as_slice()).collect();
        let manual_nested_hash = Poseidon::hashv(&nested_bytes).unwrap();

        let nested_reference_hash = [
            6, 124, 124, 67, 65, 28, 217, 111, 86, 61, 85, 93, 118, 177, 69, 25, 117, 70, 49, 96,
            28, 232, 61, 133, 166, 55, 135, 210, 49, 27, 114, 93,
        ];
        let nested_hash_result = nested_struct.hash::<Poseidon>().unwrap();

        assert_eq!(nested_struct.as_byte_vec(), manual_nested_bytes);
        assert_eq!(nested_hash_result, nested_reference_hash);
        assert_eq!(manual_nested_hash, nested_reference_hash);
        assert_eq!(nested_hash_result, manual_nested_hash);

        let manual_account_bytes: Vec<Vec<u8>> = vec![
            vec![u8::from(account.a)],
            account.b.to_le_bytes().to_vec(),
            account.c.hash::<Poseidon>().unwrap().to_vec(),
            light_utils::hash_to_bn254_field_size_be(&account.d)
                .unwrap()
                .0
                .to_vec(),
            {
                let mut bytes = vec![1u8]; // Prefix with 1 for Some
                bytes.extend_from_slice(&account.f.unwrap().to_le_bytes());
                bytes
            },
        ];

        let account_bytes: Vec<&[u8]> = manual_account_bytes.iter().map(|v| v.as_slice()).collect();
        let manual_account_hash = Poseidon::hashv(&account_bytes).unwrap();

        let account_hash_result = account.hash::<Poseidon>().unwrap();

        assert_eq!(account.as_byte_vec(), manual_account_bytes);
        assert_eq!(account_hash_result, manual_account_hash);
    }

    #[test]
    fn test_boundary_values() {
        let nested = MyNestedStruct {
            a: 0,
            b: 0,
            c: "".to_string(),
        };

        let zero_account = MyAccount {
            a: false,
            b: 0,
            c: nested,
            d: [0; 32],
            e: MyNestedNonHashableStruct {
                a: PhantomData,
                b: Rc::new(RefCell::new(0)),
            },
            f: Some(0),
        };

        let hash = zero_account.hash::<Poseidon>().unwrap();
        assert_eq!(hash.len(), 32);
    }

    mod attribute_tests {
        use super::*;

        #[test]
        fn test_array_truncation() {
            #[derive(LightHasher)]
            struct TruncatedStruct {
                #[truncate]
                data: [u8; 32],
            }

            let single = TruncatedStruct {
                data: [1u8; 32], // All 1s
            };

            let double = TruncatedStruct {
                data: [2u8; 32], // All 2s
            };

            let mixed = TruncatedStruct {
                data: {
                    let mut data = [1u8; 32];
                    data[0] = 2u8; // Change first byte
                    data
                },
            };

            let single_hash = single.hash::<Poseidon>().unwrap();

            let double_hash = double.hash::<Poseidon>().unwrap();

            let mixed_hash = mixed.hash::<Poseidon>().unwrap();

            assert_ne!(single_hash, double_hash);
            assert_ne!(single_hash, mixed_hash);
            assert_ne!(double_hash, mixed_hash);
        }

        #[test]
        fn test_truncation_longer_array() {
            #[derive(LightHasher)]
            struct LongTruncatedStruct {
                #[truncate]
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
                #[truncate]
                data1: String,
                #[truncate]
                data2: String,
            }

            let test_struct = MultiTruncate {
                data1: "a".repeat(64),
                data2: "b".repeat(64),
            };

            let hash1 = test_struct.hash::<Poseidon>().unwrap();

            // Modifying data after truncation point shouldn't affect hash
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
                #[nested]
                inner: MyNestedStruct,
                #[truncate]
                data: String,
            }

            let nested = create_test_nested_struct();
            let test_struct = NestedTruncate {
                inner: nested,
                data: "test".to_string(),
            };
            // Manual implementation for comparison
            let manual_hash = Poseidon::hashv(&[
                &test_struct.inner.hash::<Poseidon>().unwrap(),
                &light_utils::hash_to_bn254_field_size_be(&test_struct.data.as_bytes())
                    .unwrap()
                    .0,
            ])
            .unwrap();

            let hash = test_struct.hash::<Poseidon>().unwrap();

            let reference_hash = [
                8, 229, 6, 141, 101, 145, 175, 89, 106, 135, 77, 136, 167, 140, 48, 31, 80, 113,
                227, 69, 129, 37, 64, 79, 241, 231, 182, 0, 208, 8, 112, 238,
            ];

            assert_eq!(hash, reference_hash);
            assert_eq!(hash, manual_hash);
        }
    }

    mod nesting_tests {
        use super::*;

        #[test]
        fn test_recursive_nesting() {
            let nested_struct = create_test_nested_struct();

            #[derive(LightHasher)]
            struct TestNestedStruct {
                #[nested]
                one: MyNestedStruct,
                #[nested]
                two: MyNestedStruct,
            }

            let test_nested_struct = TestNestedStruct {
                one: nested_struct,
                two: create_test_nested_struct(),
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
                #[nested]
                opt: Option<MyNestedStruct>,
            }

            let with_some = NestedOption {
                opt: Some(create_test_nested_struct()),
            };
            let with_none = NestedOption { opt: None };

            let some_bytes = vec![with_some
                .opt
                .as_ref()
                .unwrap()
                .hash::<Poseidon>()
                .unwrap()
                .to_vec()];
            let none_bytes = vec![vec![0]];

            assert_eq!(with_some.as_byte_vec(), some_bytes);
            assert_eq!(with_none.as_byte_vec(), none_bytes);

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
                #[nested]
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

    // Edge cases
    mod limits_tests {
        use super::*;

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
            // Max allowed: 12 fields
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

            // 13 fields
            #[derive(LightHasher)]
            struct TooManyFields {
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
                f13: u64,
            }

            let too_many = TooManyFields {
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
                f13: 13,
            };

            assert!(too_many.hash::<Poseidon>().is_err());
        }
    }

    #[test]
    fn test_option_hashing_with_reference_values() {
        let account_none = create_test_account(None);
        assert_eq!(
            account_none.hash::<Poseidon>().unwrap(),
            [
                4, 224, 3, 136, 193, 49, 211, 217, 220, 249, 4, 20, 151, 165, 162, 5, 50, 83, 250,
                154, 142, 223, 47, 228, 106, 248, 52, 178, 16, 167, 76, 71
            ]
        );

        let account_some = create_test_account(Some(0));
        assert_eq!(
            account_some.hash::<Poseidon>().unwrap(),
            [
                39, 77, 212, 128, 109, 176, 236, 140, 193, 215, 20, 225, 100, 163, 117, 142, 64,
                175, 8, 76, 111, 97, 176, 17, 232, 235, 5, 146, 113, 75, 85, 244
            ]
        );
    }

    #[test]
    fn test_basic_option_variants() {
        #[allow(dead_code)]
        #[derive(LightHasher)]
        struct BasicOptions {
            small: Option<u32>,
            large: Option<u64>,
            empty_str: Option<String>,
            #[skip]
            long_str: Option<String>,
        }

        let test_struct = BasicOptions {
            small: Some(42),
            large: Some(u64::MAX),
            empty_str: Some("".to_string()),
            long_str: Some("a".repeat(100)),
        };

        let none_struct = BasicOptions {
            small: None,
            large: None,
            empty_str: None,
            long_str: None,
        };

        // Manual implementation
        let manual_bytes = vec![
            {
                let mut bytes = vec![1u8];
                bytes.extend_from_slice(&42u32.to_le_bytes());
                bytes
            },
            {
                let mut bytes = vec![1u8];
                bytes.extend_from_slice(&u64::MAX.to_le_bytes());
                bytes
            },
            {
                let mut bytes = vec![1u8];
                bytes.extend_from_slice("".as_bytes());
                bytes
            },
        ];

        assert_eq!(test_struct.as_byte_vec(), manual_bytes);
        assert_eq!(none_struct.as_byte_vec(), vec![vec![0], vec![0], vec![0]]);

        let test_hash = test_struct.hash::<Poseidon>();
        assert!(test_hash.is_ok());
        let none_hash = none_struct.hash::<Poseidon>().unwrap();
        assert_eq!(
            test_hash.unwrap(),
            [
                14, 35, 10, 94, 19, 216, 17, 115, 253, 52, 79, 106, 183, 242, 74, 158, 36, 37, 248,
                81, 104, 231, 89, 188, 4, 214, 34, 177, 232, 240, 255, 166
            ]
        );
        assert_eq!(
            none_hash,
            [
                11, 193, 136, 210, 125, 204, 234, 220, 29, 207, 182, 175, 10, 122, 240, 143, 226,
                134, 78, 236, 236, 150, 197, 174, 124, 238, 109, 179, 27, 165, 153, 170
            ]
        );

        #[derive(LightHasher)]
        struct BasicOptionsNoSkip {
            small: Option<u32>,
            large: Option<u64>,
            empty_str: Option<String>,
            long_str: Option<String>,
        }

        let test_struct_no_skip = BasicOptionsNoSkip {
            small: Some(42),
            large: Some(u64::MAX),
            empty_str: Some("".to_string()),
            long_str: Some("a".repeat(100)),
        };

        // Hash should fail due to long string
        assert!(test_struct_no_skip.hash::<Poseidon>().is_err());
    }

    #[test]
    fn test_truncated_option_variants() {
        #[derive(LightHasher)]
        struct TruncatedOptions {
            #[truncate]
            empty_str: Option<String>,
            #[truncate]
            short_str: Option<String>,
            #[truncate]
            long_str: Option<String>,
            #[truncate]
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

        // Manual implementation for Some values
        let manual_some_bytes = vec![
            light_utils::hash_to_bn254_field_size_be("".as_bytes())
                .unwrap()
                .0
                .to_vec(),
            light_utils::hash_to_bn254_field_size_be("test".as_bytes())
                .unwrap()
                .0
                .to_vec(),
            light_utils::hash_to_bn254_field_size_be("a".repeat(100).as_bytes())
                .unwrap()
                .0
                .to_vec(),
            light_utils::hash_to_bn254_field_size_be(&test_struct.large_array.unwrap())
                .unwrap()
                .0
                .to_vec(),
        ];

        assert_eq!(test_struct.as_byte_vec(), manual_some_bytes);
        assert_eq!(
            none_struct.as_byte_vec(),
            vec![vec![0], vec![0], vec![0], vec![0]]
        );

        // Get the actual hashes
        let test_hash = test_struct.hash::<Poseidon>().unwrap();
        let none_hash = none_struct.hash::<Poseidon>().unwrap();
        assert_eq!(
            test_hash,
            [
                37, 226, 47, 85, 30, 108, 236, 252, 82, 79, 97, 139, 68, 236, 199, 14, 159, 239,
                210, 122, 191, 200, 142, 120, 143, 34, 153, 144, 98, 192, 152, 24
            ]
        );
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
            #[nested]
            empty_struct: Option<MyNestedStruct>,
            #[nested]
            full_struct: Option<MyNestedStruct>,
        }

        let empty_nested = MyNestedStruct {
            a: 0,
            b: 0,
            c: "".to_string(),
        };

        let full_nested = create_test_nested_struct();

        let test_struct = NestedOptions {
            empty_struct: Some(empty_nested),
            full_struct: Some(full_nested),
        };

        let none_struct = NestedOptions {
            empty_struct: None,
            full_struct: None,
        };

        // Manual implementation
        let manual_bytes = vec![
            test_struct
                .empty_struct
                .as_ref()
                .unwrap()
                .hash::<Poseidon>()
                .unwrap()
                .to_vec(),
            test_struct
                .full_struct
                .as_ref()
                .unwrap()
                .hash::<Poseidon>()
                .unwrap()
                .to_vec(),
        ];

        assert_eq!(test_struct.as_byte_vec(), manual_bytes);
        assert_eq!(none_struct.as_byte_vec(), vec![vec![0], vec![0]]);
        assert_eq!(
            test_struct.hash::<Poseidon>().unwrap(),
            [
                42, 105, 33, 232, 21, 36, 254, 30, 64, 17, 152, 148, 167, 75, 205, 103, 251, 201,
                107, 128, 108, 139, 160, 166, 179, 126, 66, 209, 49, 136, 85, 121
            ]
        );
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
            #[truncate]
            truncated_small: Option<String>,
            #[truncate]
            truncated_large: Option<[u8; 64]>,
            #[nested]
            nested_empty: Option<MyNestedStruct>,
            #[nested]
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
            nested_full: Some(create_test_nested_struct()),
        };

        let partial_struct = MixedOptions {
            basic: Some(42),
            truncated_small: None,
            truncated_large: Some([42u8; 64]),
            nested_empty: None,
            nested_full: Some(create_test_nested_struct()),
        };

        let none_struct = MixedOptions {
            basic: None,
            truncated_small: None,
            truncated_large: None,
            nested_empty: None,
            nested_full: None,
        };

        // Manual implementation
        let manual_bytes = vec![
            {
                let mut bytes = vec![1u8];
                bytes.extend_from_slice(&42u32.to_le_bytes());
                bytes
            },
            light_utils::hash_to_bn254_field_size_be("test".as_bytes())
                .unwrap()
                .0
                .to_vec(),
            light_utils::hash_to_bn254_field_size_be(&[42u8; 64])
                .unwrap()
                .0
                .to_vec(),
            test_struct
                .nested_empty
                .as_ref()
                .unwrap()
                .hash::<Poseidon>()
                .unwrap()
                .to_vec(),
            test_struct
                .nested_full
                .as_ref()
                .unwrap()
                .hash::<Poseidon>()
                .unwrap()
                .to_vec(),
        ];

        assert_eq!(test_struct.as_byte_vec(), manual_bytes);
        assert_eq!(
            none_struct.as_byte_vec(),
            vec![vec![0], vec![0], vec![0], vec![0], vec![0]]
        );

        assert_eq!(
            test_struct.hash::<Poseidon>().unwrap(),
            [
                26, 255, 96, 16, 139, 10, 34, 134, 216, 157, 142, 23, 141, 76, 185, 42, 176, 151,
                14, 66, 125, 232, 121, 94, 123, 40, 249, 134, 234, 121, 136, 33
            ]
        );
        assert_eq!(
            partial_struct.hash::<Poseidon>().unwrap(),
            [
                18, 55, 25, 29, 108, 222, 90, 216, 64, 166, 192, 82, 115, 154, 22, 251, 246, 162,
                81, 155, 224, 199, 145, 50, 170, 137, 184, 95, 186, 59, 92, 45
            ]
        );
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
            #[truncate]
            truncated: Option<String>,
        }

        #[derive(LightHasher)]
        struct OuterStruct {
            #[nested]
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

        // Manual implementation
        let manual_bytes = vec![test_struct.inner.hash::<Poseidon>().unwrap().to_vec(), {
            let mut bytes = vec![1u8];
            bytes.extend_from_slice(&u64::MAX.to_le_bytes());
            bytes
        }];

        assert_eq!(test_struct.as_byte_vec(), manual_bytes);
        assert_eq!(
            test_struct.hash::<Poseidon>().unwrap(),
            [
                7, 3, 81, 207, 22, 159, 8, 6, 135, 4, 218, 21, 188, 99, 254, 111, 144, 177, 54, 33,
                5, 94, 75, 199, 179, 255, 105, 246, 194, 148, 116, 3
            ]
        );
        assert_eq!(
            none_struct.hash::<Poseidon>().unwrap(),
            [
                23, 83, 82, 87, 94, 164, 86, 13, 119, 230, 225, 21, 182, 59, 41, 174, 42, 2, 191,
                189, 157, 234, 195, 122, 103, 142, 82, 137, 231, 49, 77, 106
            ]
        );
    }
}

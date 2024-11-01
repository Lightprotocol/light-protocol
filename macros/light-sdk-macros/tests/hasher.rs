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

    fn create_test_nested_struct() -> MyNestedStruct {
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
                bytes.extend(account.f.unwrap().to_le_bytes());
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

            // Manual implementation for comparison
            // TODO(swen): Check that we can overwrite first byte from 6 to 1 for Some
            // TODO(swen): Check that we can set 32 bytes to 0 for None
            // let some_bytes = vec![with_some.opt.as_ref().unwrap().hash::<Poseidon>().unwrap().to_vec()];
            let some_bytes = {
                let mut bytes = with_some
                    .opt
                    .as_ref()
                    .unwrap()
                    .hash::<Poseidon>()
                    .unwrap()
                    .to_vec();
                bytes[0] = 1u8; // Overwrite first byte with 1 for Some
                vec![bytes]
            };
            let none_bytes = vec![vec![0u8; 32]]; // 32 zero bytes for None

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
}

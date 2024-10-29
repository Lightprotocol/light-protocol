use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use light_hasher::{bytes::AsByteVec, DataHasher, Hasher, Poseidon};
use light_sdk_macros::LightHasher;

#[derive(LightHasher, Clone)]
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

#[derive(LightHasher, Clone)]
pub struct MyNestedStruct {
    pub a: i32,
    pub b: u32,
    #[truncate]
    pub c: String,
}

#[derive(Clone)]
pub struct MyNestedNonHashableStruct {
    pub a: PhantomData<()>,
    pub b: Rc<RefCell<usize>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper functions
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

    // Basic functionality tests
    #[test]
    fn test_byte_representation() {
        // Tests basic byte vector generation and comparison with manual implementation
        let nested_struct = create_test_nested_struct();

        // Manual implementation of AsByteVec for comparison
        let manual_bytes: Vec<Vec<u8>> = vec![
            nested_struct.a.to_le_bytes().to_vec(),
            nested_struct.b.to_le_bytes().to_vec(),
            light_utils::hash_to_bn254_field_size_be(nested_struct.c.as_bytes())
                .unwrap()
                .0
                .to_vec(),
        ];

        // Compare manual implementation with macro-generated one
        assert_eq!(nested_struct.as_byte_vec(), manual_bytes);

        // Test hashing
        let hash_result = nested_struct.hash::<Poseidon>().unwrap();
        assert_eq!(hash_result.len(), 32);
    }

    #[test]
    fn test_boundary_values() {
        // Tests hashing with zero/minimum values
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

    // Attribute tests
    mod attribute_tests {
        use super::*;

        #[test]
        fn test_array_truncation() {
            // Tests #[truncate] with fixed-size arrays
            #[derive(LightHasher)]
            struct TruncatedStruct {
                #[truncate]
                data: [u8; 32],
            }

            // Create arrays with different values
            let single = TruncatedStruct {
                data: [1u8; 32], // All 1s
            };

            let double = TruncatedStruct {
                data: [2u8; 32], // All 2s
            };

            let mixed = TruncatedStruct {
                data: {
                    let mut data = [1u8; 32];
                    data[0] = 2u8; // Change first byte to make it different
                    data
                },
            };

            let single_hash = single.hash::<Poseidon>().unwrap();

            let double_hash = double.hash::<Poseidon>().unwrap();

            let mixed_hash = mixed.hash::<Poseidon>().unwrap();

            // Now the hashes should be different
            assert_ne!(single_hash, double_hash);
            assert_ne!(single_hash, mixed_hash);
            assert_ne!(double_hash, mixed_hash);
        }

        #[test]
        fn test_truncation_longer_array() {
            // Tests #[truncate] with dynamic-sized String
            #[derive(LightHasher)]
            struct LongTruncatedStruct {
                #[truncate]
                data: String,
            }

            // Create data larger than BN254 field size
            let large_data = "a".repeat(64); // 64 'a' characters
            let truncated = LongTruncatedStruct {
                data: large_data.clone(),
            };

            // Create another struct with different data that should produce different hash
            let mut modified_data = large_data.clone();
            modified_data.push('b'); // Add character - should change hash since String is hashed as one unit
            let truncated2 = LongTruncatedStruct {
                data: modified_data,
            };

            let hash1 = truncated.hash::<Poseidon>().unwrap();

            let hash2 = truncated2.hash::<Poseidon>().unwrap();
            // Hashes should be different because String is treated as a single unit
            assert_ne!(
                hash1, hash2,
                "Hashes should be different for different strings"
            );
        }

        #[test]
        fn test_multiple_truncates() {
            // Tests multiple #[truncate] fields in one struct
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
            // Tests combination of #[nested] and #[truncate]
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

            let hash = test_struct.hash::<Poseidon>().unwrap();
            assert_eq!(hash.len(), 32);
        }
    }

    // Nesting and composition tests
    mod nesting_tests {
        use super::*;

        #[test]
        fn test_recursive_nesting() {
            // Tests nested structs with manual hash comparison
            let nested_struct = create_test_nested_struct();

            #[derive(LightHasher)]
            struct TestNestedStruct {
                #[nested]
                one: MyNestedStruct,
                #[nested]
                two: MyNestedStruct,
            }

            let test_nested_struct = TestNestedStruct {
                one: nested_struct.clone(),
                two: nested_struct,
            };

            // Manual implementation for comparison
            let manual_hash = Poseidon::hashv(&[
                &test_nested_struct.one.hash::<Poseidon>().unwrap(),
                &test_nested_struct.two.hash::<Poseidon>().unwrap(),
            ])
            .unwrap();

            assert_eq!(test_nested_struct.hash::<Poseidon>().unwrap(), manual_hash);
        }

        #[test]
        fn test_nested_option() {
            // Tests Option<T> where T is a nested struct
            #[derive(LightHasher)]
            struct NestedOption {
                opt: Option<MyNestedStruct>,
            }

            let with_some = NestedOption {
                opt: Some(create_test_nested_struct()),
            };
            let with_none = NestedOption { opt: None };

            let some_hash = with_some.hash::<Poseidon>().unwrap();
            let none_hash = with_none.hash::<Poseidon>().unwrap();

            assert_ne!(some_hash, none_hash);
            assert_eq!(some_hash.len(), 32);
            assert_eq!(none_hash.len(), 32);
        }

        #[test]
        fn test_nested_field_count() {
            // Tests that nested structs count as single field
            // Inner struct with 12 fields (maximum)
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

            // Should succeed since nested counts as 1 field
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
            // Tests that empty structs fail to hash
            #[derive(LightHasher)]
            struct EmptyStruct {}

            let empty = EmptyStruct {};
            let result = empty.hash::<Poseidon>();

            assert!(result.is_err(), "Empty struct should fail to hash");
        }

        #[test]
        fn test_poseidon_width_limits() {
            // Tests maximum field limit and overflow behavior
            // Test struct with maximum allowed fields (12)
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

            // Should succeed
            assert!(max_fields.hash::<Poseidon>().is_ok());

            // Test struct exceeding maximum fields (13)
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

    // Test with reference values
    #[test]
    fn test_option_hashing() {
        // Tests Option<T> hashing
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
                21, 178, 66, 188, 152, 166, 98, 224, 150, 92, 94, 231, 230, 26, 88, 1, 86, 22, 89,
                72, 69, 230, 168, 55, 224, 148, 49, 76, 112, 6, 85, 248
            ]
        );
    }
}

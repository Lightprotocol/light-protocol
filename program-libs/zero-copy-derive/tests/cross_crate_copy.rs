#![cfg(feature = "mut")]
//! Test cross-crate Copy identification functionality
//!
//! This test validates that the zero-copy derive macro correctly identifies
//! which types implement Copy, both for built-in types and user-defined types.

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyEq, ZeroCopyMut};

// Test struct with primitive Copy types that should be in meta fields
#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct PrimitiveCopyStruct {
    pub a: u8,
    pub b: u16,
    pub c: u32,
    pub d: u64,
    pub e: bool,
    pub f: Vec<u8>, // Split point - this and following fields go to struct_fields
    pub g: u32,     // Should be in struct_fields due to field ordering rules
}

// Test struct with primitive Copy types that should be in meta fields
#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize, ZeroCopy, ZeroCopyEq, ZeroCopyMut)]
pub struct PrimitiveCopyStruct2 {
    pub f: Vec<u8>, // Split point - this and following fields go to struct_fields
    pub a: u8,
    pub b: u16,
    pub c: u32,
    pub d: u64,
    pub e: bool,
    pub g: u32,
}

// Test struct with arrays that use u8 (which supports Unaligned)
#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct ArrayCopyStruct {
    pub fixed_u8: [u8; 4],
    pub another_u8: [u8; 8],
    pub data: Vec<u8>,      // Split point
    pub more_data: [u8; 3], // Should be in struct_fields due to field ordering
}

// Test struct with Vec of primitive Copy types
#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct VecPrimitiveStruct {
    pub header: u32,
    pub data: Vec<u8>,     // Vec<u8> - special case
    pub numbers: Vec<u32>, // Vec of Copy type
    pub footer: u64,
}

#[cfg(test)]
mod tests {
    use light_zero_copy::borsh::Deserialize;

    use super::*;

    #[test]
    fn test_primitive_copy_field_splitting() {
        // This test validates that primitive Copy types are correctly
        // identified and placed in meta_fields until we hit a Vec

        let data = PrimitiveCopyStruct {
            a: 1,
            b: 2,
            c: 3,
            d: 4,
            e: true,
            f: vec![5, 6, 7],
            g: 8,
        };

        let serialized = borsh::to_vec(&data).unwrap();
        let (deserialized, _) = PrimitiveCopyStruct::zero_copy_at(&serialized).unwrap();

        // Verify we can access meta fields (should be zero-copy references)
        assert_eq!(deserialized.a, 1);
        assert_eq!(deserialized.b.get(), 2); // U16 type, use .get()
        assert_eq!(deserialized.c.get(), 3); // U32 type, use .get()
        assert_eq!(deserialized.d.get(), 4); // U64 type, use .get()
        assert_eq!(deserialized.e(), true); // bool accessor method

        // Verify we can access struct fields
        assert_eq!(deserialized.f, &[5, 6, 7]);
        assert_eq!(deserialized.g.get(), 8); // U32 type in struct fields
    }

    #[test]
    fn test_array_copy_field_splitting() {
        // Arrays should be treated as Copy types
        let data = ArrayCopyStruct {
            fixed_u8: [1, 2, 3, 4],
            another_u8: [10, 20, 30, 40, 50, 60, 70, 80],
            data: vec![5, 6],
            more_data: [30, 40, 50],
        };

        let serialized = borsh::to_vec(&data).unwrap();
        let (deserialized, _) = ArrayCopyStruct::zero_copy_at(&serialized).unwrap();

        // Arrays should be accessible (in meta_fields before Vec split)
        assert_eq!(deserialized.fixed_u8.as_ref(), &[1, 2, 3, 4]);
        assert_eq!(
            deserialized.another_u8.as_ref(),
            &[10, 20, 30, 40, 50, 60, 70, 80]
        );

        // After Vec split
        assert_eq!(deserialized.data, &[5, 6]);
        assert_eq!(deserialized.more_data.as_ref(), &[30, 40, 50]);
    }

    #[test]
    fn test_vec_primitive_types() {
        // Test Vec with various primitive Copy element types
        let data = VecPrimitiveStruct {
            header: 1,
            data: vec![10, 20, 30],
            numbers: vec![100, 200, 300],
            footer: 999,
        };

        let serialized = borsh::to_vec(&data).unwrap();
        let (deserialized, _) = VecPrimitiveStruct::zero_copy_at(&serialized).unwrap();

        assert_eq!(deserialized.header.get(), 1);

        // Vec<u8> is special case - stored as slice
        assert_eq!(deserialized.data, &[10, 20, 30]);

        // Vec<u32> should use ZeroCopySliceBorsh
        assert_eq!(deserialized.numbers.len(), 3);
        assert_eq!(deserialized.numbers[0].get(), 100);
        assert_eq!(deserialized.numbers[1].get(), 200);
        assert_eq!(deserialized.numbers[2].get(), 300);

        assert_eq!(deserialized.footer.get(), 999);
    }

    #[test]
    fn test_all_derives_with_vec_first() {
        // This test validates PrimitiveCopyStruct2 which has Vec<u8> as the first field
        // This means NO meta fields (all fields go to struct_fields due to field ordering)
        // Also tests all derive macros: ZeroCopy, ZeroCopyEq, ZeroCopyMut

        use light_zero_copy::{borsh_mut::DeserializeMut, init_mut::ZeroCopyNew};

        let data = PrimitiveCopyStruct2 {
            f: vec![1, 2, 3], // Vec first - causes all fields to be in struct_fields
            a: 10,
            b: 20,
            c: 30,
            d: 40,
            e: true,
            g: 50,
        };

        // Test ZeroCopy (immutable)
        let serialized = borsh::to_vec(&data).unwrap();
        let (deserialized, _) = PrimitiveCopyStruct2::zero_copy_at(&serialized).unwrap();

        // Since Vec is first, ALL fields should be in struct_fields (no meta fields)
        assert_eq!(deserialized.f, &[1, 2, 3]);
        assert_eq!(deserialized.a, 10); // u8 direct access
        assert_eq!(deserialized.b.get(), 20); // U16 via .get()
        assert_eq!(deserialized.c.get(), 30); // U32 via .get()
        assert_eq!(deserialized.d.get(), 40); // U64 via .get()
        assert_eq!(deserialized.e(), true); // bool accessor method
        assert_eq!(deserialized.g.get(), 50); // U32 via .get()

        // Test ZeroCopyEq (PartialEq implementation)
        let original = PrimitiveCopyStruct2 {
            f: vec![1, 2, 3],
            a: 10,
            b: 20,
            c: 30,
            d: 40,
            e: true,
            g: 50,
        };

        // Should be equal to original
        assert_eq!(deserialized, original);

        // Test inequality
        let different = PrimitiveCopyStruct2 {
            f: vec![1, 2, 3],
            a: 11,
            b: 20,
            c: 30,
            d: 40,
            e: true,
            g: 50, // Different 'a'
        };
        assert_ne!(deserialized, different);

        // Test ZeroCopyMut (mutable zero-copy)
        #[cfg(feature = "mut")]
        {
            let mut serialized_mut = borsh::to_vec(&data).unwrap();
            let (deserialized_mut, _) =
                PrimitiveCopyStruct2::zero_copy_at_mut(&mut serialized_mut).unwrap();

            // Test mutable access
            assert_eq!(deserialized_mut.f, &[1, 2, 3]);
            assert_eq!(*deserialized_mut.a, 10); // Mutable u8 field
            assert_eq!(deserialized_mut.b.get(), 20);
            let (deserialized_mut, _) =
                PrimitiveCopyStruct2::zero_copy_at(&mut serialized_mut).unwrap();

            // Test From implementation (ZeroCopyEq generates this for immutable version)
            let converted: PrimitiveCopyStruct2 = deserialized_mut.into();
            assert_eq!(converted.a, 10);
            assert_eq!(converted.b, 20);
            assert_eq!(converted.c, 30);
            assert_eq!(converted.d, 40);
            assert_eq!(converted.e, true);
            assert_eq!(converted.f, vec![1, 2, 3]);
            assert_eq!(converted.g, 50);
        }

        // Test ZeroCopyNew (configuration-based initialization)
        let config = super::PrimitiveCopyStruct2Config {
            f: 3, // Vec<u8> length
                  // Other fields don't need config (they're primitives)
        };

        // Calculate required buffer size
        let buffer_size = PrimitiveCopyStruct2::byte_len(&config);
        let mut buffer = vec![0u8; buffer_size];

        // Initialize the zero-copy struct
        let (mut initialized, _) =
            PrimitiveCopyStruct2::new_zero_copy(&mut buffer, config).unwrap();

        // Verify we can access the initialized fields
        assert_eq!(initialized.f.len(), 3); // Vec should have correct length

        // Set some values in the Vec
        initialized.f[0] = 100;
        initialized.f[1] = 101;
        initialized.f[2] = 102;
        *initialized.a = 200;

        // Verify the values were set correctly
        assert_eq!(initialized.f, &[100, 101, 102]);
        assert_eq!(*initialized.a, 200);

        println!("All derive macros (ZeroCopy, ZeroCopyEq, ZeroCopyMut) work correctly with Vec-first struct!");
    }

    #[test]
    fn test_copy_identification_compilation() {
        // The primary test is that our macro successfully processes all struct definitions
        // above without panicking or generating invalid code. The fact that compilation
        // succeeds demonstrates that our Copy identification logic works correctly.

        // Test basic functionality to ensure the generated code is sound
        let primitive_data = PrimitiveCopyStruct {
            a: 1,
            b: 2,
            c: 3,
            d: 4,
            e: true,
            f: vec![1, 2],
            g: 5,
        };

        let array_data = ArrayCopyStruct {
            fixed_u8: [1, 2, 3, 4],
            another_u8: [5, 6, 7, 8, 9, 10, 11, 12],
            data: vec![13, 14],
            more_data: [15, 16, 17],
        };

        let vec_data = VecPrimitiveStruct {
            header: 42,
            data: vec![1, 2, 3],
            numbers: vec![10, 20],
            footer: 99,
        };

        // Serialize and deserialize to verify the generated code works
        let serialized = borsh::to_vec(&primitive_data).unwrap();
        let (_, _) = PrimitiveCopyStruct::zero_copy_at(&serialized).unwrap();

        let serialized = borsh::to_vec(&array_data).unwrap();
        let (_, _) = ArrayCopyStruct::zero_copy_at(&serialized).unwrap();

        let serialized = borsh::to_vec(&vec_data).unwrap();
        let (_, _) = VecPrimitiveStruct::zero_copy_at(&serialized).unwrap();

        println!("Cross-crate Copy identification test passed - all structs compiled and work correctly!");
    }
}

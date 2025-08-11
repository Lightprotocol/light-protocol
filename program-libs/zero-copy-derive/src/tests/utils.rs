use syn::parse_quote;

use crate::shared::utils::*;

// Helper function to check if a struct implements Copy
fn check_struct_implements_copy(input: syn::DeriveInput) -> bool {
    struct_implements_copy(&input)
}

#[test]
fn test_struct_implements_copy() {
    // Ensure the cache is cleared for testing
    if let Ok(mut cache) = COPY_IMPL_CACHE.lock() {
        cache.clear();
    }
    // Test case 1: Empty struct with #[derive(Copy)]
    let input: syn::DeriveInput = parse_quote! {
        #[derive(Copy, Clone)]
        struct EmptyStruct {}
    };
    assert!(
        check_struct_implements_copy(input),
        "EmptyStruct should implement Copy with #[derive(Copy)]"
    );

    // Test case 2: Simple struct with #[derive(Copy)]
    let input: syn::DeriveInput = parse_quote! {
        #[derive(Copy, Clone)]
        struct SimpleStruct {
            a: u8,
            b: u16,
        }
    };
    assert!(
        check_struct_implements_copy(input),
        "SimpleStruct should implement Copy with #[derive(Copy)]"
    );

    // Test case 3: Struct with #[derive(Clone)] but not Copy
    let input: syn::DeriveInput = parse_quote! {
        #[derive(Clone)]
        struct StructWithoutCopy {
            a: u8,
            b: u16,
        }
    };
    assert!(
        !check_struct_implements_copy(input),
        "StructWithoutCopy should not implement Copy without #[derive(Copy)]"
    );

    // Test case 4: Struct with a non-Copy field but with derive(Copy)
    // Note: In real Rust code, this would not compile, but for our test we only check attributes
    let input: syn::DeriveInput = parse_quote! {
        #[derive(Copy, Clone)]
        struct StructWithVec {
            a: u8,
            b: Vec<u8>,
        }
    };
    assert!(
        check_struct_implements_copy(input),
        "StructWithVec has #[derive(Copy)] so our function returns true"
    );

    // Test case 5: Struct with all Copy fields but without #[derive(Copy)]
    let input: syn::DeriveInput = parse_quote! {
        struct StructWithCopyFields {
            a: u8,
            b: u16,
            c: i32,
            d: bool,
        }
    };
    assert!(
        !check_struct_implements_copy(input),
        "StructWithCopyFields should not implement Copy without #[derive(Copy)]"
    );

    // Test case 6: Unit struct without #[derive(Copy)]
    let input: syn::DeriveInput = parse_quote! {
        struct UnitStructWithoutCopy;
    };
    assert!(
        !check_struct_implements_copy(input),
        "UnitStructWithoutCopy should not implement Copy without #[derive(Copy)]"
    );

    // Test case 7: Unit struct with #[derive(Copy)]
    let input: syn::DeriveInput = parse_quote! {
        #[derive(Copy, Clone)]
        struct UnitStructWithCopy;
    };
    assert!(
        check_struct_implements_copy(input),
        "UnitStructWithCopy should implement Copy with #[derive(Copy)]"
    );

    // Test case 8: Tuple struct with #[derive(Copy)]
    let input: syn::DeriveInput = parse_quote! {
        #[derive(Copy, Clone)]
        struct TupleStruct(u32, bool, char);
    };
    assert!(
        check_struct_implements_copy(input),
        "TupleStruct should implement Copy with #[derive(Copy)]"
    );

    // Test case 9: Multiple derives including Copy
    let input: syn::DeriveInput = parse_quote! {
        #[derive(Debug, PartialEq, Copy, Clone)]
        struct MultipleDerivesStruct {
            a: u8,
        }
    };
    assert!(
        check_struct_implements_copy(input),
        "MultipleDerivesStruct should implement Copy with #[derive(Copy)]"
    );
}

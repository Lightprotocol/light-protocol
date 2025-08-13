use quote::format_ident;
use rand::{prelude::SliceRandom, rngs::StdRng, thread_rng, Rng, SeedableRng};
use syn::parse_quote;

use crate::shared::z_struct::*;

/// Generate a safe field name for testing
fn random_ident(rng: &mut StdRng) -> String {
    // Use predetermined safe field names
    const FIELD_NAMES: &[&str] = &[
        "field1", "field2", "field3", "field4", "field5", "value", "data", "count", "size", "flag",
        "name", "id", "code", "index", "key", "amount", "balance", "total", "result", "status",
    ];

    FIELD_NAMES.choose(rng).unwrap().to_string()
}

/// Generate a random Rust type
fn random_type(rng: &mut StdRng, _depth: usize) -> syn::Type {
    // Define our available types
    let types = [0, 1, 2, 3, 4, 5, 6, 7];

    // Randomly select a type index
    let selected = *types.choose(rng).unwrap();

    // Return the corresponding type
    match selected {
        0 => parse_quote!(u8),
        1 => parse_quote!(u16),
        2 => parse_quote!(u32),
        3 => parse_quote!(u64),
        4 => parse_quote!(bool),
        5 => parse_quote!(Vec<u8>),
        6 => parse_quote!(Vec<u16>),
        7 => parse_quote!(Vec<u32>),
        _ => unreachable!(),
    }
}

/// Generate a random field
fn random_field(rng: &mut StdRng) -> syn::Field {
    let name = random_ident(rng);
    let ty = random_type(rng, 0);

    // Use a safer approach to create the field
    let name_ident = format_ident!("{}", name);
    parse_quote!(pub #name_ident: #ty)
}

/// Generate a list of random fields
fn random_fields(rng: &mut StdRng, count: usize) -> Vec<syn::Field> {
    (0..count).map(|_| random_field(rng)).collect()
}

#[test]
fn test_fuzz_generate_z_struct() {
    // Set up RNG with a seed for reproducibility
    let seed = thread_rng().gen();
    println!("seed {}", seed);
    let mut rng = StdRng::seed_from_u64(seed);

    // Now that the test is working, run with 10,000 iterations
    let num_iters = 10000;

    for i in 0..num_iters {
        // Generate a random struct name
        let struct_name = format_ident!("{}", random_ident(&mut rng));
        let z_struct_name = format_ident!("Z{}", struct_name);
        let z_struct_meta_name = format_ident!("Z{}Meta", struct_name);

        // Generate random number of fields (1-10)
        let field_count = rng.gen_range(1..11);
        let fields = random_fields(&mut rng, field_count);

        // Create a named fields collection that lives longer than the process_fields call
        let syn_fields = syn::punctuated::Punctuated::from_iter(fields.iter().cloned());
        let fields_named = syn::FieldsNamed {
            brace_token: syn::token::Brace::default(),
            named: syn_fields,
        };

        // Split into meta fields and struct fields
        let (meta_fields, struct_fields) = crate::shared::utils::process_fields(&fields_named);

        // Call the function we're testing
        let result = generate_z_struct::<false>(
            &z_struct_name,
            &z_struct_meta_name,
            &struct_fields,
            &meta_fields,
            false,
        );

        // Get the generated code as a string for validation
        let result_str = result.unwrap().to_string();

        // Validate the generated code

        // Verify the result contains expected struct elements
        // Basic validation - must be non-empty
        assert!(
            !result_str.is_empty(),
            "Failed to generate TokenStream for iteration {}",
            i
        );

        // Validate that the generated code contains the expected struct definition
        let struct_pattern = format!("struct {} < 'a >", z_struct_name);
        assert!(
            result_str.contains(&struct_pattern),
            "Generated code missing struct definition for iteration {}. Expected: {}",
            i,
            struct_pattern
        );

        if meta_fields.is_empty() {
            // Validate the meta field is present
            assert!(
                !result_str.contains("meta :"),
                "Generated code had meta field for iteration {}",
                i
            );
            // Validate Deref implementation
            assert!(
                !result_str.contains("impl < 'a > :: core :: ops :: Deref"),
                "Generated code has unexpected Deref implementation for iteration {}",
                i
            );
        } else {
            // Validate the meta field is present
            assert!(
                result_str.contains("meta :"),
                "Generated code missing meta field for iteration {}",
                i
            );
            // Validate Deref implementation
            assert!(
                result_str.contains("impl < 'a > :: core :: ops :: Deref"),
                "Generated code missing Deref implementation for iteration {}",
                i
            );
            // Validate Target type
            assert!(
                result_str.contains("type Target"),
                "Generated code missing Target type for iteration {}",
                i
            );
            // Check that the deref method is implemented
            assert!(
                result_str.contains("fn deref (& self)"),
                "Generated code missing deref method for iteration {}",
                i
            );

            // Check for light_zero_copy::Ref reference
            assert!(
                result_str.contains(":: light_zero_copy :: Ref"),
                "Generated code missing light_zero_copy::Ref for iteration {}",
                i
            );
        }

        // Make sure derive attributes are present
        assert!(
            result_str.contains("# [derive (Debug , PartialEq , Clone)]"),
            "Generated code missing derive attributes for iteration {}",
            i
        );
    }
}

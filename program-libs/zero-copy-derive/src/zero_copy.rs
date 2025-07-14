use proc_macro::TokenStream as ProcTokenStream;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_quote, DeriveInput, Field, Ident};

use crate::shared::{
    meta_struct, utils,
    z_struct::{analyze_struct_fields, generate_z_struct, FieldType},
};

/// Helper function to generate deserialize call pattern for a given type
fn generate_deserialize_call<const MUT: bool>(
    field_name: &syn::Ident,
    field_type: &syn::Type,
) -> TokenStream {
    let field_type = utils::convert_to_zerocopy_type(field_type);
    let trait_path = if MUT {
        quote!( as light_zero_copy::borsh_mut::DeserializeMut>::zero_copy_at_mut)
    } else {
        quote!( as light_zero_copy::borsh::Deserialize>::zero_copy_at)
    };

    quote! {
        let (#field_name, bytes) = <#field_type #trait_path(bytes)?;
    }
}

/// Generates field deserialization code for the Deserialize implementation
/// The `MUT` parameter controls whether to generate code for mutable or immutable references
pub fn generate_deserialize_fields<'a, const MUT: bool>(
    struct_fields: &'a [&'a Field],
) -> syn::Result<impl Iterator<Item = TokenStream> + 'a> {
    let field_types = analyze_struct_fields(struct_fields)?;

    let iterator = field_types.into_iter().map(move |field_type| {
        let mutability_tokens = if MUT {
            quote!(&'a mut [u8])
        } else {
            quote!(&'a [u8])
        };
        match field_type {
            FieldType::VecU8(field_name) => {
                if MUT {
                    quote! {
                        let (#field_name, bytes) = light_zero_copy::borsh_mut::borsh_vec_u8_as_slice_mut(bytes)?;
                    }
                } else {
                    quote! {
                        let (#field_name, bytes) = light_zero_copy::borsh::borsh_vec_u8_as_slice(bytes)?;
                    }
                }
            },
            FieldType::VecCopy(field_name, inner_type) => {
                let inner_type = utils::convert_to_zerocopy_type(inner_type);

                let trait_path = if MUT {
                    quote!(light_zero_copy::slice_mut::ZeroCopySliceMutBorsh::<'a, <#inner_type as light_zero_copy::borsh_mut::ZeroCopyStructInnerMut>::ZeroCopyInnerMut>)
                } else {
                    quote!(light_zero_copy::slice::ZeroCopySliceBorsh::<'a, <#inner_type as light_zero_copy::borsh::ZeroCopyStructInner>::ZeroCopyInner>)
                };
                quote! {
                    let (#field_name, bytes) = #trait_path::from_bytes_at(bytes)?;
                }
            },
            FieldType::VecDynamicZeroCopy(field_name, field_type) => {
                generate_deserialize_call::<MUT>(field_name, field_type)
            },
            FieldType::Array(field_name, field_type) => {
                let field_type = utils::convert_to_zerocopy_type(field_type);
                quote! {
                    let (#field_name, bytes) = light_zero_copy::Ref::<#mutability_tokens, #field_type>::from_prefix(bytes)?;
                }
            },
            FieldType::Option(field_name, field_type) => {
                generate_deserialize_call::<MUT>(field_name, field_type)
            },
            FieldType::Pubkey(field_name) => {
                generate_deserialize_call::<MUT>(field_name, &parse_quote!(Pubkey))
            },
            FieldType::Primitive(field_name, field_type) => {
                if MUT {
                    quote! {
                        let (#field_name, bytes) = <#field_type as light_zero_copy::borsh_mut::DeserializeMut>::zero_copy_at_mut(bytes)?;
                    }
                } else {
                    quote! {
                        let (#field_name, bytes) = <#field_type as light_zero_copy::borsh::Deserialize>::zero_copy_at(bytes)?;
                    }
                }
            },
            FieldType::Copy(field_name, field_type) => {
                let field_ty_zerocopy = utils::convert_to_zerocopy_type(field_type);
                quote! {
                    let (#field_name, bytes) = light_zero_copy::Ref::<#mutability_tokens, #field_ty_zerocopy>::from_prefix(bytes)?;
                }
            },
            FieldType::DynamicZeroCopy(field_name, field_type) => {
                generate_deserialize_call::<MUT>(field_name, field_type)
            },
            FieldType::OptionU64(field_name) => {
                let field_ty_zerocopy = utils::convert_to_zerocopy_type(&parse_quote!(u64));
                generate_deserialize_call::<MUT>(field_name, &parse_quote!(Option<#field_ty_zerocopy>))
            },
            FieldType::OptionU32(field_name) => {
                let field_ty_zerocopy = utils::convert_to_zerocopy_type(&parse_quote!(u32));
                generate_deserialize_call::<MUT>(field_name, &parse_quote!(Option<#field_ty_zerocopy>))
            },
            FieldType::OptionU16(field_name) => {
                let field_ty_zerocopy = utils::convert_to_zerocopy_type(&parse_quote!(u16));
                generate_deserialize_call::<MUT>(field_name, &parse_quote!(Option<#field_ty_zerocopy>))
            }
        }
    });
    Ok(iterator)
}

/// Generates field initialization code for the Deserialize implementation
pub fn generate_init_fields<'a>(
    struct_fields: &'a [&'a Field],
) -> impl Iterator<Item = TokenStream> + 'a {
    struct_fields.iter().map(|field| {
        let field_name = &field.ident;
        quote! { #field_name }
    })
}

/// Generates the Deserialize implementation as a TokenStream
/// The `MUT` parameter controls whether to generate code for mutable or immutable references
pub fn generate_deserialize_impl<const MUT: bool>(
    name: &Ident,
    z_struct_name: &Ident,
    z_struct_meta_name: &Ident,
    struct_fields: &[&Field],
    meta_is_empty: bool,
    byte_len_impl: TokenStream,
) -> syn::Result<TokenStream> {
    let z_struct_name = if MUT {
        format_ident!("{}Mut", z_struct_name)
    } else {
        z_struct_name.clone()
    };
    let z_struct_meta_name = if MUT {
        format_ident!("{}Mut", z_struct_meta_name)
    } else {
        z_struct_meta_name.clone()
    };

    // Define trait and types based on mutability
    let (trait_name, mutability, method_name) = if MUT {
        (
            quote!(light_zero_copy::borsh_mut::DeserializeMut),
            quote!(mut),
            quote!(zero_copy_at_mut),
        )
    } else {
        (
            quote!(light_zero_copy::borsh::Deserialize),
            quote!(),
            quote!(zero_copy_at),
        )
    };
    let (meta_des, meta) = if meta_is_empty {
        (quote!(), quote!())
    } else {
        (
            quote! {
                let (__meta, bytes) = light_zero_copy::Ref::< &'a #mutability [u8], #z_struct_meta_name>::from_prefix(bytes)?;
            },
            quote!(__meta,),
        )
    };
    let deserialize_fields = generate_deserialize_fields::<MUT>(struct_fields)?;
    let init_fields = generate_init_fields(struct_fields);

    let result = quote! {
        impl<'a> #trait_name<'a> for #name {
            type Output = #z_struct_name<'a>;

            fn #method_name(bytes: &'a #mutability [u8]) -> Result<(Self::Output, &'a #mutability [u8]), light_zero_copy::errors::ZeroCopyError> {
                #meta_des
                #(#deserialize_fields)*
                Ok((
                    #z_struct_name {
                        #meta
                        #(#init_fields,)*
                    },
                    bytes
                ))
            }

            #byte_len_impl
        }
    };
    Ok(result)
}

// #[cfg(test)]
// mod tests {
//     use quote::format_ident;
//     use rand::{prelude::SliceRandom, rngs::StdRng, thread_rng, Rng, SeedableRng};
//     use syn::parse_quote;

//     use super::*;

//     /// Generate a safe field name for testing
//     fn random_ident(rng: &mut StdRng) -> String {
//         // Use predetermined safe field names
//         const FIELD_NAMES: &[&str] = &[
//             "field1", "field2", "field3", "field4", "field5", "value", "data", "count", "size",
//             "flag", "name", "id", "code", "index", "key", "amount", "balance", "total", "result",
//             "status",
//         ];

//         FIELD_NAMES.choose(rng).unwrap().to_string()
//     }

//     /// Generate a random Rust type
//     fn random_type(rng: &mut StdRng, _depth: usize) -> syn::Type {
//         // Define our available types
//         let types = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

//         // Randomly select a type index
//         let selected = *types.choose(rng).unwrap();

//         // Return the corresponding type
//         match selected {
//             0 => parse_quote!(u8),
//             1 => parse_quote!(u16),
//             2 => parse_quote!(u32),
//             3 => parse_quote!(u64),
//             4 => parse_quote!(bool),
//             5 => parse_quote!(Vec<u8>),
//             6 => parse_quote!(Vec<u16>),
//             7 => parse_quote!(Vec<u32>),
//             8 => parse_quote!([u32; 12]),
//             9 => parse_quote!([Vec<u16>; 12]),
//             10 => parse_quote!([Vec<u8>; 20]),
//             _ => unreachable!(),
//         }
//     }

//     /// Generate a random field
//     fn random_field(rng: &mut StdRng) -> Field {
//         let name = random_ident(rng);
//         let ty = random_type(rng, 0);

//         // Use a safer approach to create the field
//         let name_ident = format_ident!("{}", name);
//         parse_quote!(pub #name_ident: #ty)
//     }

//     /// Generate a list of random fields
//     fn random_fields(rng: &mut StdRng, count: usize) -> Vec<Field> {
//         (0..count).map(|_| random_field(rng)).collect()
//     }

//     // Test for Vec<u8> field deserialization
//     #[test]
//     fn test_deserialize_vec_u8() {
//         let field: Field = parse_quote!(pub data: Vec<u8>);
//         let struct_fields = vec![&field];

//         let result = generate_deserialize_fields::<false>(&struct_fields).collect::<Vec<_>>();
//         let result_str = result[0].to_string();
//         let expected =
//             "let (data , bytes) = light_zero_copy :: borsh :: borsh_vec_u8_as_slice (bytes) ?";

//         assert!(result_str.contains(expected));
//     }

//     // Test for Vec<T> with Copy inner type deserialization
//     #[test]
//     fn test_deserialize_vec_copy_type() {
//         let field: Field = parse_quote!(pub values: Vec<u32>);
//         let struct_fields = vec![&field];

//         let result = generate_deserialize_fields::<false>(&struct_fields).collect::<Vec<_>>();
//         let result_str = result[0].to_string();
//         let expected = "let (values , bytes) = light_zero_copy :: slice :: ZeroCopySliceBorsh :: < 'a , < u32 as light_zero_copy :: borsh :: ZeroCopyStructInner > :: ZeroCopyInner > :: from_bytes_at (bytes) ?";

//         assert!(result_str.contains(expected));
//     }

//     // Test for Vec<T> with non-Copy inner type deserialization
//     #[test]
//     fn test_deserialize_vec_non_copy_type() {
//         // This is a synthetic test as we're treating String as a non-Copy type
//         let field: Field = parse_quote!(pub names: Vec<String>);
//         let struct_fields = vec![&field];

//         let result = generate_deserialize_fields::<false>(&struct_fields).collect::<Vec<_>>();
//         let result_str = result[0].to_string();
//         let expected = "let (names , bytes) = < Vec < String > as light_zero_copy :: borsh :: Deserialize > :: zero_copy_at (bytes) ?";

//         assert!(result_str.contains(expected));
//     }

//     // Test for Option<T> type deserialization
//     #[test]
//     fn test_deserialize_option_type() {
//         let field: Field = parse_quote!(pub maybe_value: Option<u32>);
//         let struct_fields = vec![&field];

//         let result = generate_deserialize_fields::<false>(&struct_fields).collect::<Vec<_>>();
//         let result_str = result[0].to_string();
//         let expected = "let (maybe_value , bytes) = < Option < u32 > as light_zero_copy :: borsh :: Deserialize > :: zero_copy_at (bytes) ?";

//         assert!(result_str.contains(expected));
//     }

//     // Test for non-Copy type deserialization
//     #[test]
//     fn test_deserialize_non_copy_type() {
//         // Using String as a non-Copy type example
//         let field: Field = parse_quote!(pub name: String);
//         let struct_fields = vec![&field];

//         let result = generate_deserialize_fields::<false>(&struct_fields).collect::<Vec<_>>();
//         let result_str = result[0].to_string();
//         let expected = "let (name , bytes) = < String as light_zero_copy :: borsh :: Deserialize > :: zero_copy_at (bytes) ?";

//         assert!(result_str.contains(expected));
//     }

//     // Test for Copy type deserialization (primitive types)
//     #[test]
//     fn test_deserialize_copy_type() {
//         let field: Field = parse_quote!(pub count: u32);
//         let struct_fields = vec![&field];

//         let result = generate_deserialize_fields::<false>(&struct_fields).collect::<Vec<_>>();
//         let result_str = result[0].to_string();
//         let expected = "let (count , bytes) = light_zero_copy :: Ref :: < & 'a [u8] , light_zero_copy :: little_endian :: U32 > :: from_prefix (bytes) ?";
//         println!("{}", result_str);
//         assert!(result_str.contains(expected));
//     }

//     // Test for boolean type deserialization
//     #[test]
//     fn test_deserialize_bool_type() {
//         let field: Field = parse_quote!(pub flag: bool);
//         let struct_fields = vec![&field];

//         let result = generate_deserialize_fields::<false>(&struct_fields).collect::<Vec<_>>();
//         let result_str = result[0].to_string();
//         let expected =
//             "let (flag , bytes) = < u8 as light_zero_copy :: borsh :: Deserialize > :: zero_copy_at (bytes) ?";
//         println!("{}", result_str);
//         assert!(result_str.contains(expected));
//     }

//     // Test for field initialization code generation
//     #[test]
//     fn test_init_fields() {
//         let field1: Field = parse_quote!(pub id: u32);
//         let field2: Field = parse_quote!(pub name: String);
//         let struct_fields = vec![&field1, &field2];

//         let result = generate_init_fields(&struct_fields).collect::<Vec<_>>();
//         let result_str = format!("{} {}", result[0], result[1]);
//         assert!(result_str.contains("id"));
//         assert!(result_str.contains("name"));
//     }

//     // Test for complete deserialize implementation generation
//     #[test]
//     fn test_generate_deserialize_impl() {
//         let struct_name = format_ident!("TestStruct");
//         let z_struct_name = format_ident!("ZTestStruct");
//         let z_struct_meta_name = format_ident!("ZTestStructMeta");

//         let field1: Field = parse_quote!(pub id: u32);
//         let field2: Field = parse_quote!(pub values: Vec<u16>);
//         let struct_fields = vec![&field1, &field2];

//         let result = generate_deserialize_impl::<false>(
//             &struct_name,
//             &z_struct_name,
//             &z_struct_meta_name,
//             &struct_fields,
//             false,
//         )
//         .to_string();

//         // Check impl header
//         assert!(result
//             .contains("impl < 'a > light_zero_copy :: borsh :: Deserialize < 'a > for TestStruct"));

//         // Check Output type
//         assert!(result.contains("type Output = ZTestStruct < 'a >"));

//         // Check method signature
//         assert!(result.contains("fn zero_copy_at (bytes : & 'a [u8]) -> Result"));

//         // Check meta field extraction
//         assert!(result.contains("let (__meta , bytes) = light_zero_copy :: Ref :: < & 'a [u8] , ZTestStructMeta > :: from_prefix (bytes) ?"));

//         // Check field deserialization
//         assert!(result.contains("let (id , bytes) = light_zero_copy :: Ref :: < & 'a [u8] , light_zero_copy :: little_endian :: U32 > :: from_prefix (bytes) ?"));
//         assert!(result.contains("let (values , bytes) = light_zero_copy :: slice :: ZeroCopySliceBorsh :: < 'a , < u16 as light_zero_copy :: borsh :: ZeroCopyStructInner > :: ZeroCopyInner > :: from_bytes_at (bytes) ?"));

//         // Check result structure
//         assert!(result.contains("Ok ((ZTestStruct { __meta , id , values ,"));
//     }

//     // Test for complete deserialize implementation generation
//     #[test]
//     fn test_generate_deserialize_impl_no_meta() {
//         let struct_name = format_ident!("TestStruct");
//         let z_struct_name = format_ident!("ZTestStruct");
//         let z_struct_meta_name = format_ident!("ZTestStructMeta");

//         let field1: Field = parse_quote!(pub id: u32);
//         let field2: Field = parse_quote!(pub values: Vec<u16>);
//         let struct_fields = vec![&field1, &field2];

//         let result = generate_deserialize_impl::<false>(
//             &struct_name,
//             &z_struct_name,
//             &z_struct_meta_name,
//             &struct_fields,
//             true,
//         )
//         .to_string();

//         // Check impl header
//         assert!(result
//             .contains("impl < 'a > light_zero_copy :: borsh :: Deserialize < 'a > for TestStruct"));

//         // Check Output type
//         assert!(result.contains("type Output = ZTestStruct < 'a >"));

//         // Check method signature
//         assert!(result.contains("fn zero_copy_at (bytes : & 'a [u8]) -> Result"));

//         // Check meta field extraction
//         assert!(!result.contains("let (meta , bytes) = light_zero_copy :: Ref :: < & 'a [u8] , ZTestStructMeta > :: from_prefix (bytes) ?"));

//         // Check field deserialization
//         assert!(result.contains("let (id , bytes) = light_zero_copy :: Ref :: < & 'a [u8] , light_zero_copy :: little_endian :: U32 > :: from_prefix (bytes) ?"));
//         assert!(result.contains("let (values , bytes) = light_zero_copy :: slice :: ZeroCopySliceBorsh :: < 'a , < u16 as light_zero_copy :: borsh :: ZeroCopyStructInner > :: ZeroCopyInner > :: from_bytes_at (bytes) ?"));

//         // Check result structure
//         assert!(result.contains("Ok ((ZTestStruct { id , values ,"));
//     }

//     #[test]
//     fn test_fuzz_generate_deserialize_impl() {
//         // Set up RNG with a seed for reproducibility
//         let seed = thread_rng().gen();
//         println!("seed {}", seed);
//         let mut rng = StdRng::seed_from_u64(seed);

//         // Number of iterations for the test
//         let num_iters = 10000;

//         for i in 0..num_iters {
//             // Generate a random struct name
//             let struct_name = format_ident!("{}", random_ident(&mut rng));
//             let z_struct_name = format_ident!("Z{}", struct_name);
//             let z_struct_meta_name = format_ident!("Z{}Meta", struct_name);

//             // Generate random number of fields (1-10)
//             let field_count = rng.gen_range(1..11);
//             let fields = random_fields(&mut rng, field_count);

//             // Create a named fields collection
//             let syn_fields = syn::punctuated::Punctuated::from_iter(fields.iter().cloned());
//             let fields_named = syn::FieldsNamed {
//                 brace_token: syn::token::Brace::default(),
//                 named: syn_fields,
//             };

//             // Split into meta fields and struct fields
//             let (_, struct_fields) = crate::utils::process_fields(&fields_named);

//             // Call the function we're testing
//             let result = generate_deserialize_impl::<false>(
//                 &struct_name,
//                 &z_struct_name,
//                 &z_struct_meta_name,
//                 &struct_fields,
//                 false,
//             );

//             // Get the generated code as a string for validation
//             let result_str = result.to_string();

//             // Print the first result for debugging
//             if i == 0 {
//                 println!("Generated deserialize_impl code format:\n{}", result_str);
//             }

//             // Verify the result contains expected elements
//             // Basic validation - must be non-empty
//             assert!(
//                 !result_str.is_empty(),
//                 "Failed to generate TokenStream for iteration {}",
//                 i
//             );

//             // Validate that the generated code contains the expected impl definition
//             let impl_pattern = format!(
//                 "impl < 'a > light_zero_copy :: borsh :: Deserialize < 'a > for {}",
//                 struct_name
//             );
//             assert!(
//                 result_str.contains(&impl_pattern),
//                 "Generated code missing impl definition for iteration {}. Expected: {}",
//                 i,
//                 impl_pattern
//             );

//             // Validate type Output is defined
//             let output_pattern = format!("type Output = {} < 'a >", z_struct_name);
//             assert!(
//                 result_str.contains(&output_pattern),
//                 "Generated code missing Output type for iteration {}. Expected: {}",
//                 i,
//                 output_pattern
//             );

//             // Validate the zero_copy_at method is present
//             assert!(
//                 result_str.contains("fn zero_copy_at (bytes : & 'a [u8])"),
//                 "Generated code missing zero_copy_at method for iteration {}",
//                 i
//             );

//             // Check for meta field extraction
//             let meta_extraction_pattern = format!(
//                 "let (__meta , bytes) = light_zero_copy :: Ref :: < & 'a [u8] , {} > :: from_prefix (bytes) ?",
//                 z_struct_meta_name
//             );
//             assert!(
//                 result_str.contains(&meta_extraction_pattern),
//                 "Generated code missing meta field extraction for iteration {}",
//                 i
//             );

//             // Check for return with Ok pattern
//             assert!(
//                 result_str.contains("Ok (("),
//                 "Generated code missing Ok return statement for iteration {}",
//                 i
//             );

//             // Check for the struct initialization
//             let struct_init_pattern = format!("{} {{", z_struct_name);
//             assert!(
//                 result_str.contains(&struct_init_pattern),
//                 "Generated code missing struct initialization for iteration {}",
//                 i
//             );

//             // Check for meta field in the returned struct
//             assert!(
//                 result_str.contains("__meta ,"),
//                 "Generated code missing meta field in struct initialization for iteration {}",
//                 i
//             );
//         }
//     }
// }

/// Generates the ZeroCopyStructInner implementation as a TokenStream
pub fn generate_zero_copy_struct_inner<const MUT: bool>(
    name: &Ident,
    z_struct_name: &Ident,
) -> syn::Result<TokenStream> {
    let result = if MUT {
        quote! {
            // ZeroCopyStructInner implementation
            impl light_zero_copy::borsh_mut::ZeroCopyStructInnerMut for #name {
                type ZeroCopyInnerMut = #z_struct_name<'static>;
            }
        }
    } else {
        quote! {
            // ZeroCopyStructInner implementation
            impl light_zero_copy::borsh::ZeroCopyStructInner for #name {
                type ZeroCopyInner = #z_struct_name<'static>;
            }
        }
    };
    Ok(result)
}

pub fn derive_zero_copy_impl(input: ProcTokenStream) -> syn::Result<proc_macro2::TokenStream> {
    // Parse the input DeriveInput
    let input: DeriveInput = syn::parse(input)?;

    let hasher = false;

    // Process the input to extract struct information
    let (name, z_struct_name, z_struct_meta_name, fields) = utils::process_input(&input)?;

    // Process the fields to separate meta fields and struct fields
    let (meta_fields, struct_fields) = utils::process_fields(fields);

    let meta_struct_def = if !meta_fields.is_empty() {
        meta_struct::generate_meta_struct::<false>(&z_struct_meta_name, &meta_fields, hasher)?
    } else {
        quote! {}
    };

    let z_struct_def = generate_z_struct::<false>(
        &z_struct_name,
        &z_struct_meta_name,
        &struct_fields,
        &meta_fields,
        hasher,
    )?;

    let zero_copy_struct_inner_impl =
        generate_zero_copy_struct_inner::<false>(name, &z_struct_name)?;

    let deserialize_impl = generate_deserialize_impl::<false>(
        name,
        &z_struct_name,
        &z_struct_meta_name,
        &struct_fields,
        meta_fields.is_empty(),
        quote! {},
    )?;

    // Combine all implementations
    let expanded = quote! {
        #meta_struct_def
        #z_struct_def
        #zero_copy_struct_inner_impl
        #deserialize_impl
    };

    Ok(expanded)
}

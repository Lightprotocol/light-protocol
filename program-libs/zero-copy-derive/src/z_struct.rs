use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::{parse_quote, parse_str, Field, Ident, Type};

use crate::utils;

/// Enum representing the different field types for zero-copy struct
/// (Name, Type)
#[derive(Debug)]
pub enum FieldType<'a> {
    VecU8(&'a Ident),
    VecCopy(&'a Ident, &'a Type),
    VecNonCopy(&'a Ident, &'a Type),
    Array(&'a Ident, &'a Type),
    Option(&'a Ident, &'a Type),
    OptionU64(&'a Ident),
    OptionU32(&'a Ident),
    OptionU16(&'a Ident),
    Pubkey(&'a Ident),
    IntegerU64(&'a Ident),
    IntegerU32(&'a Ident),
    IntegerU16(&'a Ident),
    IntegerU8(&'a Ident),
    Bool(&'a Ident),
    CopyU8Bool(&'a Ident),
    Copy(&'a Ident, &'a Type),
    NonCopy(&'a Ident, &'a Type),
}

impl<'a> FieldType<'a> {
    /// Get the name of the field
    pub fn name(&self) -> &'a Ident {
        match self {
            FieldType::VecU8(name) => name,
            FieldType::VecCopy(name, _) => name,
            FieldType::VecNonCopy(name, _) => name,
            FieldType::Array(name, _) => name,
            FieldType::Option(name, _) => name,
            FieldType::OptionU64(name) => name,
            FieldType::OptionU32(name) => name,
            FieldType::OptionU16(name) => name,
            FieldType::Pubkey(name) => name,
            FieldType::IntegerU64(name) => name,
            FieldType::IntegerU32(name) => name,
            FieldType::IntegerU16(name) => name,
            FieldType::IntegerU8(name) => name,
            FieldType::Bool(name) => name,
            FieldType::CopyU8Bool(name) => name,
            FieldType::Copy(name, _) => name,
            FieldType::NonCopy(name, _) => name,
        }
    }
}

/// Analyze struct fields and return vector of FieldType enums
pub fn analyze_struct_fields<'a>(struct_fields: &'a [&'a Field]) -> Vec<FieldType<'a>> {
    struct_fields
        .iter()
        .map(|field| {
            if let Some(field_name) = &field.ident {
                let field_type = &field.ty;

                if utils::is_vec_type(field_type) {
                    if let Some(inner_type) = utils::get_vec_inner_type(field_type) {
                        if inner_type.to_token_stream().to_string() == "u8" {
                            FieldType::VecU8(field_name)
                        } else if utils::is_copy_type(inner_type) {
                            FieldType::VecCopy(field_name, inner_type)
                        } else {
                            FieldType::VecNonCopy(field_name, field_type)
                        }
                    } else {
                        panic!("Could not determine inner type of Vec {:?}", field_type);
                    }
                } else if let Type::Array(_) = field_type {
                    FieldType::Array(field_name, field_type)
                } else if utils::is_option_type(field_type) {
                    // Check the inner type of the Option and convert to appropriate FieldType
                    if let Some(inner_type) = utils::get_option_inner_type(field_type) {
                        if utils::is_primitive_integer(inner_type) {
                            let field_ty_str = inner_type.to_token_stream().to_string();
                            match field_ty_str.as_str() {
                                "u64" => FieldType::OptionU64(field_name),
                                "u32" => FieldType::OptionU32(field_name),
                                "u16" => FieldType::OptionU16(field_name),
                                _ => FieldType::Option(field_name, field_type),
                            }
                        } else {
                            FieldType::Option(field_name, field_type)
                        }
                    } else {
                        FieldType::Option(field_name, field_type)
                    }
                } else if utils::is_pubkey_type(field_type) {
                    FieldType::Pubkey(field_name)
                } else if utils::is_bool_type(field_type) {
                    FieldType::Bool(field_name)
                } else if utils::is_primitive_integer(field_type) {
                    let field_ty_str = field_type.to_token_stream().to_string();
                    match field_ty_str.as_str() {
                        "u64" => FieldType::IntegerU64(field_name),
                        "u32" => FieldType::IntegerU32(field_name),
                        "u16" => FieldType::IntegerU16(field_name),
                        "u8" => FieldType::IntegerU8(field_name),
                        _ => unimplemented!("Unsupported integer type: {}", field_ty_str),
                    }
                } else if utils::is_copy_type(field_type) {
                    if field_type.to_token_stream().to_string() == "u8"
                        || field_type.to_token_stream().to_string() == "bool"
                    {
                        FieldType::CopyU8Bool(field_name)
                    } else {
                        FieldType::Copy(field_name, field_type)
                    }
                } else {
                    FieldType::NonCopy(field_name, field_type)
                }
            } else {
                panic!("Could not determine field name");
            }
        })
        .collect()
}

/// Generate struct fields with zerocopy types based on field type enum
fn generate_struct_fields_with_zerocopy_types<'a, const MUT: bool>(
    struct_fields: &'a [&'a Field],
    hasher: &'a bool,
) -> impl Iterator<Item = TokenStream> + 'a {
    let field_types = analyze_struct_fields(struct_fields);
    field_types
        .into_iter()
        .zip(struct_fields.iter())
        .map(|(field_type, field)| {
            let attributes = if *hasher {
                field
                    .attrs
                    .iter()
                    .map(|attr| {
                        let path = attr;
                        quote! { #path }
                    })
                    .collect::<Vec<_>>()
            } else {
                vec![quote! {}]
            };
            let (mutability, import_path, import_slice, camel_case_suffix): (
                syn::Type,
                syn::Ident,
                syn::Ident,
                String,
            ) = if MUT {
                (
                    parse_str("&'a mut [u8]").unwrap(),
                    format_ident!("borsh_mut"),
                    format_ident!("slice_mut"),
                    String::from("Mut"),
                )
            } else {
                (
                    parse_str("&'a [u8]").unwrap(),
                    format_ident!("borsh"),
                    format_ident!("slice"),
                    String::new(),
                )
            };
            let trait_name: syn::Type = parse_str(
                format!(
                    "light_zero_copy::{}::Deserialize{}",
                    import_path, camel_case_suffix
                )
                .as_str(),
            )
            .unwrap();
            let slice_name: syn::Type = parse_str(
                format!(
                    "light_zero_copy::{}::ZeroCopySlice{}Borsh",
                    import_slice, camel_case_suffix
                )
                .as_str(),
            )
            .unwrap();
            let struct_inner_trait_name: syn::Type = parse_str(
                format!(
                    "light_zero_copy::{}::ZeroCopyStructInner{1}::ZeroCopyInner{1}",
                    import_path, camel_case_suffix
                )
                .as_str(),
            )
            .unwrap();
            match field_type {
                FieldType::VecU8(field_name) => {
                    quote! {
                        #(#attributes)*
                        pub #field_name: #mutability
                    }
                }
                FieldType::VecCopy(field_name, inner_type) => {
                    quote! {
                    #(#attributes)*
                    pub #field_name: #slice_name<'a, <#inner_type as #struct_inner_trait_name>>
                                        }
                }
                FieldType::VecNonCopy(field_name, field_type) => {
                    quote! {
                        #(#attributes)*
                        pub #field_name: <#field_type as #trait_name<'a>>::Output
                    }
                }
                FieldType::Array(field_name, field_type) => {
                    quote! {
                        #(#attributes)*
                        pub #field_name: light_zero_copy::Ref<#mutability , #field_type>
                    }
                }
                FieldType::Option(field_name, field_type) => {
                    quote! {
                        #(#attributes)*
                        pub #field_name: <#field_type as #trait_name<'a>>::Output
                    }
                }
                FieldType::OptionU64(field_name) => {
                    let field_ty_zerocopy = utils::convert_to_zerocopy_type(&parse_quote!(u64));
                    quote! {
                        #(#attributes)*
                        pub #field_name: Option<light_zero_copy::Ref<#mutability, #field_ty_zerocopy>>
                    }
                }
                FieldType::OptionU32(field_name) => {
                    let field_ty_zerocopy = utils::convert_to_zerocopy_type(&parse_quote!(u32));
                    quote! {
                        #(#attributes)*
                        pub #field_name: Option<light_zero_copy::Ref<#mutability, #field_ty_zerocopy>>
                    }
                }
                FieldType::OptionU16(field_name) => {
                    let field_ty_zerocopy = utils::convert_to_zerocopy_type(&parse_quote!(u16));
                    quote! {
                        #(#attributes)*
                        pub #field_name: Option<light_zero_copy::Ref<#mutability, #field_ty_zerocopy>>
                    }
                }
                FieldType::Pubkey(field_name) => {
                    quote! {
                        #(#attributes)*
                        pub #field_name: <Pubkey as #trait_name<'a>>::Output
                    }
                }
                FieldType::IntegerU64(field_name) => {
                    let field_ty_zerocopy = utils::convert_to_zerocopy_type(&parse_quote!(u64));
                    quote! {
                        #(#attributes)*
                        pub #field_name: light_zero_copy::Ref<#mutability, #field_ty_zerocopy>
                    }
                }
                FieldType::IntegerU32(field_name) => {
                    let field_ty_zerocopy = utils::convert_to_zerocopy_type(&parse_quote!(u32));
                    quote! {
                        #(#attributes)*
                        pub #field_name: light_zero_copy::Ref<#mutability, #field_ty_zerocopy>
                    }
                }
                FieldType::IntegerU16(field_name) => {
                    let field_ty_zerocopy = utils::convert_to_zerocopy_type(&parse_quote!(u16));
                    quote! {
                        #(#attributes)*
                        pub #field_name: light_zero_copy::Ref<#mutability, #field_ty_zerocopy>
                    }
                }
                FieldType::IntegerU8(field_name) => {
                    if MUT {
                        quote! {
                            #(#attributes)*
                            pub #field_name: light_zero_copy::Ref<#mutability, u8>
                        }
                    } else {
                        quote! {
                            #(#attributes)*
                            pub #field_name: <u8 as #trait_name<'a>>::Output
                        }
                    }
                }
                FieldType::Bool(field_name) => {
                    if MUT {
                        quote! {
                            #(#attributes)*
                            pub #field_name: light_zero_copy::Ref<#mutability, u8>
                        }
                    } else {
                        quote! {
                            #(#attributes)*
                            pub #field_name: <u8 as #trait_name<'a>>::Output
                        }
                    }
                }
                FieldType::CopyU8Bool(field_name) => {
                    quote! {
                        #(#attributes)*
                        pub #field_name: <u8 as #trait_name<'a>>::Output
                    }
                }
                FieldType::Copy(field_name, field_type) => {
                    let zerocopy_type = utils::convert_to_zerocopy_type(field_type);
                    quote! {
                        #(#attributes)*
                        pub #field_name: light_zero_copy::Ref<#mutability , #zerocopy_type>
                    }
                }
                FieldType::NonCopy(field_name, field_type) => {
                    quote! {
                        #(#attributes)*
                        pub #field_name: <#field_type as #trait_name<'a>>::Output
                    }
                }
            }
        })
}

/// Generate accessor methods for boolean fields in struct_fields.
/// We need accessors because booleans are stored as u8.
fn generate_bool_accessor_methods<'a, const MUT: bool>(
    struct_fields: &'a [&'a Field],
) -> impl Iterator<Item = TokenStream> + 'a {
    struct_fields.iter().filter_map(|field| {
        let field_name = &field.ident;
        let field_type = &field.ty;

        if utils::is_bool_type(field_type) {
            let comparison = if MUT {
                quote! { *self.#field_name > 0 }
            } else {
                quote! { self.#field_name > 0 }
            };
            
            Some(quote! {
                pub fn #field_name(&self) -> bool {
                    #comparison
                }
            })
        } else {
            None
        }
    })
}

/// Generates the ZStruct definition as a TokenStream
pub fn generate_z_struct<const MUT: bool>(
    z_struct_name: &Ident,
    z_struct_meta_name: &Ident,
    struct_fields: &[&Field],
    meta_fields: &[&Field],
    hasher: bool,
) -> TokenStream {
    let mut z_struct_name = z_struct_name.clone();
    let mut z_struct_meta_name = z_struct_meta_name.clone();
    let mutability: syn::Type = if MUT {
        z_struct_name = format_ident!("{}Mut", z_struct_name);
        z_struct_meta_name = format_ident!("{}Mut", z_struct_meta_name);
        parse_str("&'a mut [u8]").unwrap()
    } else {
        parse_str("&'a [u8] ").unwrap()
    };

    let derive_clone = if MUT {
        quote! {}
    } else {
        quote! {, Clone }
    };
    let struct_fields_with_zerocopy_types =
        generate_struct_fields_with_zerocopy_types::<MUT>(struct_fields, &hasher);

    let derive_hasher = if hasher {
        quote! {
            , LightHasher
        }
    } else {
        quote! {}
    };
    let hasher_flatten = if hasher {
        quote! {
            #[flatten]
        }
    } else {
        quote! {}
    };

    let partial_eq_derive = if MUT { quote!() } else { quote!(, PartialEq) };
    
    let mut z_struct = if meta_fields.is_empty() {
        quote! {
            // ZStruct
            #[derive(Debug #partial_eq_derive #derive_clone #derive_hasher)]
            pub struct #z_struct_name<'a> {
                #(#struct_fields_with_zerocopy_types,)*
            }
        }
    } else {
        let mut tokens = quote! {
            // ZStruct
            #[derive(Debug #partial_eq_derive #derive_clone #derive_hasher)]
            pub struct #z_struct_name<'a> {
                #hasher_flatten
                __meta: light_zero_copy::Ref<#mutability, #z_struct_meta_name>,
                #(#struct_fields_with_zerocopy_types,)*
            }
            impl<'a> core::ops::Deref for #z_struct_name<'a> {
                type Target =  light_zero_copy::Ref<#mutability  , #z_struct_meta_name>;

                fn deref(&self) -> &Self::Target {
                    &self.__meta
                }
            }
        };

        if MUT {
            tokens.append_all(quote! {
                impl<'a> core::ops::DerefMut for #z_struct_name<'a> {
                    fn deref_mut(&mut self) ->  &mut Self::Target {
                        &mut self.__meta
                    }
                }
            });
        }
        tokens
    };

    if !meta_fields.is_empty() {
        let meta_bool_accessor_methods = generate_bool_accessor_methods::<false>(meta_fields);
        z_struct.append_all(quote! {
            // Implement methods for ZStruct
            impl<'a> #z_struct_name<'a> {
                #(#meta_bool_accessor_methods)*
            }
        })
    };

    if !struct_fields.is_empty() {
        let bool_accessor_methods = generate_bool_accessor_methods::<MUT>(struct_fields);
        z_struct.append_all(quote! {
            // Implement methods for ZStruct
            impl<'a> #z_struct_name<'a> {
                #(#bool_accessor_methods)*
            }

        });
    }
    z_struct
}

#[cfg(test)]
mod tests {
    use quote::format_ident;
    use rand::{prelude::SliceRandom, rngs::StdRng, thread_rng, Rng, SeedableRng};
    use syn::parse_quote;

    use super::*;

    /// Generate a safe field name for testing
    fn random_ident(rng: &mut StdRng) -> String {
        // Use predetermined safe field names
        const FIELD_NAMES: &[&str] = &[
            "field1", "field2", "field3", "field4", "field5", "value", "data", "count", "size",
            "flag", "name", "id", "code", "index", "key", "amount", "balance", "total", "result",
            "status",
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
    fn random_field(rng: &mut StdRng) -> Field {
        let name = random_ident(rng);
        let ty = random_type(rng, 0);

        // Use a safer approach to create the field
        let name_ident = format_ident!("{}", name);
        parse_quote!(pub #name_ident: #ty)
    }

    /// Generate a list of random fields
    fn random_fields(rng: &mut StdRng, count: usize) -> Vec<Field> {
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
            let (meta_fields, struct_fields) = crate::utils::process_fields(&fields_named);

            // Call the function we're testing
            let result = generate_z_struct::<false>(
                &z_struct_name,
                &z_struct_meta_name,
                &struct_fields,
                &meta_fields,
                false,
            );

            // Get the generated code as a string for validation
            let result_str = result.to_string();

            // Print the first result for debugging
            println!("Generated code format:\n{}", result_str);

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
                    !result_str.contains("impl < 'a > core :: ops :: Deref"),
                    "Generated code missing Deref implementation for iteration {}",
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
                    result_str.contains("impl < 'a > core :: ops :: Deref"),
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
                    result_str.contains("light_zero_copy :: Ref"),
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
}

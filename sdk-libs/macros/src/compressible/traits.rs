//! Trait derivation for compressible accounts.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    DeriveInput, Expr, Field, Ident, ItemStruct, Result, Token,
};

use super::utils::{
    extract_fields_from_derive_input, extract_fields_from_item_struct, is_copy_type,
};

struct CompressAsFields {
    fields: Punctuated<CompressAsField, Token![,]>,
}

struct CompressAsField {
    name: Ident,
    value: Expr,
}

impl Parse for CompressAsField {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: Expr = input.parse()?;
        Ok(CompressAsField { name, value })
    }
}

impl Parse for CompressAsFields {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(CompressAsFields {
            fields: Punctuated::parse_terminated(input)?,
        })
    }
}

/// Validates that the struct has a `compression_info` field
fn validate_compression_info_field(
    fields: &Punctuated<Field, Token![,]>,
    struct_name: &Ident,
) -> Result<()> {
    let has_compression_info_field = fields.iter().any(|field| {
        field
            .ident
            .as_ref()
            .is_some_and(|name| name == "compression_info")
    });

    if !has_compression_info_field {
        return Err(syn::Error::new_spanned(
            struct_name,
            "Struct must have a 'compression_info' field of type Option<CompressionInfo>",
        ));
    }

    Ok(())
}

/// Generates the HasCompressionInfo trait implementation
fn generate_has_compression_info_impl(struct_name: &Ident) -> TokenStream {
    quote! {
        impl light_sdk::compressible::HasCompressionInfo for #struct_name {
            fn compression_info(&self) -> &light_sdk::compressible::CompressionInfo {
                self.compression_info.as_ref().expect("compression_info must be set")
            }

            fn compression_info_mut(&mut self) -> &mut light_sdk::compressible::CompressionInfo {
                self.compression_info.as_mut().expect("compression_info must be set")
            }

            fn compression_info_mut_opt(&mut self) -> &mut Option<light_sdk::compressible::CompressionInfo> {
                &mut self.compression_info
            }

            fn set_compression_info_none(&mut self) {
                self.compression_info = None;
            }
        }
    }
}

/// Generates field assignments for CompressAs trait, handling overrides and copy types
fn generate_compress_as_field_assignments(
    fields: &Punctuated<Field, Token![,]>,
    compress_as_fields: &Option<CompressAsFields>,
) -> Vec<TokenStream> {
    let mut field_assignments = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;

        if field.attrs.iter().any(|attr| attr.path().is_ident("skip")) {
            continue;
        }

        let has_override = compress_as_fields
            .as_ref()
            .is_some_and(|cas| cas.fields.iter().any(|f| &f.name == field_name));

        if has_override {
            let override_value = compress_as_fields
                .as_ref()
                .unwrap()
                .fields
                .iter()
                .find(|f| &f.name == field_name)
                .unwrap();
            let value = &override_value.value;
            field_assignments.push(quote! {
                #field_name: #value,
            });
        } else if is_copy_type(field_type) {
            field_assignments.push(quote! {
                #field_name: self.#field_name,
            });
        } else {
            field_assignments.push(quote! {
                #field_name: self.#field_name.clone(),
            });
        }
    }

    field_assignments
}

/// Generates the CompressAs trait implementation
fn generate_compress_as_impl(
    struct_name: &Ident,
    field_assignments: &[TokenStream],
) -> TokenStream {
    quote! {
        impl light_sdk::compressible::CompressAs for #struct_name {
            type Output = Self;

            fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
                std::borrow::Cow::Owned(Self {
                    compression_info: None,
                    #(#field_assignments)*
                })
            }
        }
    }
}

/// Generates size calculation fields for the Size trait
fn generate_size_fields(fields: &Punctuated<Field, Token![,]>) -> Vec<TokenStream> {
    let mut size_fields = Vec::new();

    for field in fields.iter() {
        let field_name = field.ident.as_ref().unwrap();

        if field.attrs.iter().any(|attr| attr.path().is_ident("skip")) {
            continue;
        }

        size_fields.push(quote! {
            + self.#field_name.try_to_vec().expect("Failed to serialize").len()
        });
    }

    size_fields
}

/// Generates the Size trait implementation
fn generate_size_impl(struct_name: &Ident, size_fields: &[TokenStream]) -> TokenStream {
    quote! {
        impl light_sdk::account::Size for #struct_name {
            fn size(&self) -> usize {
                // Always allocate space for Some(CompressionInfo) since it will be set during decompression
                // CompressionInfo size: 1 byte (Option discriminant) + <CompressionInfo as Space>::INIT_SPACE
                let compression_info_size = 1 + <light_sdk::compressible::CompressionInfo as light_sdk::compressible::Space>::INIT_SPACE;
                compression_info_size #(#size_fields)*
            }
        }
    }
}

/// Generates the CompressedInitSpace trait implementation
fn generate_compressed_init_space_impl(struct_name: &Ident) -> TokenStream {
    quote! {
        impl light_sdk::compressible::CompressedInitSpace for #struct_name {
            const COMPRESSED_INIT_SPACE: usize = Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE;
        }
    }
}

pub fn derive_compress_as(input: ItemStruct) -> Result<TokenStream> {
    let struct_name = &input.ident;
    let fields = extract_fields_from_item_struct(&input)?;

    let compress_as_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("compress_as"));

    let compress_as_fields = if let Some(attr) = compress_as_attr {
        Some(attr.parse_args::<CompressAsFields>()?)
    } else {
        None
    };

    let field_assignments = generate_compress_as_field_assignments(fields, &compress_as_fields);
    Ok(generate_compress_as_impl(struct_name, &field_assignments))
}

pub fn derive_has_compression_info(input: syn::ItemStruct) -> Result<TokenStream> {
    let struct_name = &input.ident;
    let fields = extract_fields_from_item_struct(&input)?;

    validate_compression_info_field(fields, struct_name)?;
    Ok(generate_has_compression_info_impl(struct_name))
}

pub fn derive_compressible(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;
    let fields = extract_fields_from_derive_input(&input)?;

    // Extract compress_as attribute
    let compress_as_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("compress_as"));

    let compress_as_fields = if let Some(attr) = compress_as_attr {
        Some(attr.parse_args::<CompressAsFields>()?)
    } else {
        None
    };

    // Validate compression_info field exists
    validate_compression_info_field(fields, struct_name)?;

    // Generate all trait implementations using helper functions
    let has_compression_info_impl = generate_has_compression_info_impl(struct_name);

    let field_assignments = generate_compress_as_field_assignments(fields, &compress_as_fields);
    let compress_as_impl = generate_compress_as_impl(struct_name, &field_assignments);

    let size_fields = generate_size_fields(fields);
    let size_impl = generate_size_impl(struct_name, &size_fields);

    let compressed_init_space_impl = generate_compressed_init_space_impl(struct_name);

    // Combine all implementations
    Ok(quote! {
        #has_compression_info_impl
        #compress_as_impl
        #size_impl
        #compressed_init_space_impl
    })
}

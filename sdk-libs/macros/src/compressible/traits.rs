//! Trait derivation for compressible accounts.
//!
//! This module provides derive macros for:
//! - `HasCompressionInfo`: Adds CompressionInfo field handling
//! - `CompressAs`: Allows selective field compression
//! - `Compressible`: Full compressible account support

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Data, DeriveInput, Expr, Fields, Ident, ItemStruct, Result, Token,
};

/// Parse the compress_as attribute content
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

/// Generates CompressAs trait implementation for a struct with optional compress_as attribute
pub fn derive_compress_as(input: ItemStruct) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Find the compress_as attribute (optional)
    let compress_as_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("compress_as"));

    // Parse the attribute content if it exists
    let compress_as_fields = if let Some(attr) = compress_as_attr {
        Some(attr.parse_args::<CompressAsFields>()?)
    } else {
        None
    };

    // Get struct fields
    let fields = match &input.fields {
        Fields::Named(fields) => &fields.named,
        _ => {
            return Err(syn::Error::new_spanned(
                struct_name,
                "CompressAs only supports structs with named fields",
            ))
        }
    };

    // Build the compress_as method body
    let mut field_assignments = Vec::new();

    // Add all field copies or default values
    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;

        // Skip compression_info field
        if field.attrs.iter().any(|attr| attr.path().is_ident("skip")) {
            continue;
        }

        // Check if this field has a compress_as override
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
            // For Copy types, copy the value
            field_assignments.push(quote! {
                #field_name: self.#field_name,
            });
        } else {
            // For non-Copy types, clone the value
            field_assignments.push(quote! {
                #field_name: self.#field_name.clone(),
            });
        }
    }

    Ok(quote! {
        impl light_sdk::compressible::CompressAs for #struct_name {
            type Output = Self;

            fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
                std::borrow::Cow::Owned(Self {
                    compression_info: None,
                    #(#field_assignments)*
                })
            }
        }
    })
}

/// Generates HasCompressionInfo trait implementation
pub fn derive_has_compression_info(input: syn::ItemStruct) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Verify compression_info field exists
    let fields = match &input.fields {
        Fields::Named(fields) => &fields.named,
        _ => {
            return Err(syn::Error::new_spanned(
                struct_name,
                "HasCompressionInfo only supports structs with named fields",
            ))
        }
    };

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

    Ok(quote! {
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
    })
}

/// Generates full Compressible trait implementation with Size, HasCompressionInfo, and CompressAs
pub fn derive_compressible(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Find the compress_as attribute (optional)
    let compress_as_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("compress_as"));

    // Parse the attribute content if it exists
    let compress_as_fields = if let Some(attr) = compress_as_attr {
        Some(attr.parse_args::<CompressAsFields>()?)
    } else {
        None
    };

    // Get struct fields
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    struct_name,
                    "Compressible only supports structs with named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                struct_name,
                "Compressible only supports structs",
            ))
        }
    };

    // Verify compression_info field exists
    let has_compression_info_field = fields.iter().any(|field| {
        field
            .ident
            .as_ref()
            .is_some_and(|name| name == "compression_info")
    });

    if !has_compression_info_field {
        return Err(syn::Error::new_spanned(
            struct_name,
            "Compressible struct must have a 'compression_info' field of type Option<CompressionInfo>",
        ));
    }

    // Build the compress_as method body
    let mut field_assignments = Vec::new();

    for field in fields.iter() {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;

        // Skip compression_info field
        if field.attrs.iter().any(|attr| attr.path().is_ident("skip")) {
            continue;
        }

        // Check if this field has a compress_as override
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

    // Calculate size (borsh-serialized size approximation)
    let mut size_fields = Vec::new();
    for field in fields.iter() {
        let field_name = field.ident.as_ref().unwrap();

        // Skip compression_info since it's excluded from hashing/serialization
        if field.attrs.iter().any(|attr| attr.path().is_ident("skip")) {
            continue;
        }

        size_fields.push(quote! {
            + self.#field_name.try_to_vec().expect("Failed to serialize").len()
        });
    }

    Ok(quote! {
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

        impl light_sdk::compressible::CompressAs for #struct_name {
            type Output = Self;

            fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
                std::borrow::Cow::Owned(Self {
                    compression_info: None,
                    #(#field_assignments)*
                })
            }
        }

        impl light_sdk::account::Size for #struct_name {
            fn size(&self) -> usize {
                0 #(#size_fields)*
            }
        }

        impl light_sdk::compressible::CompressedInitSpace for #struct_name {
            const COMPRESSED_INIT_SPACE: usize = Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE;
        }
    })
}

/// Helper function to determine if a type implements Copy
fn is_copy_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let type_name = segment.ident.to_string();
            matches!(
                type_name.as_str(),
                "u8" | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "f32"
                    | "f64"
                    | "bool"
                    | "char"
                    | "Pubkey"
            ) || has_copy_inner_type(&segment.arguments)
        } else {
            false
        }
    } else {
        matches!(ty, syn::Type::Array(_))
    }
}

/// Check if a generic type argument is a Copy type (e.g., Option<u64>)
fn has_copy_inner_type(args: &syn::PathArguments) -> bool {
    if let syn::PathArguments::AngleBracketed(angle_args) = args {
        angle_args.args.iter().any(|arg| {
            if let syn::GenericArgument::Type(inner_ty) = arg {
                is_copy_type(inner_ty)
            } else {
                false
            }
        })
    } else {
        false
    }
}

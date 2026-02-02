//! Trait derivation for compressible accounts.

use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{punctuated::Punctuated, DeriveInput, Expr, Field, Ident, ItemStruct, Result, Token};

use super::{
    utils::{extract_fields_from_derive_input, extract_fields_from_item_struct, is_copy_type},
    validation::validate_compression_info_field,
};

/// A single field override in #[compress_as(field = expr)]
pub(crate) struct CompressAsField {
    pub name: Ident,
    pub value: Expr,
}

/// Collection of field overrides parsed from #[compress_as(...)]
/// Uses darling's FromMeta to collect arbitrary name=value pairs.
#[derive(Default)]
pub(crate) struct CompressAsFields {
    pub fields: Vec<CompressAsField>,
}

impl FromMeta for CompressAsFields {
    fn from_list(items: &[darling::ast::NestedMeta]) -> darling::Result<Self> {
        items
            .iter()
            .map(|item| match item {
                darling::ast::NestedMeta::Meta(syn::Meta::NameValue(nv)) => {
                    let name = nv.path.get_ident().cloned().ok_or_else(|| {
                        darling::Error::custom("expected field identifier").with_span(&nv.path)
                    })?;
                    Ok(CompressAsField {
                        name,
                        value: nv.value.clone(),
                    })
                }
                other => Err(darling::Error::custom("expected field = expr").with_span(other)),
            })
            .collect::<darling::Result<Vec<_>>>()
            .map(|fields| CompressAsFields { fields })
    }
}

/// Parses compress_as overrides from struct attributes.
/// Used by LightAccount derive to extract field override values.
pub(crate) fn parse_compress_as_overrides(
    attrs: &[syn::Attribute],
) -> Result<Option<CompressAsFields>> {
    let compress_as_attr = attrs
        .iter()
        .find(|attr| attr.path().is_ident("compress_as"));

    if let Some(attr) = compress_as_attr {
        let parsed = CompressAsFields::from_meta(&attr.meta)
            .map_err(|e| syn::Error::new_spanned(attr, e.to_string()))?;
        Ok(Some(parsed))
    } else {
        Ok(None)
    }
}

/// Generates the CompressionInfoField trait implementation.
/// HasCompressionInfo is provided via blanket impl in light-sdk.
fn generate_has_compression_info_impl(
    struct_name: &Ident,
    compression_info_first: bool,
) -> TokenStream {
    quote! {
        impl light_account::CompressionInfoField for #struct_name {
            const COMPRESSION_INFO_FIRST: bool = #compression_info_first;

            fn compression_info_field(&self) -> &Option<light_account::CompressionInfo> {
                &self.compression_info
            }
            fn compression_info_field_mut(&mut self) -> &mut Option<light_account::CompressionInfo> {
                &mut self.compression_info
            }
        }
    }
}

/// Generates field assignments for CompressAs trait, handling overrides and copy types.
/// Auto-skips `compression_info` field and fields marked with `#[skip]`.
fn generate_compress_as_field_assignments(
    fields: &Punctuated<Field, Token![,]>,
    compress_as_fields: &Option<CompressAsFields>,
) -> Vec<TokenStream> {
    let mut field_assignments = Vec::new();

    for field in fields {
        let Some(field_name) = field.ident.as_ref() else {
            continue;
        };
        let field_type = &field.ty;

        // Auto-skip compression_info field (handled separately in CompressAs impl)
        if field_name == "compression_info" {
            continue;
        }

        // Also skip fields explicitly marked with #[skip]
        if field.attrs.iter().any(|attr| attr.path().is_ident("skip")) {
            continue;
        }

        let override_field = compress_as_fields
            .as_ref()
            .and_then(|cas| cas.fields.iter().find(|f| &f.name == field_name));

        if let Some(override_value) = override_field {
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
        impl light_account::CompressAs for #struct_name {
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

/// Generates the Size trait implementation.
/// Uses max(INIT_SPACE, serialized_len) to ensure enough space while handling edge cases.
fn generate_size_impl(struct_name: &Ident) -> TokenStream {
    quote! {
        impl light_sdk::account::Size for #struct_name {
            #[inline]
            fn size(&self) -> std::result::Result<usize, solana_program_error::ProgramError> {
                // Use Anchor's compile-time INIT_SPACE as the baseline.
                // Fall back to serialized length if it's somehow larger (edge case safety).
                let init_space = <Self as anchor_lang::Space>::INIT_SPACE;
                let serialized_len = self.try_to_vec()
                    .map_err(|_| solana_program_error::ProgramError::BorshIoError("serialization failed".to_string()))?
                    .len();
                Ok(core::cmp::max(init_space, serialized_len))
            }
        }
    }
}

/// Generates the CompressedInitSpace trait implementation
fn generate_compressed_init_space_impl(struct_name: &Ident) -> TokenStream {
    quote! {
        impl light_account::CompressedInitSpace for #struct_name {
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
        let parsed = CompressAsFields::from_meta(&attr.meta)
            .map_err(|e| syn::Error::new_spanned(attr, e.to_string()))?;
        Some(parsed)
    } else {
        None
    };

    let field_assignments = generate_compress_as_field_assignments(fields, &compress_as_fields);
    Ok(generate_compress_as_impl(struct_name, &field_assignments))
}

pub fn derive_has_compression_info(input: syn::ItemStruct) -> Result<TokenStream> {
    let struct_name = &input.ident;
    let fields = extract_fields_from_item_struct(&input)?;

    let compression_info_first = validate_compression_info_field(fields, struct_name)?;
    Ok(generate_has_compression_info_impl(
        struct_name,
        compression_info_first,
    ))
}

pub fn derive_compressible(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;
    let fields = extract_fields_from_derive_input(&input)?;

    // Extract compress_as attribute using darling
    let compress_as_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("compress_as"));

    let compress_as_fields = if let Some(attr) = compress_as_attr {
        let parsed = CompressAsFields::from_meta(&attr.meta)
            .map_err(|e| syn::Error::new_spanned(attr, e.to_string()))?;
        Some(parsed)
    } else {
        None
    };

    // Validate compression_info field exists and get its position
    let compression_info_first = validate_compression_info_field(fields, struct_name)?;

    // Generate all trait implementations using helper functions
    let has_compression_info_impl =
        generate_has_compression_info_impl(struct_name, compression_info_first);

    let field_assignments = generate_compress_as_field_assignments(fields, &compress_as_fields);
    let compress_as_impl = generate_compress_as_impl(struct_name, &field_assignments);

    let size_impl = generate_size_impl(struct_name);

    let compressed_init_space_impl = generate_compressed_init_space_impl(struct_name);

    // Combine all implementations
    Ok(quote! {
        #has_compression_info_impl
        #compress_as_impl
        #size_impl
        #compressed_init_space_impl
    })
}

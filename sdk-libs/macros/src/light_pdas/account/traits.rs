//! Trait derivation for compressible accounts.

use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{punctuated::Punctuated, DeriveInput, Expr, Field, Ident, ItemStruct, Result, Token};

use super::utils::{
    extract_fields_from_derive_input, extract_fields_from_item_struct, is_copy_type,
};

/// A single field override in #[compress_as(field = expr)]
struct CompressAsField {
    name: Ident,
    value: Expr,
}

/// Collection of field overrides parsed from #[compress_as(...)]
/// Uses darling's FromMeta to collect arbitrary name=value pairs.
#[derive(Default)]
struct CompressAsFields {
    fields: Vec<CompressAsField>,
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

/// Validates that the struct has a `compression_info` field as first or last field.
/// Returns `Ok(true)` if first, `Ok(false)` if last, `Err` if missing or in middle.
fn validate_compression_info_field(
    fields: &Punctuated<Field, Token![,]>,
    struct_name: &Ident,
) -> Result<bool> {
    let field_count = fields.len();
    if field_count == 0 {
        return Err(syn::Error::new_spanned(
            struct_name,
            "Struct must have at least one field",
        ));
    }

    let first_is_compression_info = fields
        .first()
        .and_then(|f| f.ident.as_ref())
        .is_some_and(|name| name == "compression_info");

    let last_is_compression_info = fields
        .last()
        .and_then(|f| f.ident.as_ref())
        .is_some_and(|name| name == "compression_info");

    if first_is_compression_info {
        Ok(true)
    } else if last_is_compression_info {
        Ok(false)
    } else {
        Err(syn::Error::new_spanned(
            struct_name,
            "Field 'compression_info: Option<CompressionInfo>' must be the first or last field in the struct \
             for efficient serialization. Move it to the beginning or end of your struct definition.",
        ))
    }
}

/// Generates the CompressionInfoField trait implementation.
/// HasCompressionInfo is provided via blanket impl in light-sdk.
fn generate_has_compression_info_impl(struct_name: &Ident, compression_info_first: bool) -> TokenStream {
    quote! {
        impl light_sdk::interface::CompressionInfoField for #struct_name {
            const COMPRESSION_INFO_FIRST: bool = #compression_info_first;

            fn compression_info_field(&self) -> &Option<light_sdk::interface::CompressionInfo> {
                &self.compression_info
            }
            fn compression_info_field_mut(&mut self) -> &mut Option<light_sdk::interface::CompressionInfo> {
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
        impl light_sdk::interface::CompressAs for #struct_name {
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
        impl light_sdk::interface::CompressedInitSpace for #struct_name {
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
    Ok(generate_has_compression_info_impl(struct_name, compression_info_first))
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
    let has_compression_info_impl = generate_has_compression_info_impl(struct_name, compression_info_first);

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

/// Validates that the struct has a `compression_info` field for Pod types.
/// Unlike Borsh version, the field type is `CompressionInfo` (not `Option<CompressionInfo>`).
/// Returns `Ok(())` if found, `Err` if missing.
fn validate_pod_compression_info_field(
    fields: &Punctuated<Field, Token![,]>,
    struct_name: &Ident,
) -> Result<()> {
    let has_compression_info = fields
        .iter()
        .any(|f| f.ident.as_ref().is_some_and(|name| name == "compression_info"));

    if !has_compression_info {
        return Err(syn::Error::new_spanned(
            struct_name,
            "Pod struct must have a 'compression_info: CompressionInfo' field (non-optional). \
             For Pod types, use `light_compressible::compression_info::CompressionInfo`.",
        ));
    }
    Ok(())
}

/// Validates that the struct has `#[repr(C)]` attribute required for Pod types.
fn validate_repr_c(attrs: &[syn::Attribute], struct_name: &Ident) -> Result<()> {
    let has_repr_c = attrs.iter().any(|attr| {
        if !attr.path().is_ident("repr") {
            return false;
        }
        // Parse the repr attribute to check for 'C'
        if let syn::Meta::List(meta_list) = &attr.meta {
            return meta_list.tokens.to_string().contains('C');
        }
        false
    });

    if !has_repr_c {
        return Err(syn::Error::new_spanned(
            struct_name,
            "Pod struct must have #[repr(C)] attribute for predictable field layout. \
             Add `#[repr(C)]` above your struct definition.",
        ));
    }
    Ok(())
}

/// Generates the PodCompressionInfoField trait implementation for Pod (zero-copy) structs.
///
/// Uses `core::mem::offset_of!()` for compile-time offset calculation.
/// This requires the struct to be `#[repr(C)]` for predictable field layout.
fn generate_pod_compression_info_impl(struct_name: &Ident) -> TokenStream {
    quote! {
        impl light_sdk::interface::PodCompressionInfoField for #struct_name {
            const COMPRESSION_INFO_OFFSET: usize = core::mem::offset_of!(#struct_name, compression_info);
        }
    }
}

/// Derives PodCompressionInfoField for a `#[repr(C)]` struct.
///
/// Requirements:
/// 1. Struct must have `#[repr(C)]` attribute
/// 2. Struct must have `compression_info: CompressionInfo` field (non-optional)
/// 3. Struct must implement `bytemuck::Pod` and `bytemuck::Zeroable`
///
/// # Example
///
/// ```ignore
/// use light_sdk_macros::PodCompressionInfoField;
/// use light_compressible::compression_info::CompressionInfo;
/// use bytemuck::{Pod, Zeroable};
///
/// #[derive(Pod, Zeroable, PodCompressionInfoField)]
/// #[repr(C)]
/// pub struct MyPodAccount {
///     pub owner: [u8; 32],
///     pub data: u64,
///     pub compression_info: CompressionInfo,
/// }
/// ```
pub fn derive_pod_compression_info_field(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;
    let fields = extract_fields_from_derive_input(&input)?;

    // Validate #[repr(C)] attribute
    validate_repr_c(&input.attrs, struct_name)?;

    // Validate compression_info field exists
    validate_pod_compression_info_field(fields, struct_name)?;

    // Generate trait implementation
    Ok(generate_pod_compression_info_impl(struct_name))
}

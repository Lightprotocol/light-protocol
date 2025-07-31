use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, ItemStruct, Result, Token,
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

    // Get all struct fields
    let struct_fields = match &input.fields {
        syn::Fields::Named(fields) => &fields.named,
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "CompressAs derive only supports structs with named fields",
            ));
        }
    };

    // Create field assignments for the compress_as method
    let field_assignments = struct_fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();

        // ALWAYS set compression_info to None - this is required for compressed storage
        if field_name == "compression_info" {
            return quote! { #field_name: None };
        }

        // Check if this field is overridden in the compress_as attribute
        let override_field = compress_as_fields
            .as_ref()
            .and_then(|fields| fields.fields.iter().find(|f| f.name == *field_name));

        if let Some(override_field) = override_field {
            let override_value = &override_field.value;
            quote! { #field_name: #override_value }
        } else {
            // Keep the original value - determine how to clone/copy based on field type
            let field_type = &field.ty;
            if is_copy_type(field_type) {
                quote! { #field_name: self.#field_name }
            } else {
                quote! { #field_name: self.#field_name.clone() }
            }
        }
    });

    // Determine if we need custom compression (any fields specified in compress_as attribute)
    let has_custom_fields = compress_as_fields.is_some();

    let compress_as_impl = if has_custom_fields {
        // Custom compression - return Cow::Owned with modified fields
        quote! {
            fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
                std::borrow::Cow::Owned(Self {
                    #(#field_assignments,)*
                })
            }
        }
    } else {
        // Simple case - return Cow::Owned with compression_info = None
        // We can't return Cow::Borrowed because compression_info must be None
        quote! {
            fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
                std::borrow::Cow::Owned(Self {
                    #(#field_assignments,)*
                })
            }
        }
    };

    let expanded = quote! {
        impl light_sdk::compressible::CompressAs for #struct_name {
            type Output = Self;

            #compress_as_impl
        }

        impl light_sdk::Size for #struct_name {
            fn size(&self) -> usize {
                Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE
            }
        }
    };

    Ok(expanded)
}

/// Determines if a type is likely to be Copy (simple heuristic)
fn is_copy_type(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                let type_name = segment.ident.to_string();
                matches!(
                    type_name.as_str(),
                    "u8" | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "usize"
                        | "i8"
                        | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "isize"
                        | "f32"
                        | "f64"
                        | "bool"
                        | "char"
                        | "Pubkey"
                ) || (type_name == "Option" && has_copy_inner_type(&segment.arguments))
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Check if Option<T> where T is Copy
fn has_copy_inner_type(args: &syn::PathArguments) -> bool {
    match args {
        syn::PathArguments::AngleBracketed(args) => args.args.iter().any(|arg| {
            if let syn::GenericArgument::Type(ty) = arg {
                is_copy_type(ty)
            } else {
                false
            }
        }),
        _ => false,
    }
}

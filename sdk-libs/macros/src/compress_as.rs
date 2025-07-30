use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, ItemStruct, Result, Token,
};

/// Parse the compressible_as attribute content
struct CompressibleAsFields {
    fields: Punctuated<CompressibleAsField, Token![,]>,
}

struct CompressibleAsField {
    name: Ident,
    value: Expr,
}

impl Parse for CompressibleAsField {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: Expr = input.parse()?;
        Ok(CompressibleAsField { name, value })
    }
}

impl Parse for CompressibleAsFields {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(CompressibleAsFields {
            fields: Punctuated::parse_terminated(input)?,
        })
    }
}

/// Generates CompressAs trait implementation for a struct with compressible_as attribute
pub fn derive_compress_as(input: ItemStruct) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Find the compressible_as attribute
    let compressible_as_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("compressible_as"))
        .ok_or_else(|| {
            syn::Error::new_spanned(
                &input,
                "CompressAs derive requires #[compressible_as(...)] attribute",
            )
        })?;

    // Parse the attribute content
    let compressible_fields: CompressibleAsFields = compressible_as_attr.parse_args()?;

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

        // Check if this field is overridden in the compressible_as attribute
        if let Some(override_field) = compressible_fields
            .fields
            .iter()
            .find(|f| f.name == *field_name)
        {
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

    let expanded = quote! {
        impl light_sdk::compressible::CompressAs for #struct_name {
            type Output = Self;

            fn compress_as(&self) -> Self::Output {
                Self {
                    #(#field_assignments,)*
                }
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

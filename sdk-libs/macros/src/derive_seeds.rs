use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Data, DeriveInput, Expr, Fields, Ident, LitStr, Result, Token,
};

/// Parse the seeds attribute content
struct SeedsAttribute {
    seeds: Punctuated<SeedElement, Token![,]>,
}

enum SeedElement {
    Literal(LitStr),
    Field(Ident),
    Expression(Expr),
}

impl Parse for SeedElement {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(LitStr) {
            Ok(SeedElement::Literal(input.parse()?))
        } else if input.peek(Ident) {
            Ok(SeedElement::Field(input.parse()?))
        } else {
            Ok(SeedElement::Expression(input.parse()?))
        }
    }
}

impl Parse for SeedsAttribute {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(SeedsAttribute {
            seeds: Punctuated::parse_terminated(input)?,
        })
    }
}

/// Generates seed getter functions for PDA and token accounts
///
/// Usage:
/// ```rust
/// #[derive(DeriveSeeds)]
/// #[seeds("user_record", owner)]
/// pub struct UserRecord {
///     pub owner: Pubkey,
///     // ...
/// }
///
/// #[derive(DeriveSeeds)]
/// #[seeds("ctoken_signer", user, mint)]
/// #[token_account]
/// pub struct CTokenSigner {
///     pub user: Pubkey,
///     pub mint: Pubkey,
/// }
/// ```
///
/// This generates:
/// - `get_user_record_seeds(owner: &Pubkey) -> (Vec<Vec<u8>>, Pubkey)`
/// - `get_c_token_signer_seeds(user: &Pubkey, mint: &Pubkey) -> (Vec<Vec<u8>>, Pubkey)`
pub fn derive_seeds(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Find the seeds attribute
    let seeds_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("seeds"))
        .ok_or_else(|| {
            syn::Error::new_spanned(
                &struct_name,
                "DeriveSeeds requires a #[seeds(...)] attribute",
            )
        })?;

    let seeds_content = seeds_attr.parse_args::<SeedsAttribute>()?;

    // Check if this is a token account
    let is_token_account = input
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("token_account"));

    // Get struct fields to determine parameters
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    &input,
                    "DeriveSeeds only supports structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "DeriveSeeds only supports structs",
            ));
        }
    };

    // Generate function name
    let fn_name = format_ident!(
        "get_{}_seeds",
        struct_name
            .to_string()
            .to_lowercase()
            .replace("record", "_record")
            .replace("session", "_session")
    );

    // Extract parameters from seeds that reference fields
    let mut parameters = Vec::new();
    let mut seed_expressions = Vec::new();

    for seed in &seeds_content.seeds {
        match seed {
            SeedElement::Literal(lit) => {
                let lit_value = lit.value();
                seed_expressions.push(quote! { #lit_value.as_bytes() });
            }
            SeedElement::Field(field_name) => {
                // Find the field type
                let field = fields
                    .iter()
                    .find(|f| f.ident.as_ref().map(|id| id == field_name).unwrap_or(false))
                    .ok_or_else(|| {
                        syn::Error::new_spanned(
                            field_name,
                            format!("Field '{}' not found in struct", field_name),
                        )
                    })?;

                let field_type = &field.ty;
                parameters.push(quote! { #field_name: &#field_type });

                // Handle different field types for seed generation
                if is_pubkey_type(field_type) {
                    seed_expressions.push(quote! { #field_name.as_ref() });
                } else if is_u64_type(field_type) {
                    seed_expressions.push(quote! { #field_name.to_le_bytes().as_ref() });
                } else {
                    return Err(syn::Error::new_spanned(
                        field_type,
                        format!(
                            "Unsupported field type for seeds: {}",
                            quote! { #field_type }
                        ),
                    ));
                }
            }
            SeedElement::Expression(expr) => {
                seed_expressions.push(quote! { #expr });
            }
        }
    }

    // Generate the function - simplified approach matching the original manual implementation
    let function_impl = quote! {
        /// Auto-generated seed function for PDA account
        pub fn #fn_name(#(#parameters),*) -> (Vec<Vec<u8>>, anchor_lang::prelude::Pubkey) {
            let seeds = [#(#seed_expressions),*];
            let (pda, bump) = anchor_lang::prelude::Pubkey::find_program_address(&seeds, &crate::ID);
            let bump_slice = vec![bump];
            let seeds_vec = vec![
                #(
                    (#seed_expressions).to_vec(),
                )*
                bump_slice,
            ];
            (seeds_vec, pda)
        }
    };

    Ok(function_impl)
}

/// Check if a type is Pubkey
fn is_pubkey_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            segment.ident == "Pubkey"
        } else {
            false
        }
    } else {
        false
    }
}

/// Check if a type is u64
fn is_u64_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            segment.ident == "u64"
        } else {
            false
        }
    } else {
        false
    }
}

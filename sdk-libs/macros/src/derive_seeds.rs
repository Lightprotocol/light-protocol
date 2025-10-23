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

#[allow(clippy::large_enum_variant)]
enum SeedElement {
    Literal(LitStr),
    Field(Ident),
    Expression(Box<Expr>),
}

impl Parse for SeedElement {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(LitStr) {
            Ok(SeedElement::Literal(input.parse()?))
        } else if input.peek(Ident) {
            Ok(SeedElement::Field(input.parse()?))
        } else {
            Ok(SeedElement::Expression(Box::new(input.parse()?)))
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
                struct_name,
                "DeriveSeeds requires a #[seeds(...)] attribute",
            )
        })?;

    let seeds_content = seeds_attr.parse_args::<SeedsAttribute>()?;

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
            .trim_end_matches("seeds")
            .replace("record", "_record")
            .replace("session", "_session")
            .replace("signer", "_signer")
    );

    // Extract parameters and generate bindings for temporaries
    // We need TWO sets: one for trait impl (uses self.field), one for client function (uses parameters)
    let mut client_parameters = Vec::new();
    let mut client_bindings = Vec::new();
    let mut client_seed_refs = Vec::new();

    let mut trait_bindings = Vec::new();
    let mut trait_seed_refs = Vec::new();

    for (i, seed) in seeds_content.seeds.iter().enumerate() {
        match seed {
            SeedElement::Literal(lit) => {
                let lit_value = lit.value();
                client_seed_refs.push(quote! { #lit_value.as_bytes() });
                trait_seed_refs.push(quote! { #lit_value.as_bytes() });
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
                client_parameters.push(quote! { #field_name: &#field_type });

                // Handle different field types for seed generation
                if is_pubkey_type(field_type) {
                    // Client function: parameter reference
                    client_seed_refs.push(quote! { #field_name.as_ref() });
                    // Trait impl: self.field reference
                    trait_seed_refs.push(quote! { self.#field_name.as_ref() });
                } else if is_u64_type(field_type) {
                    // Client function: bind temporary from parameter
                    let client_binding_name = format_ident!("_seed_{}", i);
                    client_bindings
                        .push(quote! { let #client_binding_name = #field_name.to_le_bytes(); });
                    client_seed_refs.push(quote! { #client_binding_name.as_ref() });

                    // Trait impl: bind temporary from self.field
                    let trait_binding_name = format_ident!("_seed_{}", i);
                    trait_bindings
                        .push(quote! { let #trait_binding_name = self.#field_name.to_le_bytes(); });
                    trait_seed_refs.push(quote! { #trait_binding_name.as_ref() });
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
                client_seed_refs.push(quote! { #expr });
                trait_seed_refs.push(quote! { #expr });
            }
        }
    }

    // Generate PdaSeedProvider trait implementation (uses self.field)
    let pda_seed_provider_impl = quote! {
        impl light_sdk::compressible::PdaSeedProvider for #struct_name {
            fn derive_pda_seeds(&self, program_id: &anchor_lang::prelude::Pubkey) -> (Vec<Vec<u8>>, anchor_lang::prelude::Pubkey) {
                #(#trait_bindings)*
                let seeds: &[&[u8]] = &[#(#trait_seed_refs),*];
                let (pda, bump) = anchor_lang::prelude::Pubkey::find_program_address(seeds, program_id);
                let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
                for seed in seeds {
                    seeds_vec.push(seed.to_vec());
                }
                seeds_vec.push(vec![bump]);
                (seeds_vec, pda)
            }
        }
    };

    // Generate client-side seed function (uses parameters)
    let client_function = quote! {
        /// Auto-generated client seed function
        pub fn #fn_name(#(#client_parameters),*) -> (Vec<Vec<u8>>, anchor_lang::prelude::Pubkey) {
            #(#client_bindings)*
            let seeds: &[&[u8]] = &[#(#client_seed_refs),*];
            let (pda, bump) = anchor_lang::prelude::Pubkey::find_program_address(seeds, &crate::ID);
            let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
            for seed in seeds {
                seeds_vec.push(seed.to_vec());
            }
            seeds_vec.push(vec![bump]);
            (seeds_vec, pda)
        }
    };

    Ok(quote! {
        #pda_seed_provider_impl
        #client_function
    })
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

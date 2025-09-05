use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Data, DeriveInput, Expr, Ident, LitStr, Result, Token,
};

/// Parse seed specification for a token account variant
struct TokenSeedSpec {
    variant: Ident,
    _eq: Token![=],
    seeds: Punctuated<SeedElement, Token![,]>,
}

impl Parse for TokenSeedSpec {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(TokenSeedSpec {
            variant: input.parse()?,
            _eq: input.parse()?,
            seeds: {
                let content;
                syn::parenthesized!(content in input);
                Punctuated::parse_terminated(&content)?
            },
        })
    }
}

/// Parse the entire token_seeds attribute content
struct TokenSeedsAttribute {
    specs: Punctuated<TokenSeedSpec, Token![,]>,
}

impl Parse for TokenSeedsAttribute {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(TokenSeedsAttribute {
            specs: Punctuated::parse_terminated(input)?,
        })
    }
}

enum SeedElement {
    /// String literal like "user_record"
    Literal(LitStr),
    /// Context field access like ctx.fee_payer, ctx.mint, ctx.owner
    ContextField(Ident),
    /// Account field access like ctx.accounts.some_field
    AccountField(Ident, Ident), // ctx.accounts, field_name
    /// Expression like some_id.to_le_bytes()
    Expression(Expr),
}

impl Parse for SeedElement {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(LitStr) {
            Ok(SeedElement::Literal(input.parse()?))
        } else if input.peek(Ident) {
            let first_ident: Ident = input.parse()?;

            // Check if it's ctx.accounts.field or ctx.field
            if first_ident == "ctx" && input.peek(Token![.]) {
                let _dot: Token![.] = input.parse()?;
                let second_ident: Ident = input.parse()?;

                if second_ident == "accounts" && input.peek(Token![.]) {
                    let _dot2: Token![.] = input.parse()?;
                    let field_name: Ident = input.parse()?;
                    Ok(SeedElement::AccountField(second_ident, field_name))
                } else {
                    Ok(SeedElement::ContextField(second_ident))
                }
            } else {
                // Parse as expression
                let expr = syn::Expr::Path(syn::ExprPath {
                    attrs: vec![],
                    qself: None,
                    path: syn::Path::from(first_ident),
                });
                Ok(SeedElement::Expression(expr))
            }
        } else {
            Ok(SeedElement::Expression(input.parse()?))
        }
    }
}

/// Derives CTokenSeedProvider implementation for an enum
///
/// Usage:
/// ```rust
/// #[derive(DeriveCTokenSeeds)]
/// #[token_seeds(
///     CTokenSigner = ("ctoken_signer", ctx.fee_payer, ctx.mint),
///     UserVault = ("user_vault", ctx.accounts.user, ctx.mint),
///     CustomVault = ("custom", ctx.accounts.custom_seed, some_id.to_le_bytes())
/// )]
/// pub enum CTokenAccountVariant {
///     CTokenSigner,
///     UserVault,
///     CustomVault,
///     AssociatedTokenAccount, // Can be left without seeds if not implemented
/// }
/// ```
///
/// This generates an implementation of `ctoken_seed_system::CTokenSeedProvider` that:
/// - Matches on each variant
/// - Calls the appropriate seed function based on the specification
/// - Has access to ctx.fee_payer, ctx.mint, ctx.owner, and ctx.accounts.* fields
/// - Returns unreachable!() for variants without seed specifications
pub fn derive_ctoken_seeds(input: DeriveInput) -> Result<TokenStream> {
    let enum_name = &input.ident;

    // Find the token_seeds attribute
    let token_seeds_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("token_seeds"))
        .ok_or_else(|| {
            syn::Error::new_spanned(
                &enum_name,
                "DeriveCTokenSeeds requires a #[token_seeds(...)] attribute",
            )
        })?;

    let token_seeds_content = token_seeds_attr.parse_args::<TokenSeedsAttribute>()?;

    // Get enum variants
    let variants = match &input.data {
        Data::Enum(data) => &data.variants,
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "DeriveCTokenSeeds only supports enums",
            ));
        }
    };

    // Generate match arms
    let mut match_arms = Vec::new();

    for variant in variants {
        let variant_name = &variant.ident;

        // Find seed specification for this variant
        if let Some(spec) = token_seeds_content
            .specs
            .iter()
            .find(|s| s.variant == *variant_name)
        {
            let seed_expressions = generate_seed_expressions(&spec.seeds)?;

            let match_arm = quote! {
                #enum_name::#variant_name => {
                    let seeds = [#(#seed_expressions),*];
                    let (pda, bump) = anchor_lang::prelude::Pubkey::find_program_address(&seeds, &crate::ID);
                    let seeds_vec = vec![
                        #(
                            (#seed_expressions).to_vec(),
                        )*
                        vec![bump],
                    ];
                    (seeds_vec, pda)
                }
            };
            match_arms.push(match_arm);
        } else {
            // Generate unreachable for variants without seed specs
            let match_arm = quote! {
                #enum_name::#variant_name => {
                    unreachable!("Seed specification not provided for variant {}", stringify!(#variant_name))
                }
            };
            match_arms.push(match_arm);
        }
    }

    // Generate the trait implementation
    let implementation = quote! {
        impl ctoken_seed_system::CTokenSeedProvider for #enum_name {
            fn get_seeds<'a, 'info>(
                &self,
                ctx: &ctoken_seed_system::CTokenSeedContext<'a, 'info>,
            ) -> (Vec<Vec<u8>>, anchor_lang::prelude::Pubkey) {
                match self {
                    #(#match_arms)*
                }
            }
        }
    };

    Ok(implementation)
}

/// Generate seed expressions from SeedElement specifications
fn generate_seed_expressions(
    seeds: &Punctuated<SeedElement, Token![,]>,
) -> Result<Vec<TokenStream>> {
    let mut expressions = Vec::new();

    for seed in seeds {
        let expr = match seed {
            SeedElement::Literal(lit) => {
                let value = lit.value();
                quote! { #value.as_bytes() }
            }
            SeedElement::ContextField(field) => match field.to_string().as_str() {
                "fee_payer" => quote! { ctx.fee_payer.as_ref() },
                "mint" => quote! { ctx.mint.as_ref() },
                "owner" => quote! { ctx.owner.as_ref() },
                _ => {
                    return Err(syn::Error::new_spanned(
                        field,
                        format!(
                            "Unknown context field: {}. Available: fee_payer, mint, owner",
                            field
                        ),
                    ));
                }
            },
            SeedElement::AccountField(_accounts, field_name) => {
                quote! { ctx.accounts.#field_name.key().as_ref() }
            }
            SeedElement::Expression(expr) => {
                quote! { (#expr).as_ref() }
            }
        };
        expressions.push(expr);
    }

    Ok(expressions)
}

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, ItemMod, LitStr, Result, Token,
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

/// Parse token seeds specification
struct TokenSeedsArgs {
    specs: Punctuated<TokenSeedSpec, Token![,]>,
}

impl Parse for TokenSeedsArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(TokenSeedsArgs {
            specs: Punctuated::parse_terminated(input)?,
        })
    }
}

/// Generate CTokenSeedProvider implementation
///
/// Usage:
/// ```rust
/// #[ctoken_seeds(CTokenSigner = ("ctoken_signer", ctx.fee_payer, ctx.mint))]
/// #[add_compressible_instructions_enhanced(UserRecord, GameSession)]
/// #[program]
/// pub mod my_program {
///     // Your instructions...
/// }
/// ```
pub fn ctoken_seeds(args: TokenStream, input: ItemMod) -> Result<TokenStream> {
    let token_seeds = syn::parse2::<TokenSeedsArgs>(args)?;

    // Generate the CTokenSeedProvider implementation
    let ctoken_implementation = generate_ctoken_seed_provider_implementation(&token_seeds.specs)?;

    Ok(quote! {
        // Generate the CTokenSeedProvider implementation
        #ctoken_implementation

        // Pass through the original module unchanged
        #input
    })
}

/// Generate CTokenSeedProvider implementation from token seed specifications
fn generate_ctoken_seed_provider_implementation(
    token_seeds: &Punctuated<TokenSeedSpec, Token![,]>,
) -> Result<TokenStream> {
    let mut match_arms = Vec::new();

    for spec in token_seeds {
        let variant_name = &spec.variant;
        let seed_expressions = generate_seed_expressions(&spec.seeds)?;

        let match_arm = quote! {
            CTokenAccountVariant::#variant_name => {
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
    }

    Ok(quote! {
        /// Auto-generated CTokenSeedProvider implementation
        impl ctoken_seed_system::CTokenSeedProvider for CTokenAccountVariant {
            fn get_seeds<'a, 'info>(
                &self,
                ctx: &ctoken_seed_system::CTokenSeedContext<'a, 'info>,
            ) -> (Vec<Vec<u8>>, anchor_lang::prelude::Pubkey) {
                match self {
                    #(#match_arms)*
                    _ => {
                        unreachable!("CToken variant not configured with seeds")
                    }
                }
            }
        }
    })
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

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, LitInt, Result, Token,
};

/// Parse CToken variant definitions with their seed function mappings
struct CTokenVariantMapping {
    variant: Ident,
    discriminator: LitInt,
    seed_function: Ident,
    seed_params: Vec<Expr>,
}

struct CTokenVariantList {
    variants: Punctuated<CTokenVariantMapping, Token![,]>,
}

impl Parse for CTokenVariantMapping {
    fn parse(input: ParseStream) -> Result<Self> {
        let variant: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let discriminator: LitInt = input.parse()?;
        input.parse::<Token![=>]>()?;
        let seed_function: Ident = input.parse()?;

        let seed_params = if input.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            let params: Punctuated<Expr, Token![,]> = content.parse_terminated(Expr::parse)?;
            params.into_iter().collect()
        } else {
            Vec::new()
        };

        Ok(CTokenVariantMapping {
            variant,
            discriminator,
            seed_function,
            seed_params,
        })
    }
}

impl Parse for CTokenVariantList {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(CTokenVariantList {
            variants: Punctuated::parse_terminated(input)?,
        })
    }
}

/// Generates automatic CToken variant dispatch based on user-defined mappings
///
/// Usage:
/// ```rust
/// generate_ctoken_dispatch! {
///     CTokenSigner = 0 => get_ctoken_signer_seeds(fee_payer, mint),
///     AssociatedTokenAccount = 255 => get_associated_token_account_seeds(owner, mint),
///     CustomTokenAccount = 42 => get_custom_token_account_seeds(user, mint, custom_param),
/// }
/// ```
pub fn generate_ctoken_dispatch(input: TokenStream) -> Result<TokenStream> {
    let variant_list = syn::parse2::<CTokenVariantList>(input)?;

    let match_arms = variant_list.variants.iter().map(|mapping| {
        let variant = &mapping.variant;
        let seed_function = &mapping.seed_function;
        let seed_params = &mapping.seed_params;

        quote! {
            CTokenAccountVariant::#variant => {
                #seed_function(#(#seed_params),*).0
            }
        }
    });

    Ok(quote! {
        match token_data.variant {
            #(#match_arms)*
        }
    })
}

/// Alternative approach: Generate a trait-based system for complete automation
pub fn generate_ctoken_seed_trait_system() -> TokenStream {
    quote! {
        /// Trait that CToken variants can implement to provide their seed derivation
        pub trait CTokenSeedProvider {
            fn get_seeds(&self, ctx: &CTokenSeedContext) -> (Vec<Vec<u8>>, Pubkey);
        }

        /// Context struct that provides all available parameters for seed derivation
        pub struct CTokenSeedContext<'a> {
            pub fee_payer: &'a Pubkey,
            pub owner: &'a Pubkey,
            pub mint: &'a Pubkey,
            // Add more parameters as needed
        }

        /// Automatic implementation for the CTokenAccountVariant enum
        impl CTokenSeedProvider for CTokenAccountVariant {
            fn get_seeds(&self, ctx: &CTokenSeedContext) -> (Vec<Vec<u8>>, Pubkey) {
                match self {
                    CTokenAccountVariant::CTokenSigner => {
                        get_ctoken_signer_seeds(ctx.fee_payer, ctx.mint)
                    }
                    CTokenAccountVariant::AssociatedTokenAccount => {
                        // Would call get_associated_token_account_seeds when implemented
                        unreachable!("AssociatedTokenAccount not implemented")
                    }
                    // Additional variants automatically handled when trait is implemented
                }
            }
        }
    }
}

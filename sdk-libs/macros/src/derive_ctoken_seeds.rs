use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Data, DeriveInput, Ident, LitStr, Result, Token,
};

/// Parse the ctoken_seeds attribute content
struct CTokenSeedsAttribute {
    seeds: Punctuated<SeedElement, Token![,]>,
    authority: Option<Vec<SeedElement>>,
}

enum SeedElement {
    Literal(LitStr),
    Field(Ident),
}

impl Parse for SeedElement {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(LitStr) {
            Ok(SeedElement::Literal(input.parse()?))
        } else {
            Ok(SeedElement::Field(input.parse()?))
        }
    }
}

impl Parse for CTokenSeedsAttribute {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut seeds = Punctuated::new();
        let mut authority = None;

        while !input.is_empty() {
            // Check for "authority = (...)" pattern
            if input.peek(Ident) {
                let fork = input.fork();
                if let Ok(ident) = fork.parse::<Ident>() {
                    if ident == "authority" && fork.peek(Token![=]) {
                        // Found authority assignment
                        let _: Ident = input.parse()?;
                        let _: Token![=] = input.parse()?;

                        let auth_content;
                        syn::parenthesized!(auth_content in input);
                        let mut auth_seeds = Vec::new();

                        while !auth_content.is_empty() {
                            auth_seeds.push(auth_content.parse::<SeedElement>()?);
                            if auth_content.peek(Token![,]) {
                                let _: Token![,] = auth_content.parse()?;
                            } else {
                                break;
                            }
                        }
                        authority = Some(auth_seeds);

                        if input.peek(Token![,]) {
                            let _: Token![,] = input.parse()?;
                        }
                        continue;
                    }
                }
            }

            // Regular seed element
            seeds.push(input.parse::<SeedElement>()?);

            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            } else {
                break;
            }
        }

        Ok(CTokenSeedsAttribute { seeds, authority })
    }
}

/// Generates CTokenSeedProvider trait implementation for token account seed derivation
///
/// Usage on enum variant types:
/// ```rust
/// #[derive(DeriveCTokenSeeds)]
/// #[ctoken_seeds("ctoken_signer", fee_payer, mint, authority = (cpi_authority))]
/// #[repr(u8)]
/// pub enum CTokenAccountVariant {
///     CTokenSigner = 0,
/// }
/// ```
///
/// This generates the CTokenSeedProvider trait impl with get_seeds and get_authority_seeds methods.
pub fn derive_ctoken_seeds(input: DeriveInput) -> Result<TokenStream> {
    let enum_name = &input.ident;

    // This should be an enum
    let variants = match &input.data {
        Data::Enum(data) => &data.variants,
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "DeriveCTokenSeeds only supports enums",
            ));
        }
    };

    // Find the ctoken_seeds attribute
    let ctoken_seeds_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("ctoken_seeds"))
        .ok_or_else(|| {
            syn::Error::new_spanned(
                enum_name,
                "DeriveCTokenSeeds requires a #[ctoken_seeds(...)] attribute",
            )
        })?;

    let seeds_content = ctoken_seeds_attr.parse_args::<CTokenSeedsAttribute>()?;

    // For now, we support single-variant enums
    // Multi-variant would need per-variant seed specifications
    if variants.len() != 1 {
        return Err(syn::Error::new_spanned(
            enum_name,
            "DeriveCTokenSeeds currently only supports single-variant enums. For multi-variant, specify seeds per variant or use manual implementation.",
        ));
    }

    let variant_name = &variants.first().unwrap().ident;

    // Generate seed derivation code for get_seeds
    let mut seed_bindings = Vec::new();
    let mut seed_refs = Vec::new();

    for (i, seed) in seeds_content.seeds.iter().enumerate() {
        match seed {
            SeedElement::Literal(lit) => {
                let value = lit.value();
                seed_refs.push(quote! { #value.as_bytes() });
            }
            SeedElement::Field(field_name) => {
                // Assume these are accessed from accounts struct
                let binding_name = format_ident!("_seed_{}", i);
                seed_bindings.push(quote! {
                    let #binding_name: [u8; 32] = accounts.#field_name.key.to_bytes();
                });
                seed_refs.push(quote! { &#binding_name });
            }
        }
    }

    // Generate authority seed derivation
    let authority_impl = if let Some(authority_seeds) = &seeds_content.authority {
        let mut auth_bindings = Vec::new();
        let mut auth_refs = Vec::new();

        for (i, seed) in authority_seeds.iter().enumerate() {
            match seed {
                SeedElement::Literal(lit) => {
                    let value = lit.value();
                    auth_refs.push(quote! { #value.as_bytes() });
                }
                // TODO: check why unused param.
                SeedElement::Field(_field_name) => {
                    let binding_name = format_ident!("_auth_seed_{}", i);
                    auth_bindings.push(quote! {
                        let #binding_name = crate::LIGHT_CPI_SIGNER.cpi_signer;
                    });
                    auth_refs.push(quote! { &#binding_name });
                }
            }
        }

        quote! {
            #(#auth_bindings)*
            let seeds: &[&[u8]] = &[#(#auth_refs),*];
            let (pda, bump) = anchor_lang::prelude::Pubkey::find_program_address(seeds, &crate::ID);
            let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
            seeds_vec.extend(seeds.iter().map(|s| s.to_vec()));
            seeds_vec.push(vec![bump]);
            Ok((seeds_vec, pda))
        }
    } else {
        quote! {
            Err(anchor_lang::prelude::ProgramError::InvalidAccountData)
        }
    };

    let impl_code = quote! {
        impl light_compressed_token_sdk::CTokenSeedProvider for #enum_name {
            type Accounts<'info> = crate::instruction_accounts::DecompressAccountsIdempotent<'info>;

            fn get_seeds<'a, 'info>(
                &self,
                accounts: &'a Self::Accounts<'info>,
                _remaining_accounts: &'a [anchor_lang::prelude::AccountInfo<'info>],
            ) -> std::result::Result<(Vec<Vec<u8>>, anchor_lang::prelude::Pubkey), anchor_lang::prelude::ProgramError> {
                match self {
                    #enum_name::#variant_name => {
                        #(#seed_bindings)*
                        let seeds: &[&[u8]] = &[#(#seed_refs),*];
                        let (pda, bump) = anchor_lang::prelude::Pubkey::find_program_address(seeds, &crate::ID);
                        let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
                        seeds_vec.extend(seeds.iter().map(|s| s.to_vec()));
                        seeds_vec.push(vec![bump]);
                        Ok((seeds_vec, pda))
                    }
                }
            }

            fn get_authority_seeds<'a, 'info>(
                &self,
                _accounts: &'a Self::Accounts<'info>,
                _remaining_accounts: &'a [anchor_lang::prelude::AccountInfo<'info>],
            ) -> std::result::Result<(Vec<Vec<u8>>, anchor_lang::prelude::Pubkey), anchor_lang::prelude::ProgramError> {
                match self {
                    #enum_name::#variant_name => {
                        #authority_impl
                    }
                }
            }
        }
    };

    Ok(impl_code)
}

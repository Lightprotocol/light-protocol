use light_sdk_types::constants::RENT_SPONSOR_SEED;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse_macro_input, punctuated::Punctuated, Expr, LitStr, Token};

struct Args {
    program_id: LitStr,
}
impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let elems = Punctuated::<Expr, Token![,]>::parse_terminated(input)?;
        if elems.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "Expected a program id string literal",
            ));
        }
        // First argument must be a string literal
        let program_id = match &elems[0] {
            Expr::Lit(expr_lit) => {
                if let syn::Lit::Str(ls) = &expr_lit.lit {
                    ls.clone()
                } else {
                    return Err(syn::Error::new_spanned(
                        &elems[0],
                        "Argument must be a string literal program id",
                    ));
                }
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    &elems[0],
                    "Argument must be a string literal program id",
                ))
            }
        };
        // Ignore any additional arguments for backwards compatibility
        Ok(Args { program_id })
    }
}

/// Derives a Rent Sponsor PDA for a program at compile time.
///
/// Seeds: ["rent_sponsor"]
///
/// Usage:
///   const DATA: ([u8; 32], u8) = derive_light_rent_sponsor_pda!("Program1111111111111111111111111111111111");
pub fn derive_light_rent_sponsor_pda(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as Args);
    let program_id_str = args.program_id.value();

    // Parse program ID at compile time
    use std::str::FromStr;
    let program_id = match solana_pubkey::Pubkey::from_str(&program_id_str) {
        Ok(id) => id,
        Err(_) => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                "Invalid program ID format. Expected a base58 encoded public key",
            )
            .to_compile_error()
            .into();
        }
    };

    let seeds = &[RENT_SPONSOR_SEED];
    let (pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, &program_id);

    let pda_bytes = pda.to_bytes();
    let bytes = pda_bytes
        .iter()
        .map(|b| proc_macro2::Literal::u8_unsuffixed(*b));

    let output = quote! {
        ([#(#bytes),*], #bump)
    };
    output.into()
}

/// Derives a Rent Sponsor configuration struct at compile time.
///
/// Returns `::light_sdk::sdk_types::RentSponsor { program_id, rent_sponsor, bump }`.
///
/// Usage:
///   const RENT_SPONSOR: ::light_sdk::sdk_types::RentSponsor =
///       derive_light_rent_sponsor!("Program1111111111111111111111111111111111");
pub fn derive_light_rent_sponsor(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as Args);
    let program_id_str = args.program_id.value();

    // Parse program ID at compile time
    use std::str::FromStr;
    let program_id = match solana_pubkey::Pubkey::from_str(&program_id_str) {
        Ok(id) => id,
        Err(_) => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                "Invalid program ID format. Expected a base58 encoded public key",
            )
            .to_compile_error()
            .into();
        }
    };

    let seeds = &[RENT_SPONSOR_SEED];
    let (pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, &program_id);

    let program_id_bytes = program_id.to_bytes();
    let pda_bytes = pda.to_bytes();

    let program_id_literals = program_id_bytes
        .iter()
        .map(|b| proc_macro2::Literal::u8_unsuffixed(*b));
    let pda_literals = pda_bytes
        .iter()
        .map(|b| proc_macro2::Literal::u8_unsuffixed(*b));

    let output = quote! {
        {
            ::light_sdk::sdk_types::RentSponsor {
                program_id: [#(#program_id_literals),*],
                rent_sponsor: [#(#pda_literals),*],
                bump: #bump,
            }
        }
    };
    output.into()
}

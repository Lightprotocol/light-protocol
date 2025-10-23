use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse_macro_input, punctuated::Punctuated, Expr, LitInt, LitStr, Token};

struct Args {
    program_id: LitStr,
    version: Option<LitInt>,
}
impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let elems = Punctuated::<Expr, Token![,]>::parse_terminated(input)?;
        if elems.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "Expected at least a program id string literal",
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
                        "First argument must be a string literal program id",
                    ));
                }
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    &elems[0],
                    "First argument must be a string literal program id",
                ))
            }
        };
        // Optional second argument: version as integer literal (u16)
        let version = if elems.len() > 1 {
            match &elems[1] {
                Expr::Lit(expr_lit) => {
                    if let syn::Lit::Int(li) = &expr_lit.lit {
                        Some(li.clone())
                    } else {
                        return Err(syn::Error::new_spanned(
                            &elems[1],
                            "Second argument must be an integer literal (u16 version)",
                        ));
                    }
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        &elems[1],
                        "Second argument must be an integer literal (u16 version)",
                    ))
                }
            }
        } else {
            None
        };
        Ok(Args {
            program_id,
            version,
        })
    }
}

/// Derives a Rent Sponsor PDA for a program at compile time.
///
/// Seeds: ["rent_sponsor", <u16 version little-endian>]
///
/// Usage:
/// - With default version=1:
///   const DATA: ([u8; 32], u8) = derive_light_rent_sponsor_pda!("Program1111111111111111111111111111111111");
/// - With explicit version:
///   const DATA: ([u8; 32], u8) = derive_light_rent_sponsor_pda!("Program1111111111111111111111111111111111", 2);
pub fn derive_light_rent_sponsor_pda(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as Args);
    let program_id_str = args.program_id.value();
    let version_u16: u16 = match args.version.as_ref() {
        Some(lit) => lit.base10_parse::<u16>().unwrap_or(1u16),
        None => 1u16,
    };

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

    let seeds = &[b"rent_sponsor".as_slice(), &version_u16.to_le_bytes()[..]];
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
/// Returns `::light_sdk_types::RentSponsor { program_id, rent_sponsor, bump, version }`.
///
/// Usage:
///   const RENT_SPONSOR: ::light_sdk_types::RentSponsor =
///       derive_light_rent_sponsor!("Program1111111111111111111111111111111111", 1);
pub fn derive_light_rent_sponsor(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as Args);
    let program_id_str = args.program_id.value();
    let version_u16: u16 = match args.version.as_ref() {
        Some(lit) => lit.base10_parse::<u16>().unwrap_or(1u16),
        None => 1u16,
    };

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

    let seeds = &[b"rent_sponsor".as_slice(), &version_u16.to_le_bytes()[..]];
    let (pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, &program_id);

    let program_id_bytes = program_id.to_bytes();
    let pda_bytes = pda.to_bytes();

    let program_id_literals = program_id_bytes
        .iter()
        .map(|b| proc_macro2::Literal::u8_unsuffixed(*b));
    let pda_literals = pda_bytes
        .iter()
        .map(|b| proc_macro2::Literal::u8_unsuffixed(*b));

    let version_lit = proc_macro2::Literal::u16_unsuffixed(version_u16);
    let output = quote! {
        {
            ::light_sdk_types::RentSponsor {
                program_id: [#(#program_id_literals),*],
                rent_sponsor: [#(#pda_literals),*],
                bump: #bump,
                version: #version_lit,
            }
        }
    };
    output.into()
}

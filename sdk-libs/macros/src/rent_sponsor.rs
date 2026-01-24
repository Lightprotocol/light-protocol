use light_sdk_types::constants::RENT_SPONSOR_SEED;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse_macro_input, LitStr};

struct ProgramIdArg {
    program_id: LitStr,
}

impl Parse for ProgramIdArg {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let program_id: LitStr = input.parse()?;
        Ok(ProgramIdArg { program_id })
    }
}

/// Derives 4 Rent Sponsor PDAs (versions 1-4) at compile time.
///
/// Returns a `RentSponsors` struct containing an array of 4 `RentSponsor` entries.
/// Version 1 is always the default, accessed via `.default()` or index `[0]`.
///
/// Usage:
/// ```ignore
/// use light_sdk_macros::derive_light_rent_sponsors;
///
/// pub const RENT_SPONSORS: ::light_sdk::sdk_types::RentSponsors =
///     derive_light_rent_sponsors!("Program1111111111111111111111111111111111");
///
/// // Get default (version 1)
/// let default_sponsor = RENT_SPONSORS.default();
///
/// // Get specific version (1-indexed)
/// let v2_sponsor = RENT_SPONSORS.get(2).unwrap();
/// ```
pub fn derive_light_rent_sponsors(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as ProgramIdArg);
    let program_id_str = args.program_id.value();

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

    let program_id_bytes = program_id.to_bytes();
    let program_id_literals: Vec<_> = program_id_bytes
        .iter()
        .map(|b| proc_macro2::Literal::u8_unsuffixed(*b))
        .collect();

    let mut sponsor_entries = Vec::with_capacity(4);
    for version in 1u16..=4u16 {
        let seeds = &[RENT_SPONSOR_SEED, &version.to_le_bytes()[..]];
        let (pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, &program_id);
        let pda_bytes = pda.to_bytes();
        let pda_literals: Vec<_> = pda_bytes
            .iter()
            .map(|b| proc_macro2::Literal::u8_unsuffixed(*b))
            .collect();
        let version_lit = proc_macro2::Literal::u16_unsuffixed(version);
        let prog_lits = &program_id_literals;

        sponsor_entries.push(quote! {
            ::light_sdk::sdk_types::RentSponsor {
                program_id: [#(#prog_lits),*],
                rent_sponsor: [#(#pda_literals),*],
                bump: #bump,
                version: #version_lit,
            }
        });
    }

    let output = quote! {
        ::light_sdk::sdk_types::RentSponsors {
            sponsors: [#(#sponsor_entries),*],
        }
    };
    output.into()
}

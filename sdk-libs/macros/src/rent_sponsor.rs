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

/// Derives the Rent Sponsor PDA at compile time (version 1, hardcoded).
///
/// Returns a `RentSponsor` struct with the PDA address and bump.
///
/// Usage:
/// ```ignore
/// use light_sdk_macros::derive_light_rent_sponsor;
///
/// pub const RENT_SPONSOR: ::light_sdk::sdk_types::RentSponsor =
///     derive_light_rent_sponsor!("Program1111111111111111111111111111111111");
///
/// // Access the pubkey
/// let pubkey = Pubkey::from(RENT_SPONSOR.rent_sponsor);
/// // Access the bump for signing
/// let bump = RENT_SPONSOR.bump;
/// ```
pub fn derive_light_rent_sponsor(input: TokenStream) -> TokenStream {
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

    // Always version 1
    const VERSION: u16 = 1;
    let seeds = &[RENT_SPONSOR_SEED, &VERSION.to_le_bytes()[..]];
    let (pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, &program_id);
    let pda_bytes = pda.to_bytes();
    let pda_literals: Vec<_> = pda_bytes
        .iter()
        .map(|b| proc_macro2::Literal::u8_unsuffixed(*b))
        .collect();

    let output = quote! {
        ::light_sdk::sdk_types::RentSponsor {
            program_id: [#(#program_id_literals),*],
            rent_sponsor: [#(#pda_literals),*],
            bump: #bump,
        }
    };
    output.into()
}

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

pub fn derive_light_cpi_signer_pda(input: TokenStream) -> TokenStream {
    // Parse the input - just a program ID string literal
    let program_id_lit = parse_macro_input!(input as LitStr);
    let program_id_str = program_id_lit.value();

    // Compute the PDA at compile time using solana-pubkey with "cpi_authority" seed
    use std::str::FromStr;

    // Parse program ID at compile time
    let program_id = match solana_pubkey::Pubkey::from_str(&program_id_str) {
        Ok(id) => id,
        Err(_) => {
            return syn::Error::new(
                program_id_lit.span(),
                "Invalid program ID format. Expected a base58 encoded public key",
            )
            .to_compile_error()
            .into();
        }
    };

    // Use fixed "cpi_authority" seed
    let seeds = &[b"cpi_authority".as_slice()];

    // Compute the PDA at compile time
    let (pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, &program_id);

    // Generate the output code with precomputed byte array and bump
    let pda_bytes = pda.to_bytes();
    let bytes = pda_bytes
        .iter()
        .map(|b| proc_macro2::Literal::u8_unsuffixed(*b));

    let output = quote! {
        ([#(#bytes),*], #bump)
    };

    output.into()
}

pub fn derive_light_cpi_signer(input: TokenStream) -> TokenStream {
    // Parse the input - just a program ID string literal
    let program_id_lit = parse_macro_input!(input as LitStr);
    let program_id_str = program_id_lit.value();

    // Compute the PDA at compile time using solana-pubkey with "cpi_authority" seed
    use std::str::FromStr;

    // Parse program ID at compile time
    let program_id = match solana_pubkey::Pubkey::from_str(&program_id_str) {
        Ok(id) => id,
        Err(_) => {
            return syn::Error::new(
                program_id_lit.span(),
                "Invalid program ID format. Expected a base58 encoded public key",
            )
            .to_compile_error()
            .into();
        }
    };

    // Use fixed "cpi_authority" seed
    let seeds = &[b"cpi_authority".as_slice()];

    // Compute the PDA at compile time
    let (pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, &program_id);

    // Generate the output code with precomputed CpiSigner struct
    let program_id_bytes = program_id.to_bytes();
    let pda_bytes = pda.to_bytes();

    let program_id_literals = program_id_bytes
        .iter()
        .map(|b| proc_macro2::Literal::u8_unsuffixed(*b));
    let cpi_signer_literals = pda_bytes
        .iter()
        .map(|b| proc_macro2::Literal::u8_unsuffixed(*b));

    let output = quote! {
        {
            // Use the CpiSigner type from the current scope (should be imported)
            CpiSigner {
                program_id: [#(#program_id_literals),*],
                cpi_signer: [#(#cpi_signer_literals),*],
                bump: #bump,
            }
        }
    };

    output.into()
}

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

const CPI_AUTHORITY_SEED: &[u8] = b"cpi_authority";

/// Derives a Light Protocol CPI signer PDA at compile time
///
/// This macro computes the CPI signer PDA using the "cpi_authority" seed
/// for the given program ID at compile time.
///
/// ## Usage
///
/// ```rust
/// # use light_macros::derive_light_cpi_signer_pda;
/// // Derive CPI signer for your program
/// const CPI_SIGNER_DATA: ([u8; 32], u8) = derive_light_cpi_signer_pda!("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");
/// const CPI_SIGNER: [u8; 32] = CPI_SIGNER_DATA.0;
/// const CPI_SIGNER_BUMP: u8 = CPI_SIGNER_DATA.1;
/// ```
///
/// Returns a tuple of `([u8; 32], u8)` containing the derived PDA address and bump seed.
pub fn derive_light_cpi_signer_pda(input: TokenStream) -> TokenStream {
    let program_id_lit = parse_macro_input!(input as LitStr);
    let program_id_str = program_id_lit.value();

    // Parse program ID at compile time
    use std::str::FromStr;
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
    let seeds = &[CPI_AUTHORITY_SEED];

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

/// Derives a complete Light Protocol CPI signer configuration at compile time
///
/// This macro computes the CPI signer configuration for the given program ID
/// at compile time.
///
/// ## Usage
///
/// ```rust
/// # use light_macros::derive_light_cpi_signer;
/// // Requires CpiSigner struct to be in scope
/// struct CpiSigner {
///     program_id: [u8; 32],
///     cpi_signer: [u8; 32],
///     bump: u8,
/// }
/// // In a Solana program
/// const LIGHT_CPI_SIGNER: CpiSigner = derive_light_cpi_signer!("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");
/// ```
///
/// Returns a `CpiSigner` struct (must be in scope) containing:
/// - `program_id`: Program ID bytes
/// - `cpi_signer`: CPI signer PDA address
/// - `bump`: Bump seed
pub fn derive_light_cpi_signer(input: TokenStream) -> TokenStream {
    let program_id_lit = parse_macro_input!(input as LitStr);
    let program_id_str = program_id_lit.value();

    // Parse program ID at compile time
    use std::str::FromStr;
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
    let seeds = &[CPI_AUTHORITY_SEED];

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
        CpiSigner {
            program_id: [#(#program_id_literals),*],
            cpi_signer: [#(#cpi_signer_literals),*],
            bump: #bump,
        }
    };

    output.into()
}

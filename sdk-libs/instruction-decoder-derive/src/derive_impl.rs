//! Implementation of the `#[derive(InstructionDecoder)]` macro.
//!
//! This module provides the core implementation for deriving InstructionDecoder
//! on instruction enums.

use darling::FromDeriveInput;
use proc_macro2::TokenStream as TokenStream2;
use syn::DeriveInput;

use crate::{builder::InstructionDecoderBuilder, parsing::InstructionDecoderArgs};

/// Main implementation for the `#[derive(InstructionDecoder)]` macro.
///
/// This parses the input enum and its attributes, then generates the decoder
/// struct and trait implementation.
///
/// # Errors
///
/// Returns an error if:
/// - Input is not an enum
/// - Required attributes are missing (program_id)
/// - Attribute values are invalid (e.g., invalid base58, unsupported discriminator_size)
pub fn derive_instruction_decoder_impl(input: DeriveInput) -> syn::Result<TokenStream2> {
    // Parse attributes using darling
    let args = InstructionDecoderArgs::from_derive_input(&input)
        .map_err(|e| syn::Error::new(e.span(), e.to_string()))?;

    // Create builder and generate code
    let builder = InstructionDecoderBuilder::new(&args, &input)?;
    builder.generate(&input)
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::*;

    #[test]
    fn test_derive_basic_enum() {
        let input: DeriveInput = syn::parse2(quote! {
            #[instruction_decoder(
                program_id = "11111111111111111111111111111111",
                program_name = "Test Program"
            )]
            pub enum TestInstruction {
                Init,
                Process,
            }
        })
        .unwrap();

        let result = derive_instruction_decoder_impl(input);
        assert!(result.is_ok());

        let output = result.unwrap().to_string();
        assert!(output.contains("TestInstructionDecoder"));
        assert!(output.contains("InstructionDecoder"));
    }

    #[test]
    fn test_derive_with_fields() {
        let input: DeriveInput = syn::parse2(quote! {
            #[instruction_decoder(
                program_id = "11111111111111111111111111111111",
                discriminator_size = 1
            )]
            pub enum TestInstruction {
                Transfer { amount: u64 },
                Withdraw(u64),
            }
        })
        .unwrap();

        let result = derive_instruction_decoder_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_derive_with_accounts() {
        let input: DeriveInput = syn::parse2(quote! {
            #[instruction_decoder(
                program_id = "11111111111111111111111111111111"
            )]
            pub enum TestInstruction {
                #[instruction_decoder(accounts = CreateRecord)]
                CreateRecord,
            }
        })
        .unwrap();

        let result = derive_instruction_decoder_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_derive_missing_program_id() {
        let input: DeriveInput = syn::parse2(quote! {
            pub enum TestInstruction {
                Init,
            }
        })
        .unwrap();

        let result = derive_instruction_decoder_impl(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_derive_invalid_discriminator_size() {
        let input: DeriveInput = syn::parse2(quote! {
            #[instruction_decoder(
                program_id = "11111111111111111111111111111111",
                discriminator_size = 16
            )]
            pub enum TestInstruction {
                Init,
            }
        })
        .unwrap();

        let result = derive_instruction_decoder_impl(input);
        assert!(result.is_err());
    }
}

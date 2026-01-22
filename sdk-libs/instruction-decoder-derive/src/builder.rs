//! Builder pattern for InstructionDecoder code generation.
//!
//! This module provides the `InstructionDecoderBuilder` which handles the core
//! code generation logic for both the derive macro and attribute macro.

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::Fields;

use crate::{
    crate_context::CrateContext,
    parsing::{
        parse_explicit_discriminator, ExplicitDiscriminator, InstructionDecoderArgs,
        VariantDecoderArgs,
    },
    utils::{compute_anchor_discriminator, to_snake_case},
};

/// Builder for generating InstructionDecoder implementations.
///
/// Handles the code generation for decoder structs and trait implementations.
pub struct InstructionDecoderBuilder<'a> {
    /// Parsed top-level attributes
    args: &'a InstructionDecoderArgs,
    /// Explicit discriminator values (indexed by variant position)
    explicit_discriminators: Vec<Option<ExplicitDiscriminator>>,
    /// Parsed program ID bytes as token stream
    program_id_bytes: TokenStream2,
    /// Crate context for resolving struct field names at compile time
    crate_ctx: Option<CrateContext>,
}

impl<'a> InstructionDecoderBuilder<'a> {
    /// Create a new builder from parsed arguments.
    ///
    /// # Errors
    ///
    /// Returns an error if program_id is invalid or discriminator_size is unsupported.
    pub fn new(args: &'a InstructionDecoderArgs, input: &syn::DeriveInput) -> syn::Result<Self> {
        // Validate arguments
        args.validate()?;

        // Parse program ID bytes
        let program_id_bytes = args.program_id_bytes(args.ident.span())?;

        // Parse explicit discriminators from variants
        let explicit_discriminators = Self::parse_explicit_discriminators(input)?;

        // Try to parse CrateContext for account name resolution
        // This allows us to extract struct field names at compile time
        let crate_ctx = CrateContext::parse_from_manifest().ok();

        Ok(Self {
            args,
            explicit_discriminators,
            program_id_bytes,
            crate_ctx,
        })
    }

    /// Parse explicit discriminator attributes from all variants.
    fn parse_explicit_discriminators(
        input: &syn::DeriveInput,
    ) -> syn::Result<Vec<Option<ExplicitDiscriminator>>> {
        match &input.data {
            syn::Data::Enum(data_enum) => data_enum
                .variants
                .iter()
                .map(parse_explicit_discriminator)
                .collect(),
            _ => Err(syn::Error::new_spanned(
                input,
                "InstructionDecoder can only be derived for enums",
            )),
        }
    }

    /// Generate the complete decoder implementation.
    pub fn generate(&self, input: &syn::DeriveInput) -> syn::Result<TokenStream2> {
        let name = &self.args.ident;
        let decoder_name = format_ident!("{}Decoder", name);
        let program_name = self.args.display_name();

        // Generate match arms
        let match_arms = self.generate_match_arms(input)?;

        // Generate decoder based on discriminator size
        let inner = self.generate_decoder_impl(&decoder_name, &program_name, &match_arms);

        // Wrap in cfg gate and module
        let mod_name = format_ident!("__instruction_decoder_{}", name.to_string().to_lowercase());
        Ok(quote! {
            #[cfg(not(target_os = "solana"))]
            mod #mod_name {
                use super::*;
                #inner
            }
            #[cfg(not(target_os = "solana"))]
            pub use #mod_name::#decoder_name;
        })
    }

    /// Generate match arms for all variants.
    fn generate_match_arms(&self, input: &syn::DeriveInput) -> syn::Result<Vec<TokenStream2>> {
        let data_enum = match &input.data {
            syn::Data::Enum(data) => data,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "InstructionDecoder can only be derived for enums",
                ))
            }
        };

        let variants = self.args.variants();

        data_enum
            .variants
            .iter()
            .zip(variants.iter())
            .enumerate()
            .map(|(idx, (variant, variant_args))| {
                self.generate_match_arm(idx, variant, variant_args)
            })
            .collect()
    }

    /// Generate a single match arm for a variant.
    fn generate_match_arm(
        &self,
        index: usize,
        variant: &syn::Variant,
        variant_args: &VariantDecoderArgs,
    ) -> syn::Result<TokenStream2> {
        let instruction_name = variant.ident.to_string();

        // Generate the body code based on whether we have a dynamic resolver
        let body_code = self.generate_match_arm_body(variant, variant_args)?;

        match self.args.discriminator_size {
            1 => {
                let disc = match &self.explicit_discriminators[index] {
                    Some(ExplicitDiscriminator::U32(d)) => {
                        if *d > u8::MAX as u32 {
                            return Err(syn::Error::new(
                                variant.ident.span(),
                                format!(
                                    "discriminator value {} exceeds u8::MAX (255) for 1-byte discriminator size",
                                    d
                                ),
                            ));
                        }
                        *d as u8
                    }
                    Some(ExplicitDiscriminator::Array(_)) => {
                        return Err(syn::Error::new(
                            variant.ident.span(),
                            "array discriminator not supported for 1-byte discriminator size",
                        ));
                    }
                    None => {
                        if index > u8::MAX as usize {
                            return Err(syn::Error::new(
                                variant.ident.span(),
                                format!(
                                    "variant index {} exceeds u8::MAX (255) for 1-byte discriminator size",
                                    index
                                ),
                            ));
                        }
                        index as u8
                    }
                };
                Ok(quote! {
                    #disc => {
                        #body_code
                        Some(light_instruction_decoder::DecodedInstruction::with_fields_and_accounts(
                            #instruction_name,
                            fields,
                            account_names,
                        ))
                    }
                })
            }
            4 => {
                let disc = match &self.explicit_discriminators[index] {
                    Some(ExplicitDiscriminator::U32(d)) => *d,
                    Some(ExplicitDiscriminator::Array(_)) => {
                        return Err(syn::Error::new(
                            variant.ident.span(),
                            "array discriminator not supported for 4-byte discriminator size",
                        ));
                    }
                    None => {
                        if index > u32::MAX as usize {
                            return Err(syn::Error::new(
                                variant.ident.span(),
                                format!(
                                    "variant index {} exceeds u32::MAX for 4-byte discriminator size",
                                    index
                                ),
                            ));
                        }
                        index as u32
                    }
                };
                Ok(quote! {
                    #disc => {
                        #body_code
                        Some(light_instruction_decoder::DecodedInstruction::with_fields_and_accounts(
                            #instruction_name,
                            fields,
                            account_names,
                        ))
                    }
                })
            }
            8 => {
                // For 8-byte mode: check for explicit array discriminator first,
                // then fall back to explicit u32, then to computed Anchor discriminator
                let discriminator: [u8; 8] = match &self.explicit_discriminators[index] {
                    Some(ExplicitDiscriminator::Array(arr)) => *arr,
                    Some(ExplicitDiscriminator::U32(_)) => {
                        return Err(syn::Error::new(
                            variant.ident.span(),
                            "use array discriminator syntax #[discriminator = [a, b, ...]] for 8-byte discriminator size",
                        ));
                    }
                    None => {
                        // Fall back to computed Anchor discriminator
                        let snake_name = to_snake_case(&instruction_name);
                        compute_anchor_discriminator(&snake_name)
                    }
                };
                let disc_array = discriminator.iter();
                Ok(quote! {
                    [#(#disc_array),*] => {
                        #body_code
                        Some(light_instruction_decoder::DecodedInstruction::with_fields_and_accounts(
                            #instruction_name,
                            fields,
                            account_names,
                        ))
                    }
                })
            }
            _ => Err(syn::Error::new(
                variant.ident.span(),
                "unsupported discriminator size",
            )),
        }
    }

    /// Generate the body code for a match arm.
    ///
    /// When `account_names_resolver_from_params` is specified, this generates code that:
    /// 1. Parses params first
    /// 2. Calls the resolver function with params and accounts to get dynamic account names
    /// 3. Calls the formatter if specified
    ///
    /// Otherwise, it uses static account names from `accounts` or `account_names`.
    fn generate_match_arm_body(
        &self,
        variant: &syn::Variant,
        variant_args: &VariantDecoderArgs,
    ) -> syn::Result<TokenStream2> {
        // Check if we have a dynamic account names resolver
        if let (Some(resolver_path), Some(params_ty)) = (
            &variant_args.account_names_resolver_from_params,
            variant_args.params_type(),
        ) {
            // Dynamic resolver mode: parse params first, then call resolver
            let fields_code = if let Some(formatter_path) = &variant_args.pretty_formatter {
                // Use custom formatter
                quote! {
                    let mut fields = Vec::new();
                    let formatted = #formatter_path(&params, accounts);
                    fields.push(light_instruction_decoder::DecodedField::new(
                        "",
                        formatted,
                    ));
                    fields
                }
            } else {
                // Use Debug formatting
                quote! {
                    let mut fields = Vec::new();
                    fields.push(light_instruction_decoder::DecodedField::new(
                        "",
                        format!("{:#?}", params),
                    ));
                    fields
                }
            };

            Ok(quote! {
                let (account_names, fields) = if let Ok(params) = <#params_ty as borsh::BorshDeserialize>::try_from_slice(remaining) {
                    let account_names = #resolver_path(&params, accounts);
                    let fields = { #fields_code };
                    (account_names, fields)
                } else {
                    let account_names: Vec<String> = Vec::new();
                    let mut fields = Vec::new();
                    if !remaining.is_empty() {
                        fields.push(light_instruction_decoder::DecodedField::new(
                            "data_len",
                            remaining.len().to_string(),
                        ));
                    }
                    (account_names, fields)
                };
            })
        } else {
            // Static account names mode
            let account_names_code = variant_args.account_names_code(self.crate_ctx.as_ref());
            let fields_code = self.generate_fields_code(variant, variant_args)?;

            Ok(quote! {
                let account_names: Vec<String> = #account_names_code;
                let fields = { #fields_code };
            })
        }
    }

    /// Generate field parsing code for a variant.
    fn generate_fields_code(
        &self,
        variant: &syn::Variant,
        variant_args: &VariantDecoderArgs,
    ) -> syn::Result<TokenStream2> {
        // If params type is specified, use borsh deserialization
        if let Some(params_ty) = variant_args.params_type() {
            // Check if pretty_formatter is specified
            if let Some(formatter_path) = &variant_args.pretty_formatter {
                return Ok(quote! {
                    let mut fields = Vec::new();
                    if let Ok(params) = <#params_ty as borsh::BorshDeserialize>::try_from_slice(remaining) {
                        // Call custom formatter with params and accounts
                        let formatted = #formatter_path(&params, accounts);
                        fields.push(light_instruction_decoder::DecodedField::new(
                            "",
                            formatted,
                        ));
                    } else if !remaining.is_empty() {
                        fields.push(light_instruction_decoder::DecodedField::new(
                            "data_len",
                            remaining.len().to_string(),
                        ));
                    }
                    fields
                });
            }

            // Default: use Debug formatting
            return Ok(quote! {
                let mut fields = Vec::new();
                if let Ok(params) = <#params_ty as borsh::BorshDeserialize>::try_from_slice(remaining) {
                    fields.push(light_instruction_decoder::DecodedField::new(
                        "",
                        format!("{:#?}", params),
                    ));
                } else if !remaining.is_empty() {
                    fields.push(light_instruction_decoder::DecodedField::new(
                        "data_len",
                        remaining.len().to_string(),
                    ));
                }
                fields
            });
        }

        // Otherwise, generate native field parsing based on variant fields
        generate_native_fields_code(variant)
    }

    /// Generate the decoder struct and impl based on discriminator size.
    fn generate_decoder_impl(
        &self,
        decoder_name: &syn::Ident,
        program_name: &str,
        match_arms: &[TokenStream2],
    ) -> TokenStream2 {
        let program_id_bytes = &self.program_id_bytes;
        let disc_size = self.args.discriminator_size as usize;

        match self.args.discriminator_size {
            1 => quote! {
                /// Generated InstructionDecoder implementation
                pub struct #decoder_name;

                impl light_instruction_decoder::InstructionDecoder for #decoder_name {
                    fn program_id(&self) -> light_instruction_decoder::solana_pubkey::Pubkey {
                        light_instruction_decoder::solana_pubkey::Pubkey::new_from_array(#program_id_bytes)
                    }

                    fn program_name(&self) -> &'static str {
                        #program_name
                    }

                    fn decode(
                        &self,
                        data: &[u8],
                        accounts: &[light_instruction_decoder::solana_instruction::AccountMeta],
                    ) -> Option<light_instruction_decoder::DecodedInstruction> {
                        if data.len() < #disc_size {
                            return None;
                        }

                        let discriminator = data[0];
                        let remaining = &data[1..];

                        match discriminator {
                            #(#match_arms)*
                            _ => None,
                        }
                    }
                }
            },
            4 => quote! {
                /// Generated InstructionDecoder implementation
                pub struct #decoder_name;

                impl light_instruction_decoder::InstructionDecoder for #decoder_name {
                    fn program_id(&self) -> light_instruction_decoder::solana_pubkey::Pubkey {
                        light_instruction_decoder::solana_pubkey::Pubkey::new_from_array(#program_id_bytes)
                    }

                    fn program_name(&self) -> &'static str {
                        #program_name
                    }

                    fn decode(
                        &self,
                        data: &[u8],
                        accounts: &[light_instruction_decoder::solana_instruction::AccountMeta],
                    ) -> Option<light_instruction_decoder::DecodedInstruction> {
                        if data.len() < 4 {
                            return None;
                        }

                        let discriminator = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                        let remaining = &data[4..];

                        match discriminator {
                            #(#match_arms)*
                            _ => None,
                        }
                    }
                }
            },
            _ => quote! {
                /// Generated InstructionDecoder implementation
                pub struct #decoder_name;

                impl light_instruction_decoder::InstructionDecoder for #decoder_name {
                    fn program_id(&self) -> light_instruction_decoder::solana_pubkey::Pubkey {
                        light_instruction_decoder::solana_pubkey::Pubkey::new_from_array(#program_id_bytes)
                    }

                    fn program_name(&self) -> &'static str {
                        #program_name
                    }

                    fn decode(
                        &self,
                        data: &[u8],
                        accounts: &[light_instruction_decoder::solana_instruction::AccountMeta],
                    ) -> Option<light_instruction_decoder::DecodedInstruction> {
                        if data.len() < 8 {
                            return None;
                        }

                        let discriminator: [u8; 8] = data[0..8].try_into().ok()?;
                        let remaining = &data[8..];

                        match discriminator {
                            #(#match_arms)*
                            _ => None,
                        }
                    }
                }
            },
        }
    }
}

/// Generate field parsing code for native program instructions.
/// Parses fields based on their types (u8, u16, u32, u64, i64) using little-endian byte reading.
pub fn generate_native_fields_code(variant: &syn::Variant) -> syn::Result<TokenStream2> {
    match &variant.fields {
        Fields::Named(fields_named) => {
            let mut field_parsers = Vec::new();
            let mut offset: usize = 0;

            for field in &fields_named.named {
                let field_name = field.ident.as_ref().unwrap().to_string();
                let field_type = &field.ty;
                let type_str = quote!(#field_type).to_string();

                let (parser, size) = generate_field_parser(&field_name, &type_str, offset);
                field_parsers.push(parser);
                offset += size;
            }

            Ok(quote! {
                let mut fields = Vec::new();
                #(#field_parsers)*
                fields
            })
        }
        Fields::Unnamed(fields_unnamed) => {
            let mut field_parsers = Vec::new();
            let mut offset: usize = 0;

            for (i, field) in fields_unnamed.unnamed.iter().enumerate() {
                let field_name = format!("arg{}", i);
                let field_type = &field.ty;
                let type_str = quote!(#field_type).to_string();

                let (parser, size) = generate_field_parser(&field_name, &type_str, offset);
                field_parsers.push(parser);
                offset += size;
            }

            Ok(quote! {
                let mut fields = Vec::new();
                #(#field_parsers)*
                fields
            })
        }
        Fields::Unit => Ok(quote! {
            let fields: Vec<light_instruction_decoder::DecodedField> = Vec::new();
            fields
        }),
    }
}

/// Generate parser code for a single field based on its type.
fn generate_field_parser(field_name: &str, type_str: &str, offset: usize) -> (TokenStream2, usize) {
    match type_str {
        "u8" => (
            quote! {
                if remaining.len() > #offset {
                    let value = remaining[#offset];
                    fields.push(light_instruction_decoder::DecodedField::new(
                        #field_name,
                        value.to_string(),
                    ));
                }
            },
            1,
        ),
        "u16" => (
            quote! {
                if remaining.len() > #offset + 1 {
                    let value = u16::from_le_bytes([
                        remaining[#offset],
                        remaining[#offset + 1],
                    ]);
                    fields.push(light_instruction_decoder::DecodedField::new(
                        #field_name,
                        value.to_string(),
                    ));
                }
            },
            2,
        ),
        "u32" => (
            quote! {
                if remaining.len() > #offset + 3 {
                    let value = u32::from_le_bytes([
                        remaining[#offset],
                        remaining[#offset + 1],
                        remaining[#offset + 2],
                        remaining[#offset + 3],
                    ]);
                    fields.push(light_instruction_decoder::DecodedField::new(
                        #field_name,
                        value.to_string(),
                    ));
                }
            },
            4,
        ),
        "u64" => (
            quote! {
                if remaining.len() > #offset + 7 {
                    let value = u64::from_le_bytes([
                        remaining[#offset],
                        remaining[#offset + 1],
                        remaining[#offset + 2],
                        remaining[#offset + 3],
                        remaining[#offset + 4],
                        remaining[#offset + 5],
                        remaining[#offset + 6],
                        remaining[#offset + 7],
                    ]);
                    fields.push(light_instruction_decoder::DecodedField::new(
                        #field_name,
                        value.to_string(),
                    ));
                }
            },
            8,
        ),
        "i64" => (
            quote! {
                if remaining.len() > #offset + 7 {
                    let value = i64::from_le_bytes([
                        remaining[#offset],
                        remaining[#offset + 1],
                        remaining[#offset + 2],
                        remaining[#offset + 3],
                        remaining[#offset + 4],
                        remaining[#offset + 5],
                        remaining[#offset + 6],
                        remaining[#offset + 7],
                    ]);
                    fields.push(light_instruction_decoder::DecodedField::new(
                        #field_name,
                        value.to_string(),
                    ));
                }
            },
            8,
        ),
        _ => (
            quote! {
                fields.push(light_instruction_decoder::DecodedField::new(
                    #field_name,
                    format!("({}bytes)", remaining.len().saturating_sub(#offset)),
                ));
            },
            0,
        ),
    }
}

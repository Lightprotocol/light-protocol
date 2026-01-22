//! Implementation of the `#[instruction_decoder]` attribute macro.
//!
//! This module provides the attribute macro that can be applied to Anchor program
//! modules to automatically generate InstructionDecoder implementations.

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::ItemMod;

use crate::{
    crate_context::CrateContext,
    parsing::ModuleDecoderArgs,
    utils::{compute_anchor_discriminator, to_pascal_case},
};

/// Information about a single parameter
struct ParamInfo {
    name: syn::Ident,
    ty: syn::Type,
}

/// Information about an instruction extracted from the module.
struct InstructionInfo {
    /// Function name (snake_case)
    name: String,
    /// Account field names extracted from the Accounts struct
    account_names: Vec<String>,
    /// All parameters after Context
    params: Vec<ParamInfo>,
}

/// Main implementation for the `#[instruction_decoder]` attribute macro.
///
/// This extracts function names from an Anchor program module and generates
/// an InstructionDecoder implementation.
///
/// # Errors
///
/// Returns an error if:
/// - Input is not a module
/// - Module parsing fails
pub fn instruction_decoder_attr(
    attr: TokenStream2,
    item: TokenStream2,
) -> syn::Result<TokenStream2> {
    let module: ItemMod = syn::parse2(item.clone())?;

    // Parse attribute arguments
    let mut args = ModuleDecoderArgs::parse(attr)?;

    // Try to find declare_id! in module if program_id not specified
    args.find_declare_id(&module)?;

    // Extract function info from the module
    let instructions = extract_instruction_info(&module)?;

    if instructions.is_empty() {
        // No functions found, just return the module as-is
        return Ok(item);
    }

    let module_name = &module.ident;
    let decoder_name = format_ident!(
        "{}InstructionDecoder",
        to_pascal_case(&module_name.to_string())
    );

    // Generate match arms for each instruction
    let match_arms = generate_match_arms(&instructions);

    // Generate params structs for all instructions that have params
    let params_structs: Vec<TokenStream2> = instructions
        .iter()
        .filter_map(|info| generate_params_struct(&info.name, &info.params))
        .collect();

    // Get program ID and name
    let program_id_source = args.program_id_source();
    let program_id_impl = program_id_source.program_id_impl();
    let program_name = args.program_name(&module_name.to_string());

    // Generate the decoder struct and implementation
    let decoder_impl = quote! {
        #[cfg(not(target_os = "solana"))]
        /// Generated InstructionDecoder for the program module (off-chain only)
        pub struct #decoder_name;

        // Generated params structs for deserialization (off-chain only)
        #(
            #[cfg(not(target_os = "solana"))]
            #params_structs
        )*

        #[cfg(not(target_os = "solana"))]
        impl light_instruction_decoder::InstructionDecoder for #decoder_name {
            #program_id_impl

            fn program_name(&self) -> &'static str {
                #program_name
            }

            fn decode(
                &self,
                data: &[u8],
                _accounts: &[light_instruction_decoder::solana_instruction::AccountMeta],
            ) -> Option<light_instruction_decoder::DecodedInstruction> {
                if data.len() < 8 {
                    return None;
                }

                let discriminator: [u8; 8] = data[0..8].try_into().ok()?;

                match discriminator {
                    #(#match_arms)*
                    _ => None,
                }
            }
        }
    };

    // Return the original module plus the generated decoder
    Ok(quote! {
        #item
        #decoder_impl
    })
}

/// Generate match arms for all instructions.
fn generate_match_arms(instructions: &[InstructionInfo]) -> Vec<TokenStream2> {
    instructions
        .iter()
        .map(|info| {
            let pascal_name = to_pascal_case(&info.name);
            let discriminator = compute_anchor_discriminator(&info.name);
            let disc_array = discriminator.iter();

            // Generate params decoding code using the generated DecoderParams struct
            let fields_code = if info.params.is_empty() {
                quote! { Vec::new() }
            } else {
                let params_struct_name = format_ident!("{}DecoderParams", pascal_name);
                // Generate field accessors for each parameter with their field names
                let field_pushes: Vec<TokenStream2> = info.params.iter().map(|param| {
                    let field_name = &param.name;
                    let field_name_str = field_name.to_string();
                    quote! {
                        fields.push(light_instruction_decoder::DecodedField::new(
                            #field_name_str,
                            format!("{:#?}", params.#field_name),
                        ));
                    }
                }).collect();
                quote! {
                    let mut fields = Vec::new();
                    if let Ok(params) = <#params_struct_name as borsh::BorshDeserialize>::try_from_slice(remaining) {
                        #(#field_pushes)*
                    } else if !remaining.is_empty() {
                        fields.push(light_instruction_decoder::DecodedField::new(
                            "data_len",
                            remaining.len().to_string(),
                        ));
                    }
                    fields
                }
            };

            let account_names = &info.account_names;
            if account_names.is_empty() {
                quote! {
                    [#(#disc_array),*] => {
                        let remaining = &data[8..];
                        let fields = { #fields_code };
                        Some(light_instruction_decoder::DecodedInstruction::with_fields_and_accounts(
                            #pascal_name,
                            fields,
                            Vec::new(),
                        ))
                    }
                }
            } else {
                quote! {
                    [#(#disc_array),*] => {
                        let remaining = &data[8..];
                        let fields = { #fields_code };
                        Some(light_instruction_decoder::DecodedInstruction::with_fields_and_accounts(
                            #pascal_name,
                            fields,
                            vec![#(#account_names.to_string()),*],
                        ))
                    }
                }
            }
        })
        .collect()
}

/// Extract public function information from an Anchor program module.
fn extract_instruction_info(module: &ItemMod) -> syn::Result<Vec<InstructionInfo>> {
    // Parse entire crate to find Accounts structs
    let crate_ctx = CrateContext::parse_from_manifest()?;

    let mut instructions = Vec::new();

    if let Some(ref content) = module.content {
        for item in &content.1 {
            if let syn::Item::Fn(func) = item {
                // Only include public functions
                if matches!(func.vis, syn::Visibility::Public(_)) {
                    let name = func.sig.ident.to_string();

                    // Extract Context<T> type from first parameter and look up account names
                    let account_names = if let Some(type_name) = extract_context_type(&func.sig) {
                        crate_ctx
                            .get_struct_field_names(&type_name)
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    };

                    // Extract all parameters after Context
                    let params = extract_all_params(&func.sig);

                    instructions.push(InstructionInfo {
                        name,
                        account_names,
                        params,
                    });
                }
            }
        }
    }

    Ok(instructions)
}

/// Extract the type name from Context<T> in a function signature.
///
/// Handles various patterns:
/// - `Context<'_, '_, '_, 'info, T<'info>>` -> "T"
/// - `Context<T>` -> "T"
fn extract_context_type(sig: &syn::Signature) -> Option<String> {
    for input in &sig.inputs {
        if let syn::FnArg::Typed(pat_type) = input {
            if let syn::Type::Path(type_path) = &*pat_type.ty {
                if let Some(segment) = type_path.path.segments.last() {
                    if segment.ident == "Context" {
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            // Get the last type argument (accounts struct)
                            if let Some(syn::GenericArgument::Type(ty)) = args.args.last() {
                                return extract_type_name(ty);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Extract the simple type name from a Type.
fn extract_type_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(type_path) => type_path.path.segments.last().map(|s| s.ident.to_string()),
        _ => None,
    }
}

/// Extract ALL parameters after Context from a function signature.
///
/// This mirrors how Anchor generates its instruction structs - iterating
/// all args after Context and generating a struct field for each.
/// We generate our own struct with Debug derive for decoding.
fn extract_all_params(sig: &syn::Signature) -> Vec<ParamInfo> {
    let mut params = Vec::new();
    let mut found_context = false;

    for input in &sig.inputs {
        if let syn::FnArg::Typed(pat_type) = input {
            if let syn::Type::Path(type_path) = &*pat_type.ty {
                if let Some(segment) = type_path.path.segments.last() {
                    if segment.ident == "Context" {
                        found_context = true;
                        continue;
                    }
                }
            }
            if found_context {
                if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                    params.push(ParamInfo {
                        name: pat_ident.ident.clone(),
                        ty: (*pat_type.ty).clone(),
                    });
                }
            }
        }
    }
    params
}

/// Generate a params struct for an instruction with Debug and BorshDeserialize.
///
/// Returns None if the instruction has no parameters.
fn generate_params_struct(instruction_name: &str, params: &[ParamInfo]) -> Option<TokenStream2> {
    if params.is_empty() {
        return None;
    }

    let struct_name = format_ident!("{}DecoderParams", to_pascal_case(instruction_name));

    let fields: Vec<TokenStream2> = params
        .iter()
        .map(|param| {
            let name = &param.name;
            let ty = &param.ty;
            quote! { pub #name: #ty }
        })
        .collect();

    Some(quote! {
        #[derive(Debug, borsh::BorshDeserialize)]
        struct #struct_name {
            #(#fields),*
        }
    })
}

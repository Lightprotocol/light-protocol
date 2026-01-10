//! Code generation for LightFinalize trait implementation.
//!
//! Currently supports:
//! - Compressible PDAs via `#[compressible(...)]` attribute
//!
//! NOT YET SUPPORTED:
//! - `#[light_mint(...)]` attribute (use light_ctoken_sdk directly)
//! - Mixed PDAs + mints

use super::parse::{CompressibleField, ParsedCompressibleStruct};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generate the LightFinalize trait implementation
pub fn generate_finalize_impl(parsed: &ParsedCompressibleStruct) -> TokenStream {
    let struct_name = &parsed.struct_name;
    let (impl_generics, ty_generics, where_clause) = parsed.generics.split_for_impl();

    // Get the params type from instruction args (first arg)
    let params_type = parsed
        .instruction_args
        .as_ref()
        .and_then(|args| args.first())
        .map(|arg| &arg.ty);

    let params_type = match params_type {
        Some(ty) => ty,
        None => {
            // No instruction args - generate no-op impl
            return quote! {
                #[automatically_derived]
                impl #impl_generics light_sdk::compressible::LightFinalize<'info, ()> for #struct_name #ty_generics #where_clause {
                    fn light_finalize(
                        &mut self,
                        _remaining: &[solana_account_info::AccountInfo<'info>],
                        _params: &(),
                    ) -> std::result::Result<(), light_sdk::error::LightSdkError> {
                        Ok(())
                    }
                }
            };
        }
    };

    let params_ident = parsed
        .instruction_args
        .as_ref()
        .and_then(|args| args.first())
        .map(|arg| &arg.name)
        .expect("params ident must exist if type exists");

    let has_pdas = !parsed.compressible_fields.is_empty();
    let has_mints = !parsed.light_mint_fields.is_empty();

    // If nothing to process, generate no-op
    if !has_pdas && !has_mints {
        return quote! {
            #[automatically_derived]
            impl #impl_generics light_sdk::compressible::LightFinalize<'info, #params_type> for #struct_name #ty_generics #where_clause {
                fn light_finalize(
                    &mut self,
                    _remaining: &[solana_account_info::AccountInfo<'info>],
                    _params: &#params_type,
                ) -> std::result::Result<(), light_sdk::error::LightSdkError> {
                    Ok(())
                }
            }
        };
    }

    // Get fee payer field
    let fee_payer = parsed
        .fee_payer_field
        .as_ref()
        .map(|f| quote! { #f })
        .unwrap_or_else(|| quote! { fee_payer });

    let compression_config = parsed
        .compression_config_field
        .as_ref()
        .map(|f| quote! { #f })
        .unwrap_or_else(|| quote! { compression_config });

    // Generate the finalize body based on what we have
    let finalize_body = if has_pdas && has_mints {
        generate_mixed_finalize(parsed, params_ident, &fee_payer, &compression_config)
    } else if has_pdas {
        generate_pda_only_finalize(parsed, params_ident, &fee_payer, &compression_config)
    } else {
        generate_mint_only_finalize(parsed, params_ident, &fee_payer)
    };

    quote! {
        #[automatically_derived]
        impl #impl_generics light_sdk::compressible::LightFinalize<'info, #params_type> for #struct_name #ty_generics #where_clause {
            fn light_finalize(
                &mut self,
                _remaining: &[solana_account_info::AccountInfo<'info>],
                #params_ident: &#params_type,
            ) -> std::result::Result<(), light_sdk::error::LightSdkError> {
                use anchor_lang::ToAccountInfo;
                #finalize_body
                Ok(())
            }
        }
    }
}

/// Generate finalize code for PDAs only (no mints)
/// 
/// PDAs-only does NOT need CPI context - Light System Program handles
/// multiple PDAs in a single call via with_new_addresses(&[...])
fn generate_pda_only_finalize(
    parsed: &ParsedCompressibleStruct,
    params_ident: &syn::Ident,
    fee_payer: &TokenStream,
    compression_config: &TokenStream,
) -> TokenStream {
    let (compress_blocks, new_addr_idents) =
        generate_pda_compress_blocks(&parsed.compressible_fields);
    let compressible_count = parsed.compressible_fields.len() as u8;

    quote! {
        // Build CPI accounts (no CPI context needed for PDAs-only)
        let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new(
            &self.#fee_payer,
            _remaining,
            crate::LIGHT_CPI_SIGNER,
        );

        // Load compression config
        let compression_config_data = light_sdk::compressible::CompressibleConfig::load_checked(
            &self.#compression_config,
            &crate::ID
        )?;

        // Collect compressed infos for all compressible accounts
        let mut all_compressed_infos = Vec::with_capacity(#compressible_count as usize);
        #(#compress_blocks)*

        // Execute Light System Program CPI directly with proof
        // No CPI context needed - single call handles all PDAs
        use light_sdk::cpi::{InvokeLightSystemProgram, LightCpiInstruction};
        light_sdk::cpi::v2::LightSystemProgramCpi::new_cpi(
            crate::LIGHT_CPI_SIGNER,
            #params_ident.proof.clone()
        )
            .with_new_addresses(&[#(#new_addr_idents),*])
            .with_account_infos(&all_compressed_infos)
            .invoke(cpi_accounts)?;
    }
}

/// Generate finalize code for mints only (no PDAs)
///
/// NOTE: light_mint support is currently disabled due to incomplete implementation.
/// Users should use the ctoken-sdk directly for mint creation.
fn generate_mint_only_finalize(
    _parsed: &ParsedCompressibleStruct,
    _params_ident: &syn::Ident,
    _fee_payer: &TokenStream,
) -> TokenStream {
    // light_mint support is incomplete - SystemAccountInfos::try_from_remaining_accounts doesn't exist
    // Return a compile error if this path is reached
    quote! {
        compile_error!("#[light_mint] attribute is not yet supported. Use light_ctoken_sdk directly for mint creation.");
    }
}

/// Generate finalize code for mixed PDAs + mints
///
/// NOTE: light_mint support is currently disabled due to incomplete implementation.
/// Users should use the ctoken-sdk directly for mint creation.
fn generate_mixed_finalize(
    _parsed: &ParsedCompressibleStruct,
    _params_ident: &syn::Ident,
    _fee_payer: &TokenStream,
    _compression_config: &TokenStream,
) -> TokenStream {
    // Mixed PDA + mint support is incomplete - requires light_mint which isn't implemented
    quote! {
        compile_error!("Mixed #[compressible] and #[light_mint] attributes are not yet supported. Use light_ctoken_sdk directly for mint creation.");
    }
}

/// Generate compression blocks for PDA fields
fn generate_pda_compress_blocks(fields: &[CompressibleField]) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let mut blocks = Vec::new();
    let mut addr_idents = Vec::new();

    for (idx, field) in fields.iter().enumerate() {
        let idx_lit = idx as u8;
        let ident = &field.ident;
        let addr_tree_info = &field.address_tree_info;
        let output_tree = &field.output_tree;

        let tree_info_ident = format_ident!("{}_tree_info", ident);
        let new_addr_params_ident = format_ident!("{}_new_address_params", ident);
        let compressed_address_ident = format_ident!("{}_compressed_address", ident);
        let compressed_infos_ident = format_ident!("{}_compressed_infos", ident);

        addr_idents.push(quote! { #new_addr_params_ident });

        let acc_ty_path = extract_inner_account_type(&field.ty);
        let acc_expr = if field.is_boxed {
            quote! { &mut **self.#ident }
        } else {
            quote! { &mut *self.#ident }
        };

        blocks.push(quote! {
            let #tree_info_ident = #addr_tree_info;
            let #new_addr_params_ident = #tree_info_ident
                .into_new_address_params_assigned_packed(
                    light_sdk_types::address::AddressSeed(self.#ident.key().to_bytes()),
                    Some(#idx_lit),
                );

            let #compressed_address_ident = light_compressed_account::address::derive_address(
                &self.#ident.key().to_bytes(),
                &cpi_accounts
                    .get_tree_account_info(#new_addr_params_ident.address_merkle_tree_account_index as usize)?
                    .key()
                    .to_bytes(),
                &crate::ID.to_bytes(),
            );

            let #compressed_infos_ident = light_sdk::compressible::prepare_compressed_account_on_init::<#acc_ty_path>(
                &self.#ident.to_account_info(),
                #acc_expr,
                &compression_config_data,
                #compressed_address_ident,
                #new_addr_params_ident,
                #output_tree,
                &cpi_accounts,
                &compression_config_data.address_space,
                false,
            )?;
            all_compressed_infos.push(#compressed_infos_ident);
        });
    }

    (blocks, addr_idents)
}

// NOTE: light_mint functionality has been disabled. The following functions were removed:
// - generate_single_mint_block
// - generate_mint_blocks
// - generate_mint_blocks_with_context
// - generate_mint_blocks_impl
// These require SystemAccountInfos::try_from_remaining_accounts which doesn't exist.
// For mint creation, use light_ctoken_sdk directly.

/// Extract the inner type T from Account<'info, T> or Box<Account<'info, T>>
fn extract_inner_account_type(ty: &syn::Type) -> TokenStream {
    match ty {
        syn::Type::Path(type_path) => {
            let path = &type_path.path;
            if let Some(segment) = path.segments.last() {
                let ident_str = segment.ident.to_string();

                if ident_str == "Account" {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        for arg in &args.args {
                            if let syn::GenericArgument::Type(inner_ty) = arg {
                                return quote! { #inner_ty };
                            }
                        }
                    }
                }

                if ident_str == "Box" {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                            return extract_inner_account_type(inner);
                        }
                    }
                }
            }
            quote! { #ty }
        }
        _ => quote! { #ty },
    }
}

//! Decompress code generation.
//!
//! This module provides the `DecompressBuilder` for generating decompress instruction
//! code including context implementation, processor, entrypoint, accounts struct,
//! and PDA seed provider implementations.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Result, Type};

use super::{
    expr_traversal::transform_expr_for_ctx_seeds,
    parsing::{SeedElement, TokenSeedSpec},
    seed_utils::ctx_fields_to_set,
    variant_enum::PdaCtxSeedInfo,
};
use crate::light_pdas::shared_utils::{is_constant_identifier, qualify_type_with_crate};

// =============================================================================
// DECOMPRESS BUILDER
// =============================================================================

/// Builder for generating decompress instruction code.
///
/// Encapsulates all data needed to generate decompress-related code:
/// context implementation, processor function, instruction entrypoint,
/// accounts struct, and PDA seed provider implementations.
pub(super) struct DecompressBuilder {
    /// PDA context seed information for each variant.
    pda_ctx_seeds: Vec<PdaCtxSeedInfo>,
    /// Token variant identifier (e.g., TokenAccountVariant).
    token_variant_ident: Ident,
    /// Account types that can be decompressed.
    account_types: Vec<Type>,
    /// PDA seed specifications.
    pda_seeds: Option<Vec<TokenSeedSpec>>,
}

impl DecompressBuilder {
    /// Create a new DecompressBuilder with all required configuration.
    ///
    /// # Arguments
    /// * `pda_ctx_seeds` - PDA context seed information for each variant
    /// * `token_variant_ident` - Token variant identifier
    /// * `account_types` - Account types that can be decompressed
    /// * `pda_seeds` - PDA seed specifications
    pub fn new(
        pda_ctx_seeds: Vec<PdaCtxSeedInfo>,
        token_variant_ident: Ident,
        account_types: Vec<Type>,
        pda_seeds: Option<Vec<TokenSeedSpec>>,
    ) -> Self {
        Self {
            pda_ctx_seeds,
            token_variant_ident,
            account_types,
            pda_seeds,
        }
    }

    // -------------------------------------------------------------------------
    // Code Generation Methods
    // -------------------------------------------------------------------------

    /// Generate the decompress context implementation module.
    pub fn generate_context_impl(&self) -> Result<syn::ItemMod> {
        let lifetime: syn::Lifetime = syn::parse_quote!('info);

        let trait_impl =
            crate::light_pdas::account::decompress_context::generate_decompress_context_trait_impl(
                self.pda_ctx_seeds.clone(),
                self.token_variant_ident.clone(),
                lifetime,
            )?;

        Ok(syn::parse_quote! {
            mod __decompress_context_impl {
                use super::*;

                #trait_impl
            }
        })
    }

    /// Generate the processor function for decompress accounts.
    pub fn generate_processor(&self) -> Result<syn::ItemFn> {
        Ok(syn::parse_quote! {
            #[inline(never)]
            pub fn process_decompress_accounts_idempotent<'info>(
                accounts: &DecompressAccountsIdempotent<'info>,
                remaining_accounts: &[solana_account_info::AccountInfo<'info>],
                proof: light_sdk::instruction::ValidityProof,
                compressed_accounts: Vec<LightAccountData>,
                system_accounts_offset: u8,
            ) -> Result<()> {
                light_sdk::interface::process_decompress_accounts_idempotent(
                    accounts,
                    remaining_accounts,
                    compressed_accounts,
                    proof,
                    system_accounts_offset,
                    LIGHT_CPI_SIGNER,
                    &crate::ID,
                    None,
                )
                .map_err(|e: solana_program_error::ProgramError| -> anchor_lang::error::Error { e.into() })
            }
        })
    }

    /// Generate the decompress instruction entrypoint function.
    pub fn generate_entrypoint(&self) -> Result<syn::ItemFn> {
        Ok(syn::parse_quote! {
            #[inline(never)]
            pub fn decompress_accounts_idempotent<'info>(
                ctx: Context<'_, '_, '_, 'info, DecompressAccountsIdempotent<'info>>,
                proof: light_sdk::instruction::ValidityProof,
                compressed_accounts: Vec<LightAccountData>,
                system_accounts_offset: u8,
            ) -> Result<()> {
                __processor_functions::process_decompress_accounts_idempotent(
                    &ctx.accounts,
                    &ctx.remaining_accounts,
                    proof,
                    compressed_accounts,
                    system_accounts_offset,
                )
            }
        })
    }

    /// Generate the decompress accounts struct.
    ///
    /// The accounts struct is the same for all variants since it provides
    /// shared infrastructure for decompression operations. The variant behavior
    /// is determined by the context implementation, not the accounts struct.
    pub fn generate_accounts_struct(&self) -> Result<syn::ItemStruct> {
        Ok(syn::parse_quote! {
            #[derive(Accounts)]
            pub struct DecompressAccountsIdempotent<'info> {
                #[account(mut)]
                pub fee_payer: Signer<'info>,
                /// CHECK: Checked by SDK
                pub config: AccountInfo<'info>,
                /// CHECK: anyone can pay
                #[account(mut)]
                pub rent_sponsor: UncheckedAccount<'info>,
                /// CHECK: optional - only needed if decompressing tokens
                #[account(mut)]
                pub ctoken_rent_sponsor: Option<AccountInfo<'info>>,
                /// CHECK:
                #[account(address = solana_pubkey::pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"))]
                pub light_token_program: Option<UncheckedAccount<'info>>,
                /// CHECK:
                #[account(address = solana_pubkey::pubkey!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy"))]
                pub light_token_cpi_authority: Option<UncheckedAccount<'info>>,
                /// CHECK: Checked by SDK
                pub ctoken_config: Option<UncheckedAccount<'info>>,
            }
        })
    }

    /// Generate PDA seed provider implementations.
    pub fn generate_seed_provider_impls(&self) -> Result<Vec<TokenStream>> {
        let pda_seed_specs = self.pda_seeds.as_ref().ok_or_else(|| {
            let span_source = self
                .account_types
                .first()
                .map(|t| quote::quote!(#t))
                .unwrap_or_else(|| quote::quote!(unknown));
            super::parsing::macro_error!(span_source, "No seed specifications provided")
        })?;

        let mut results = Vec::with_capacity(self.pda_ctx_seeds.len());

        for ctx_info in self.pda_ctx_seeds.iter() {
            let variant_str = ctx_info.variant_name.to_string();
            let spec = pda_seed_specs
                .iter()
                .find(|s| s.variant == variant_str)
                .ok_or_else(|| {
                    super::parsing::macro_error!(
                        &ctx_info.variant_name,
                        "No seed specification for variant '{}'",
                        variant_str
                    )
                })?;

            let ctx_seeds_struct_name = format_ident!("{}CtxSeeds", ctx_info.variant_name);
            let inner_type = qualify_type_with_crate(&ctx_info.inner_type);
            let ctx_fields = &ctx_info.ctx_seed_fields;
            let ctx_fields_decl: Vec<_> = ctx_fields
                .iter()
                .map(|field| {
                    quote! { pub #field: solana_pubkey::Pubkey }
                })
                .collect();

            let ctx_seeds_struct = if ctx_fields.is_empty() {
                quote! {
                    #[derive(Default)]
                    pub struct #ctx_seeds_struct_name;
                }
            } else {
                quote! {
                    #[derive(Default)]
                    pub struct #ctx_seeds_struct_name {
                        #(#ctx_fields_decl),*
                    }
                }
            };

            let params_only_fields = &ctx_info.params_only_seed_fields;
            let seed_derivation = generate_pda_seed_derivation_for_trait_with_ctx_seeds(
                spec,
                ctx_fields,
                &ctx_info.state_field_names,
                params_only_fields,
            )?;

            let has_params_only = !params_only_fields.is_empty();
            let seed_params_impl = if has_params_only {
                quote! {
                    #ctx_seeds_struct

                    impl light_sdk::interface::PdaSeedDerivation<#ctx_seeds_struct_name, SeedParams> for #inner_type {
                        fn derive_pda_seeds_with_accounts(
                            &self,
                            program_id: &solana_pubkey::Pubkey,
                            ctx_seeds: &#ctx_seeds_struct_name,
                            seed_params: &SeedParams,
                        ) -> std::result::Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey), solana_program_error::ProgramError> {
                            #seed_derivation
                        }
                    }
                }
            } else {
                quote! {
                    #ctx_seeds_struct

                    impl light_sdk::interface::PdaSeedDerivation<#ctx_seeds_struct_name, SeedParams> for #inner_type {
                        fn derive_pda_seeds_with_accounts(
                            &self,
                            program_id: &solana_pubkey::Pubkey,
                            ctx_seeds: &#ctx_seeds_struct_name,
                            _seed_params: &SeedParams,
                        ) -> std::result::Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey), solana_program_error::ProgramError> {
                            #seed_derivation
                        }
                    }
                }
            };
            results.push(seed_params_impl);
        }

        Ok(results)
    }
}

// =============================================================================
// PDA SEED DERIVATION (Internal helpers used by DecompressBuilder)
// =============================================================================

/// Generate PDA seed derivation that uses CtxSeeds struct instead of DecompressAccountsIdempotent.
/// Maps ctx.field -> ctx_seeds.field (direct Pubkey access, no Option unwrapping needed)
/// Only maps data.field -> self.field if the field exists on the state struct.
/// For params-only fields, uses seed_params.field instead of skipping.
#[inline(never)]
fn generate_pda_seed_derivation_for_trait_with_ctx_seeds(
    spec: &TokenSeedSpec,
    ctx_seed_fields: &[syn::Ident],
    state_field_names: &std::collections::HashSet<String>,
    params_only_fields: &[(syn::Ident, syn::Type, bool)],
) -> Result<TokenStream> {
    // Build a lookup for params-only field names
    let params_only_names: std::collections::HashSet<String> = params_only_fields
        .iter()
        .map(|(name, _, _)| name.to_string())
        .collect();
    let params_only_has_conversion: std::collections::HashMap<String, bool> = params_only_fields
        .iter()
        .map(|(name, _, has_conv)| (name.to_string(), *has_conv))
        .collect();
    let mut bindings: Vec<TokenStream> = Vec::new();
    let mut seed_refs = Vec::new();

    // Convert ctx_seed_fields to a set for quick lookup
    let ctx_field_names = ctx_fields_to_set(ctx_seed_fields);

    for (i, seed) in spec.seeds.iter().enumerate() {
        match seed {
            SeedElement::Literal(lit) => {
                let value = lit.value();
                seed_refs.push(quote! { #value.as_bytes() });
            }
            SeedElement::Expression(expr) => {
                // Handle byte string literals: b"seed" -> use directly (no .as_bytes())
                if let syn::Expr::Lit(lit_expr) = &**expr {
                    if let syn::Lit::ByteStr(byte_str) = &lit_expr.lit {
                        let bytes = byte_str.value();
                        seed_refs.push(quote! { &[#(#bytes),*] });
                        continue;
                    }
                }

                // Handle uppercase constants (single-segment and multi-segment paths)
                if let syn::Expr::Path(path_expr) = &**expr {
                    if let Some(ident) = path_expr.path.get_ident() {
                        // Single-segment path like AUTH_SEED
                        let ident_str = ident.to_string();
                        if is_constant_identifier(&ident_str) {
                            seed_refs.push(
                                quote! { { let __seed: &[u8] = crate::#ident.as_ref(); __seed } },
                            );
                            continue;
                        }
                    } else if let Some(last_seg) = path_expr.path.segments.last() {
                        // Multi-segment path like crate::AUTH_SEED
                        if is_constant_identifier(&last_seg.ident.to_string()) {
                            let path = &path_expr.path;
                            seed_refs
                                .push(quote! { { let __seed: &[u8] = #path.as_ref(); __seed } });
                            continue;
                        }
                    }
                }

                // Check if this is a data.field expression where the field doesn't exist on state
                // If so, use seed_params.field instead of skipping
                if let Some(field_name) = get_params_only_field_name(expr, state_field_names) {
                    if params_only_names.contains(&field_name) {
                        let field_ident =
                            syn::Ident::new(&field_name, proc_macro2::Span::call_site());
                        let binding_name =
                            syn::Ident::new(&format!("seed_{}", i), proc_macro2::Span::call_site());

                        // Check if this field has a conversion (to_le_bytes, to_be_bytes)
                        let has_conversion = params_only_has_conversion
                            .get(&field_name)
                            .copied()
                            .unwrap_or(false);

                        if has_conversion {
                            // u64 field with to_le_bytes conversion
                            // Must bind bytes to a variable to avoid temporary value dropped while borrowed
                            let bytes_binding_name = syn::Ident::new(
                                &format!("{}_bytes", binding_name),
                                proc_macro2::Span::call_site(),
                            );
                            bindings.push(quote! {
                                let #binding_name = seed_params.#field_ident
                                    .ok_or(solana_program_error::ProgramError::InvalidAccountData)?;
                                let #bytes_binding_name = #binding_name.to_le_bytes();
                            });
                            seed_refs.push(quote! { #bytes_binding_name.as_ref() });
                        } else {
                            // Pubkey field
                            bindings.push(quote! {
                                let #binding_name = seed_params.#field_ident
                                    .ok_or(solana_program_error::ProgramError::InvalidAccountData)?;
                            });
                            seed_refs.push(quote! { #binding_name.as_ref() });
                        }
                        continue;
                    }
                }

                let binding_name =
                    syn::Ident::new(&format!("seed_{}", i), proc_macro2::Span::call_site());
                let mapped_expr =
                    transform_expr_for_ctx_seeds(expr, &ctx_field_names, state_field_names);
                bindings.push(quote! {
                    let #binding_name = #mapped_expr;
                });
                seed_refs.push(quote! { (#binding_name).as_ref() });
            }
        }
    }

    let indices: Vec<usize> = (0..seed_refs.len()).collect();

    Ok(quote! {
        #(#bindings)*
        let seeds: &[&[u8]] = &[#(#seed_refs,)*];
        let (pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, program_id);
        let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
        #(
            seeds_vec.push(seeds[#indices].to_vec());
        )*
        // Avoid vec![bump] macro which expands to box_new allocation
        {
            let mut bump_vec = Vec::with_capacity(1);
            bump_vec.push(bump);
            seeds_vec.push(bump_vec);
        }
        Ok((seeds_vec, pda))
    })
}

/// Get the field name from a params-only seed expression.
/// Returns Some(field_name) if the expression is a data.field where field doesn't exist on state.
fn get_params_only_field_name(
    expr: &syn::Expr,
    state_field_names: &std::collections::HashSet<String>,
) -> Option<String> {
    use crate::light_pdas::shared_utils::is_base_path;

    match expr {
        syn::Expr::Field(field_expr) => {
            if let syn::Member::Named(field_name) = &field_expr.member {
                if is_base_path(&field_expr.base, "data") {
                    let name = field_name.to_string();
                    if !state_field_names.contains(&name) {
                        return Some(name);
                    }
                }
            }
            None
        }
        syn::Expr::MethodCall(method_call) => {
            get_params_only_field_name(&method_call.receiver, state_field_names)
        }
        syn::Expr::Reference(ref_expr) => {
            get_params_only_field_name(&ref_expr.expr, state_field_names)
        }
        _ => None,
    }
}

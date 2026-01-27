//! Decompress code generation.
//!
//! This module provides the `DecompressBuilder` for generating decompress instruction
//! code including context implementation, processor, entrypoint, accounts struct,
//! and PDA seed provider implementations.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Result;

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
    /// PDA seed specifications.
    pda_seeds: Option<Vec<TokenSeedSpec>>,
}

impl DecompressBuilder {
    /// Create a new DecompressBuilder with all required configuration.
    ///
    /// # Arguments
    /// * `pda_ctx_seeds` - PDA context seed information for each variant
    /// * `pda_seeds` - PDA seed specifications
    pub fn new(
        pda_ctx_seeds: Vec<PdaCtxSeedInfo>,
        pda_seeds: Option<Vec<TokenSeedSpec>>,
    ) -> Self {
        Self {
            pda_ctx_seeds,
            pda_seeds,
        }
    }

    // -------------------------------------------------------------------------
    // Code Generation Methods
    // -------------------------------------------------------------------------

    /// Generate the processor function for decompress accounts (v2 interface).
    pub fn generate_processor(&self) -> Result<syn::ItemFn> {
        Ok(syn::parse_quote! {
            #[inline(never)]
            pub fn process_decompress_accounts_idempotent<'info>(
                remaining_accounts: &[solana_account_info::AccountInfo<'info>],
                instruction_data: &[u8],
            ) -> Result<()> {
                light_sdk::interface::process_decompress_pda_accounts_idempotent::<PackedLightAccountVariant>(
                    remaining_accounts,
                    instruction_data,
                    LIGHT_CPI_SIGNER,
                    &crate::ID,
                )
                .map_err(|e: solana_program_error::ProgramError| -> anchor_lang::error::Error { e.into() })
            }
        })
    }

    /// Generate the decompress instruction entrypoint function (v2 interface).
    ///
    /// Accepts `instruction_data: Vec<u8>` as a single parameter.
    /// The SDK client wraps the serialized data in a Vec<u8> (4-byte length prefix),
    /// and Anchor deserializes Vec<u8> correctly with this format.
    pub fn generate_entrypoint(&self) -> Result<syn::ItemFn> {
        Ok(syn::parse_quote! {
            #[inline(never)]
            pub fn decompress_accounts_idempotent<'info>(
                ctx: Context<'_, '_, '_, 'info, DecompressAccountsIdempotent>,
                instruction_data: Vec<u8>,
            ) -> Result<()> {
                __processor_functions::process_decompress_accounts_idempotent(
                    ctx.remaining_accounts,
                    &instruction_data,
                )
            }
        })
    }

    /// Generate the decompress accounts struct.
    ///
    /// Empty struct - all accounts are passed via remaining_accounts
    /// and validated by the SDK's process_decompress_pda_accounts_idempotent.
    pub fn generate_accounts_struct(&self) -> Result<syn::ItemStruct> {
        Ok(syn::parse_quote! {
            #[derive(Accounts)]
            pub struct DecompressAccountsIdempotent {}
        })
    }

    /// Generate PDA seed provider implementations.
    /// Returns empty Vec for mint-only or token-only programs that have no PDA seeds.
    pub fn generate_seed_provider_impls(&self) -> Result<Vec<TokenStream>> {
        // For mint-only or token-only programs, there are no PDA seeds - return empty Vec
        let pda_seed_specs = match self.pda_seeds.as_ref() {
            Some(specs) if !specs.is_empty() => specs,
            _ => {
                // Fail fast if pda_ctx_seeds has variants but pda_seeds is missing
                if !self.pda_ctx_seeds.is_empty() {
                    let variant_names: Vec<_> = self
                        .pda_ctx_seeds
                        .iter()
                        .map(|v| v.variant_name.to_string())
                        .collect();
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        format!(
                            "generate_seed_provider_impls: pda_seeds is None/empty but \
                             pda_ctx_seeds contains {} variant(s): [{}]. \
                             Each pda_ctx_seeds variant requires a corresponding PDA seed \
                             specification in pda_seeds.",
                            self.pda_ctx_seeds.len(),
                            variant_names.join(", ")
                        ),
                    ));
                }
                return Ok(Vec::new());
            }
        };

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
                        // Multi-segment path like crate::AUTH_SEED or <Type as Trait>::CONSTANT
                        if is_constant_identifier(&last_seg.ident.to_string()) {
                            // Use the full ExprPath (not just path) to preserve qself
                            // for type-qualified paths like <SeedHolder as HasSeed>::TRAIT_SEED
                            let full_expr = &**expr;
                            seed_refs
                                .push(quote! { { let __seed: &[u8] = #full_expr.as_ref(); __seed } });
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

                // Strip trailing .as_ref() / .as_bytes() to avoid binding a temporary
                // reference (E0515/E0716). Instead, bind the owned value and call
                // .as_ref() when constructing the seeds array.
                //
                // Before: let seed_0 = crate::id().as_ref();  // ERROR: temporary dropped
                // After:  let seed_0 = crate::id();  seed_0.as_ref()  // OK: owned value lives long enough
                let (stripped_expr, trailing_method) =
                    strip_trailing_ref_method(&mapped_expr);
                let ref_method =
                    trailing_method.unwrap_or_else(|| format_ident!("as_ref"));

                bindings.push(quote! {
                    let #binding_name = #stripped_expr;
                });
                seed_refs.push(quote! { (#binding_name).#ref_method() });
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

/// Strip trailing `.as_ref()` or `.as_bytes()` method call from an expression.
///
/// Returns `(stripped_expr, Some(method_name))` if a trailing method was stripped,
/// or `(original_expr, None)` if no stripping was needed.
///
/// This avoids the E0515/E0716 error where binding a temporary reference:
///   `let seed = crate::id().as_ref();`  // ERROR: temporary value dropped
/// is replaced with:
///   `let seed = crate::id();`           // OK: owned value
///   `seed.as_ref()`                     // borrow from owned
fn strip_trailing_ref_method(expr: &syn::Expr) -> (syn::Expr, Option<syn::Ident>) {
    if let syn::Expr::MethodCall(mc) = expr {
        let method_name = mc.method.to_string();
        if (method_name == "as_ref" || method_name == "as_bytes") && mc.args.is_empty() {
            return ((*mc.receiver).clone(), Some(mc.method.clone()));
        }
    }
    (expr.clone(), None)
}

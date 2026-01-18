//! Decompress code generation.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Result};

use super::{
    expr_traversal::transform_expr_for_ctx_seeds,
    parsing::{InstructionVariant, SeedElement, TokenSeedSpec},
    seed_utils::ctx_fields_to_set,
    variant_enum::PdaCtxSeedInfo,
};
use crate::rentfree::shared_utils::{is_constant_identifier, qualify_type_with_crate};

// =============================================================================
// DECOMPRESS CONTEXT IMPL
// =============================================================================

pub fn generate_decompress_context_impl(
    pda_ctx_seeds: Vec<PdaCtxSeedInfo>,
    token_variant_ident: Ident,
) -> Result<syn::ItemMod> {
    let lifetime: syn::Lifetime = syn::parse_quote!('info);

    let trait_impl =
        crate::rentfree::account::decompress_context::generate_decompress_context_trait_impl(
            pda_ctx_seeds,
            token_variant_ident,
            lifetime,
        )?;

    Ok(syn::parse_quote! {
        mod __decompress_context_impl {
            use super::*;

            #trait_impl
        }
    })
}

// =============================================================================
// DECOMPRESS PROCESSOR
// =============================================================================

pub fn generate_process_decompress_accounts_idempotent() -> Result<syn::ItemFn> {
    Ok(syn::parse_quote! {
        #[inline(never)]
        pub fn process_decompress_accounts_idempotent<'info>(
            accounts: &DecompressAccountsIdempotent<'info>,
            remaining_accounts: &[solana_account_info::AccountInfo<'info>],
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<RentFreeAccountData>,
            system_accounts_offset: u8,
        ) -> Result<()> {
            light_sdk::compressible::process_decompress_accounts_idempotent(
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

// =============================================================================
// DECOMPRESS INSTRUCTION ENTRYPOINT
// =============================================================================

pub fn generate_decompress_instruction_entrypoint() -> Result<syn::ItemFn> {
    Ok(syn::parse_quote! {
        #[inline(never)]
        pub fn decompress_accounts_idempotent<'info>(
            ctx: Context<'_, '_, '_, 'info, DecompressAccountsIdempotent<'info>>,
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<RentFreeAccountData>,
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

// =============================================================================
// DECOMPRESS ACCOUNTS STRUCT
// =============================================================================

#[inline(never)]
pub fn generate_decompress_accounts_struct(variant: InstructionVariant) -> Result<syn::ItemStruct> {
    // Only Mixed variant is supported - PdaOnly and TokenOnly are not implemented
    match variant {
        InstructionVariant::PdaOnly | InstructionVariant::TokenOnly => {
            unreachable!("decompress_accounts_struct only supports Mixed variant")
        }
        InstructionVariant::Mixed => {}
    }

    let account_fields = vec![
        quote! {
            #[account(mut)]
            pub fee_payer: Signer<'info>
        },
        quote! {
            /// CHECK: Checked by SDK
            pub config: AccountInfo<'info>
        },
        quote! {
            /// CHECK: anyone can pay
            #[account(mut)]
            pub rent_sponsor: UncheckedAccount<'info>
        },
        quote! {
            /// CHECK: optional - only needed if decompressing tokens
            #[account(mut)]
            pub ctoken_rent_sponsor: Option<AccountInfo<'info>>
        },
        quote! {
            /// CHECK:
            #[account(address = solana_pubkey::pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"))]
            pub light_token_program: Option<UncheckedAccount<'info>>
        },
        quote! {
            /// CHECK:
            #[account(address = solana_pubkey::pubkey!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy"))]
            pub ctoken_cpi_authority: Option<UncheckedAccount<'info>>
        },
        quote! {
            /// CHECK: Checked by SDK
            pub ctoken_config: Option<UncheckedAccount<'info>>
        },
    ];

    let struct_def = quote! {
        #[derive(Accounts)]
        pub struct DecompressAccountsIdempotent<'info> {
            #(#account_fields,)*
        }
    };

    syn::parse2(struct_def)
}

// =============================================================================
// PDA SEED DERIVATION
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

                // Handle uppercase constants
                if let syn::Expr::Path(path_expr) = &**expr {
                    if let Some(ident) = path_expr.path.get_ident() {
                        let ident_str = ident.to_string();
                        if is_constant_identifier(&ident_str) {
                            seed_refs.push(
                                quote! { { let __seed: &[u8] = crate::#ident.as_ref(); __seed } },
                            );
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

/// Check if a seed expression is a params-only seed (data.field where field doesn't exist on state)
#[allow(dead_code)]
fn is_params_only_seed(
    expr: &syn::Expr,
    state_field_names: &std::collections::HashSet<String>,
) -> bool {
    get_params_only_field_name(expr, state_field_names).is_some()
}

/// Get the field name from a params-only seed expression.
/// Returns Some(field_name) if the expression is a data.field where field doesn't exist on state.
fn get_params_only_field_name(
    expr: &syn::Expr,
    state_field_names: &std::collections::HashSet<String>,
) -> Option<String> {
    use crate::rentfree::shared_utils::is_base_path;

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

// =============================================================================
// PDA SEED PROVIDER IMPLS
// =============================================================================

#[inline(never)]
pub fn generate_pda_seed_provider_impls(
    account_types: &[syn::Type],
    pda_ctx_seeds: &[PdaCtxSeedInfo],
    pda_seeds: &Option<Vec<TokenSeedSpec>>,
) -> Result<Vec<TokenStream>> {
    let pda_seed_specs = pda_seeds.as_ref().ok_or_else(|| {
        // Use first account type for error span, or create a dummy span
        let span_source = account_types
            .first()
            .map(|t| quote::quote!(#t))
            .unwrap_or_else(|| quote::quote!(unknown));
        super::parsing::macro_error!(span_source, "No seed specifications provided")
    })?;

    let mut results = Vec::with_capacity(pda_ctx_seeds.len());

    // Iterate over pda_ctx_seeds which has both variant_name and inner_type
    for ctx_info in pda_ctx_seeds.iter() {
        // Match spec by variant_name (field name based)
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

        // Use variant_name for struct naming (e.g., RecordCtxSeeds)
        let ctx_seeds_struct_name = format_ident!("{}CtxSeeds", ctx_info.variant_name);
        // Use inner_type for the impl (e.g., impl ... for crate::SinglePubkeyRecord)
        // Qualify with crate:: to ensure it's accessible from generated code
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

        // Generate impl for inner_type, but use variant-based struct name
        // Use SeedParams if there are params-only fields, otherwise use ()
        let has_params_only = !params_only_fields.is_empty();
        let seed_params_impl = if has_params_only {
            quote! {
                #ctx_seeds_struct

                impl light_sdk::compressible::PdaSeedDerivation<#ctx_seeds_struct_name, SeedParams> for #inner_type {
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

                impl light_sdk::compressible::PdaSeedDerivation<#ctx_seeds_struct_name, SeedParams> for #inner_type {
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

//! Decompress code generation.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Result};

use super::parsing::{InstructionDataSpec, InstructionVariant, SeedElement, TokenSeedSpec};
use super::variant_enum::PdaCtxSeedInfo;

// =============================================================================
// DECOMPRESS CONTEXT IMPL
// =============================================================================

pub fn generate_decompress_context_impl(
    _variant: InstructionVariant,
    pda_ctx_seeds: Vec<PdaCtxSeedInfo>,
    token_variant_ident: Ident,
) -> Result<syn::ItemMod> {
    let lifetime: syn::Lifetime = syn::parse_quote!('info);

    let trait_impl =
        crate::rentfree::traits::decompress_context::generate_decompress_context_trait_impl(
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

pub fn generate_process_decompress_accounts_idempotent(
    _variant: InstructionVariant,
    _instruction_data: &[InstructionDataSpec],
) -> Result<syn::ItemFn> {
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
                std::option::Option::None::<&()>,
            )
            .map_err(|e: solana_program_error::ProgramError| -> anchor_lang::error::Error { e.into() })
        }
    })
}

// =============================================================================
// DECOMPRESS INSTRUCTION ENTRYPOINT
// =============================================================================

pub fn generate_decompress_instruction_entrypoint(
    _variant: InstructionVariant,
    _instruction_data: &[InstructionDataSpec],
) -> Result<syn::ItemFn> {
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
pub fn generate_decompress_accounts_struct(
    required_accounts: &[String],
    variant: InstructionVariant,
) -> Result<syn::ItemStruct> {
    let mut account_fields = vec![
        quote! {
            #[account(mut)]
            pub fee_payer: Signer<'info>
        },
        quote! {
            /// CHECK: Checked by SDK
            pub config: AccountInfo<'info>
        },
    ];

    match variant {
        InstructionVariant::PdaOnly => {
            unreachable!()
        }
        InstructionVariant::TokenOnly => {
            unreachable!()
        }
        InstructionVariant::Mixed => {
            account_fields.extend(vec![
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
            ]);
        }
    }

    match variant {
        InstructionVariant::TokenOnly => {
            unreachable!()
        }
        InstructionVariant::Mixed => {
            account_fields.extend(vec![
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
            ]);
        }
        InstructionVariant::PdaOnly => {
            unreachable!()
        }
    }

    let standard_fields = [
        "fee_payer",
        "rent_sponsor",
        "ctoken_rent_sponsor",
        "config",
        "light_token_program",
        "ctoken_cpi_authority",
        "ctoken_config",
    ];

    for account_name in required_accounts {
        if !standard_fields.contains(&account_name.as_str()) {
            let account_ident = syn::Ident::new(account_name, proc_macro2::Span::call_site());
            // Mark seed accounts as writable to support CPI calls that may need them writable
            account_fields.push(quote! {
                /// CHECK: optional seed account - may be used in CPIs
                #[account(mut)]
                pub #account_ident: Option<UncheckedAccount<'info>>
            });
        }
    }

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
#[inline(never)]
pub fn generate_pda_seed_derivation_for_trait_with_ctx_seeds(
    spec: &TokenSeedSpec,
    _instruction_data: &[InstructionDataSpec],
    ctx_seed_fields: &[syn::Ident],
) -> Result<TokenStream> {
    let mut bindings: Vec<TokenStream> = Vec::new();
    let mut seed_refs = Vec::new();

    // Convert ctx_seed_fields to a set for quick lookup
    let ctx_field_names: std::collections::HashSet<String> =
        ctx_seed_fields.iter().map(|f| f.to_string()).collect();

    // Recursively rewrite expressions:
    // - `data.<field>` -> `self.<field>` (from unpacked compressed account data)
    // - `ctx.accounts.<account>` -> `ctx_seeds.<account>` (direct Pubkey on CtxSeeds struct)
    // - `ctx.<field>` -> `ctx_seeds.<field>` (direct Pubkey on CtxSeeds struct)
    fn map_pda_expr_to_ctx_seeds(
        expr: &syn::Expr,
        ctx_field_names: &std::collections::HashSet<String>,
    ) -> syn::Expr {
        match expr {
            syn::Expr::Field(field_expr) => {
                if let syn::Member::Named(field_name) = &field_expr.member {
                    // Handle nested field access: ctx.accounts.field_name -> ctx_seeds.field_name
                    if let syn::Expr::Field(nested_field) = &*field_expr.base {
                        if let syn::Member::Named(base_name) = &nested_field.member {
                            if base_name == "accounts" {
                                if let syn::Expr::Path(path) = &*nested_field.base {
                                    if let Some(segment) = path.path.segments.first() {
                                        if segment.ident == "ctx" {
                                            // ctx.accounts.field -> ctx_seeds.field (direct Pubkey)
                                            return syn::parse_quote! { ctx_seeds.#field_name };
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Handle direct field access
                    if let syn::Expr::Path(path) = &*field_expr.base {
                        if let Some(segment) = path.path.segments.first() {
                            if segment.ident == "data" {
                                // data.field -> self.field (from unpacked compressed account data)
                                return syn::parse_quote! { self.#field_name };
                            } else if segment.ident == "ctx" {
                                let field_str = field_name.to_string();
                                if ctx_field_names.contains(&field_str) {
                                    // ctx.field -> ctx_seeds.field (direct Pubkey)
                                    return syn::parse_quote! { ctx_seeds.#field_name };
                                }
                            }
                        }
                    }
                }
                expr.clone()
            }
            syn::Expr::MethodCall(method_call) => {
                let mut new_method_call = method_call.clone();
                new_method_call.receiver = Box::new(map_pda_expr_to_ctx_seeds(
                    &method_call.receiver,
                    ctx_field_names,
                ));
                new_method_call.args = method_call
                    .args
                    .iter()
                    .map(|a| map_pda_expr_to_ctx_seeds(a, ctx_field_names))
                    .collect();
                syn::Expr::MethodCall(new_method_call)
            }
            syn::Expr::Call(call_expr) => {
                let mut new_call_expr = call_expr.clone();
                new_call_expr.args = call_expr
                    .args
                    .iter()
                    .map(|a| map_pda_expr_to_ctx_seeds(a, ctx_field_names))
                    .collect();
                syn::Expr::Call(new_call_expr)
            }
            syn::Expr::Reference(ref_expr) => {
                let mut new_ref_expr = ref_expr.clone();
                new_ref_expr.expr =
                    Box::new(map_pda_expr_to_ctx_seeds(&ref_expr.expr, ctx_field_names));
                syn::Expr::Reference(new_ref_expr)
            }
            _ => expr.clone(),
        }
    }

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
                        if ident_str.chars().all(|c| c.is_uppercase() || c == '_') {
                            seed_refs.push(
                                quote! { { let __seed: &[u8] = crate::#ident.as_ref(); __seed } },
                            );
                            continue;
                        }
                    }
                }

                let binding_name =
                    syn::Ident::new(&format!("seed_{}", i), proc_macro2::Span::call_site());
                let mapped_expr = map_pda_expr_to_ctx_seeds(expr, &ctx_field_names);
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
        seeds_vec.push(vec![bump]);
        Ok((seeds_vec, pda))
    })
}

// =============================================================================
// PDA SEED PROVIDER IMPLS
// =============================================================================

#[inline(never)]
pub fn generate_pda_seed_provider_impls(
    account_types: &[Ident],
    pda_ctx_seeds: &[PdaCtxSeedInfo],
    pda_seeds: &Option<Vec<TokenSeedSpec>>,
    instruction_data: &[InstructionDataSpec],
) -> Result<Vec<TokenStream>> {
    account_types
        .iter()
        .zip(pda_ctx_seeds.iter())
        .map(|(name, ctx_info)| {
            let name_str = name.to_string();
            let spec = if let Some(ref pda_seed_specs) = pda_seeds {
                pda_seed_specs
                    .iter()
                    .find(|s| s.variant == name_str)
                    .ok_or_else(|| {
                        super::parsing::macro_error!(
                            name,
                            "No seed specification for account type '{}'",
                            name_str
                        )
                    })?
            } else {
                return Err(super::parsing::macro_error!(
                    name,
                    "No seed specifications provided"
                ));
            };

            let ctx_seeds_struct_name = format_ident!("{}CtxSeeds", name);
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

            let seed_derivation =
                generate_pda_seed_derivation_for_trait_with_ctx_seeds(spec, instruction_data, ctx_fields)?;
            Ok(quote! {
                #ctx_seeds_struct

                impl light_sdk::compressible::PdaSeedDerivation<#ctx_seeds_struct_name, ()> for #name {
                    fn derive_pda_seeds_with_accounts(
                        &self,
                        program_id: &solana_pubkey::Pubkey,
                        ctx_seeds: &#ctx_seeds_struct_name,
                        _seed_params: &(),
                    ) -> std::result::Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey), solana_program_error::ProgramError> {
                        #seed_derivation
                    }
                }
            })
        })
        .collect()
}

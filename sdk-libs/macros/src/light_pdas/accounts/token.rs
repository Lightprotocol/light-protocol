//! Light token account code generation.
//!
//! This module handles code generation for token account and ATA CPI invocations.
//! Parsing is handled by `light_account.rs`.
//!
//! ## Code Generation
//!
//! Token accounts and ATAs are created in `LightPreInit` (before instruction logic)
//! so they are available for use during the instruction handler (transfers, etc.).
//!
//! - **Token Accounts**: Use `CreateTokenAccountCpi` with PDA signing
//! - **ATAs**: Use `CreateTokenAtaCpi` with `idempotent()` builder
//!
//! ## Requirements
//!
//! Programs using `#[light_account(init, token, ...)]` must have a `crate::ID`
//! constant, which is the standard pattern when using Anchor's `declare_id!` macro.
//! The generated code passes `&crate::ID` to `CreateTokenAccountCpi::rent_free()`
//! for PDA signing verification.

use proc_macro2::TokenStream;
use quote::quote;

use super::{
    light_account::{AtaField, TokenAccountField},
    mint::InfraRefs,
};

/// Generate token account creation CPI code for a single token account field.
///
/// Generated code uses `CreateTokenAccountCpi` with rent-free mode and PDA signing.
///
/// Bump handling:
/// - If `bump` parameter is provided, uses that value
/// - If `bump` is not provided, auto-derives using `Pubkey::find_program_address()`
/// - Bump is always appended as the final seed in the signer seeds
#[allow(dead_code)]
pub(super) fn generate_token_account_cpi(
    field: &TokenAccountField,
    infra: &InfraRefs,
) -> Option<TokenStream> {
    // Only generate creation code if has_init is true
    if !field.has_init {
        return None;
    }

    let field_ident = &field.field_ident;
    let light_token_config = &infra.light_token_config;
    let light_token_rent_sponsor = &infra.light_token_rent_sponsor;
    let fee_payer = &infra.fee_payer;

    // Generate token account PDA seeds array from parsed seeds (WITHOUT bump - bump is added separately)
    // These are the seeds for the token account itself (for PDA signing), NOT the authority seeds.
    // Bind each seed to a local variable first, then call .as_ref() to avoid
    // temporary lifetime issues (e.g., self.mint.key() creates a Pubkey that
    // would be dropped before .as_ref() completes if done in one expression)
    //
    // User provides expressions WITHOUT bump in the array:
    //   seeds = [VAULT_SEED, self.mint.key()]
    // Generates:
    //   let __seed_0 = VAULT_SEED; let __seed_0_ref: &[u8] = __seed_0.as_ref();
    //   let __seed_1 = self.mint.key(); let __seed_1_ref: &[u8] = __seed_1.as_ref();
    //   // bump is auto-derived or provided via bump parameter
    //   &[__seed_0_ref, __seed_1_ref, &[bump]]
    let token_seeds = &field.seeds;
    let seed_bindings: Vec<TokenStream> = token_seeds
        .iter()
        .enumerate()
        .map(|(i, seed)| {
            let val_name =
                syn::Ident::new(&format!("__seed_{}", i), proc_macro2::Span::call_site());
            let ref_name =
                syn::Ident::new(&format!("__seed_{}_ref", i), proc_macro2::Span::call_site());
            quote! {
                let #val_name = #seed;
                let #ref_name: &[u8] = #val_name.as_ref();
            }
        })
        .collect();
    let seed_refs: Vec<TokenStream> = (0..token_seeds.len())
        .map(|i| {
            let ref_name =
                syn::Ident::new(&format!("__seed_{}_ref", i), proc_macro2::Span::call_site());
            quote! { #ref_name }
        })
        .collect();

    // Get bump - either from parameter or auto-derive using find_program_address
    let bump_derivation = field
        .bump
        .as_ref()
        .map(|b| quote! { let __bump: u8 = #b; })
        .unwrap_or_else(|| {
            // Auto-derive bump from seeds
            if token_seeds.is_empty() {
                quote! {
                    let __bump: u8 = {
                        let (_, bump) = solana_pubkey::Pubkey::find_program_address(&[], &solana_pubkey::Pubkey::from(crate::LIGHT_CPI_SIGNER.program_id));
                        bump
                    };
                }
            } else {
                quote! {
                    let __bump: u8 = {
                        let seeds: &[&[u8]] = &[#(#seed_refs),*];
                        let (_, bump) = solana_pubkey::Pubkey::find_program_address(seeds, &solana_pubkey::Pubkey::from(crate::LIGHT_CPI_SIGNER.program_id));
                        bump
                    };
                }
            }
        });

    // Build seeds array with bump appended as final seed
    let seeds_array_expr = if token_seeds.is_empty() {
        quote! { &[&__bump_slice[..]] }
    } else {
        quote! { &[#(#seed_refs,)* &__bump_slice[..]] }
    };

    // Get mint binding from field or default
    let mint_binding = field
        .mint
        .as_ref()
        .map(|m| quote! { let __mint_info = self.#m.to_account_info(); })
        .unwrap_or_else(|| quote! { let __mint_info = self.mint.to_account_info(); });

    // owner is [u8; 32] - the owner of the token account
    let owner_expr = field
        .owner
        .as_ref()
        .map(|o| quote! { self.#o.to_account_info().key.to_bytes() })
        .unwrap_or_else(|| quote! { self.fee_payer.to_account_info().key.to_bytes() });

    Some(quote! {
        // Create token account: #field_ident
        {
            use light_account::CreateTokenAccountCpi;

            // Bind seeds to local variables to extend temporary lifetimes
            #(#seed_bindings)*

            // Get bump - either provided or auto-derived
            #bump_derivation
            let __bump_slice: [u8; 1] = [__bump];
            let __token_account_seeds: &[&[u8]] = #seeds_array_expr;

            // Bind account infos to local variables so we can pass references
            let __payer_info = self.#fee_payer.to_account_info();
            let __account_info = self.#field_ident.to_account_info();
            #mint_binding
            let __config_info = self.#light_token_config.to_account_info();
            let __sponsor_info = self.#light_token_rent_sponsor.to_account_info();

            CreateTokenAccountCpi {
                payer: &__payer_info,
                account: &__account_info,
                mint: &__mint_info,
                owner: #owner_expr,
            }
            .rent_free(
                &__config_info,
                &__sponsor_info,
                &__system_program,
                &crate::LIGHT_CPI_SIGNER.program_id,
            )
            .invoke_signed(__token_account_seeds)?;
        }
    })
}

/// Generate ATA creation CPI code for a single ATA field.
///
/// Generated code uses `CreateTokenAtaCpi` builder with rent-free mode.
#[allow(dead_code)]
pub(super) fn generate_ata_cpi(field: &AtaField, infra: &InfraRefs) -> Option<TokenStream> {
    // Only generate creation code if has_init is true
    if !field.has_init {
        return None;
    }

    let field_ident = &field.field_ident;
    let owner = &field.owner;
    let mint = &field.mint;
    let light_token_config = &infra.light_token_config;
    let light_token_rent_sponsor = &infra.light_token_rent_sponsor;
    let fee_payer = &infra.fee_payer;

    Some(quote! {
        // Create ATA: #field_ident
        {
            use light_account::CreateTokenAtaCpi;

            // Bind account infos to local variables so we can pass references
            let __payer_info = self.#fee_payer.to_account_info();
            let __owner_info = self.#owner.to_account_info();
            let __mint_info = self.#mint.to_account_info();
            let __ata_info = self.#field_ident.to_account_info();
            let __config_info = self.#light_token_config.to_account_info();
            let __sponsor_info = self.#light_token_rent_sponsor.to_account_info();

            CreateTokenAtaCpi {
                payer: &__payer_info,
                owner: &__owner_info,
                mint: &__mint_info,
                ata: &__ata_info,
            }
            .idempotent()
            .rent_free(
                &__config_info,
                &__sponsor_info,
                &__system_program,
            )
            .invoke()?;
        }
    })
}

/// Builder for generating finalize code for token accounts and ATAs.
pub(super) struct TokenAccountsBuilder<'a> {
    token_account_fields: &'a [TokenAccountField],
    ata_fields: &'a [AtaField],
    infra: &'a InfraRefs,
}

impl<'a> TokenAccountsBuilder<'a> {
    /// Create a new builder.
    pub fn new(
        token_account_fields: &'a [TokenAccountField],
        ata_fields: &'a [AtaField],
        infra: &'a InfraRefs,
    ) -> Self {
        Self {
            token_account_fields,
            ata_fields,
            infra,
        }
    }

    /// Check if any token accounts or ATAs need to be created.
    pub fn needs_creation(&self) -> bool {
        self.token_account_fields.iter().any(|f| f.has_init)
            || self.ata_fields.iter().any(|f| f.has_init)
    }

    /// Generate token account and ATA creation code for pre_init.
    ///
    /// Returns None if no token accounts or ATAs need to be created.
    /// Otherwise returns the CPI code (without Ok() return).
    pub fn generate_pre_init_token_creation(&self) -> Option<TokenStream> {
        if !self.needs_creation() {
            return None;
        }

        // Generate token account creation code
        let token_account_cpis: Vec<TokenStream> = self
            .token_account_fields
            .iter()
            .filter_map(|f| generate_token_account_cpi(f, self.infra))
            .collect();

        // Generate ATA creation code
        let ata_cpis: Vec<TokenStream> = self
            .ata_fields
            .iter()
            .filter_map(|f| generate_ata_cpi(f, self.infra))
            .collect();

        Some(quote! {
            // Get system program from the struct's system_program field
            let __system_program = self.system_program.to_account_info();

            // Create token accounts (in pre_init so they're available for instruction logic)
            #(#token_account_cpis)*

            // Create ATAs (in pre_init so they're available for instruction logic)
            #(#ata_cpis)*
        })
    }
}

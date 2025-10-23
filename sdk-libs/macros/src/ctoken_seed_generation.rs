//! Shared CToken seed provider generation logic.
use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, Result};

use crate::compressible_instructions::{SeedElement, TokenSeedSpec};

/// Generate CTokenAccountVariant enum from token seed specifications.
pub fn generate_ctoken_account_variant_enum(token_seeds: &[TokenSeedSpec]) -> Result<TokenStream> {
    let variants = token_seeds.iter().enumerate().map(|(index, spec)| {
        let variant_name = &spec.variant;
        let index_u8 = index as u8;
        quote! {
            #variant_name = #index_u8,
        }
    });

    Ok(quote! {
        /// Auto-generated CTokenAccountVariant enum from token seed specifications
        #[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy)]
        #[repr(u8)]
        pub enum CTokenAccountVariant {
            #(#variants)*
        }
    })
}

/// Generate CTokenSeedProvider implementation from token seed specifications.
///
/// This is the core logic shared by both the main macro and any future derives.
pub fn generate_ctoken_seed_provider_implementation(
    token_seeds: &[TokenSeedSpec],
) -> Result<TokenStream> {
    let mut get_seeds_match_arms = Vec::new();
    let mut get_authority_seeds_match_arms = Vec::new();

    for spec in token_seeds {
        let variant_name = &spec.variant;

        // Skip ATA variants
        if spec.is_ata {
            let get_seeds_arm = quote! {
                CTokenAccountVariant::#variant_name => {
                    Err(anchor_lang::prelude::ProgramError::Custom(
                        CompressibleInstructionError::AtaDoesNotUseSeedDerivation.into()
                    ).into())
                }
            };
            get_seeds_match_arms.push(get_seeds_arm);

            let authority_arm = quote! {
                CTokenAccountVariant::#variant_name => {
                    Err(anchor_lang::prelude::ProgramError::Custom(
                        CompressibleInstructionError::AtaDoesNotUseSeedDerivation.into()
                    ).into())
                }
            };
            get_authority_seeds_match_arms.push(authority_arm);
            continue;
        }

        // Generate token account seeds
        let mut token_bindings = Vec::new();
        let mut token_seed_refs = Vec::new();

        for (i, seed) in spec.seeds.iter().enumerate() {
            match seed {
                SeedElement::Literal(lit) => {
                    let value = lit.value();
                    token_seed_refs.push(quote! { #value.as_bytes() });
                }
                SeedElement::Expression(expr) => {
                    // Check for uppercase consts
                    if let syn::Expr::Path(path_expr) = &**expr {
                        if let Some(ident) = path_expr.path.get_ident() {
                            let ident_str = ident.to_string();
                            if ident_str.chars().all(|c| c.is_uppercase() || c == '_') {
                                // Special handling for LIGHT_CPI_SIGNER - use .cpi_signer field
                                if ident_str == "LIGHT_CPI_SIGNER" {
                                    token_seed_refs.push(quote! { #ident.cpi_signer.as_ref() });
                                } else {
                                    token_seed_refs.push(quote! { #ident.as_bytes() });
                                }
                                continue;
                            }
                        }
                    }

                    let mut handled = false;
                    if let syn::Expr::Field(field_expr) = &**expr {
                        if let syn::Member::Named(field_name) = &field_expr.member {
                            if let syn::Expr::Field(nested_field) = &*field_expr.base {
                                if let syn::Member::Named(base_name) = &nested_field.member {
                                    if base_name == "accounts" {
                                        if let syn::Expr::Path(path) = &*nested_field.base {
                                            if let Some(segment) = path.path.segments.first() {
                                                if segment.ident == "ctx" {
                                                    let binding_name = syn::Ident::new(
                                                        &format!("seed_{}", i),
                                                        expr.span(),
                                                    );
                                                    let field_name_str = field_name.to_string();
                                                    let is_standard_field = matches!(
                                                        field_name_str.as_str(),
                                                        "fee_payer"
                                                            | "rent_payer"
                                                            | "config"
                                                            | "rent_sponsor"
                                                            | "ctoken_rent_sponsor"
                                                            | "ctoken_program"
                                                            | "ctoken_cpi_authority"
                                                            | "ctoken_config"
                                                            | "compression_authority"
                                                            | "ctoken_compression_authority"
                                                    );
                                                    if is_standard_field {
                                                        token_bindings.push(quote! {
                                                            let #binding_name = ctx.accounts.#field_name.key();
                                                        });
                                                    } else {
                                                        token_bindings.push(quote! {
                                                            let #binding_name = ctx.accounts.#field_name
                                                                .as_ref()
                                                                .ok_or_else(|| -> anchor_lang::error::Error {
                                                                    anchor_lang::prelude::ProgramError::Custom(
                                                                        CompressibleInstructionError::MissingSeedAccount.into()
                                                                    ).into()
                                                                })?
                                                                .key();
                                                        });
                                                    }
                                                    token_seed_refs
                                                        .push(quote! { #binding_name.as_ref() });
                                                    handled = true;
                                                }
                                            }
                                        }
                                    }
                                }
                            } else if let syn::Expr::Path(path) = &*field_expr.base {
                                if let Some(segment) = path.path.segments.first() {
                                    if segment.ident == "ctx" {
                                        let binding_name =
                                            syn::Ident::new(&format!("seed_{}", i), expr.span());
                                        let field_name_str = field_name.to_string();
                                        let is_standard_field = matches!(
                                            field_name_str.as_str(),
                                            "fee_payer"
                                                | "rent_payer"
                                                | "config"
                                                | "rent_sponsor"
                                                | "ctoken_rent_sponsor"
                                                | "ctoken_program"
                                                | "ctoken_cpi_authority"
                                                | "ctoken_config"
                                                | "compression_authority"
                                                | "ctoken_compression_authority"
                                        );
                                        if is_standard_field {
                                            token_bindings.push(quote! {
                                                let #binding_name = ctx.accounts.#field_name.key();
                                            });
                                        } else {
                                            token_bindings.push(quote! {
                                                let #binding_name = ctx.accounts.#field_name
                                                    .as_ref()
                                                    .ok_or_else(|| -> anchor_lang::error::Error {
                                                        anchor_lang::prelude::ProgramError::Custom(
                                                            CompressibleInstructionError::MissingSeedAccount.into()
                                                        ).into()
                                                    })?
                                                    .key();
                                            });
                                        }
                                        token_seed_refs.push(quote! { #binding_name.as_ref() });
                                        handled = true;
                                    }
                                }
                            }
                        }
                    }

                    if !handled {
                        token_seed_refs.push(quote! { (#expr).as_ref() });
                    }
                }
            }
        }

        let get_seeds_arm = quote! {
            CTokenAccountVariant::#variant_name => {
                #(#token_bindings)*
                let seeds: &[&[u8]] = &[#(#token_seed_refs),*];
                let (token_account_pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, &crate::ID);
                let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
                seeds_vec.extend(seeds.iter().map(|s| s.to_vec()));
                seeds_vec.push(vec![bump]);
                Ok((seeds_vec, token_account_pda))
            }
        };
        get_seeds_match_arms.push(get_seeds_arm);

        // Generate authority seeds
        if let Some(authority_seeds) = &spec.authority {
            let mut auth_bindings: Vec<TokenStream> = Vec::new();
            let mut auth_seed_refs = Vec::new();

            for (i, authority_seed) in authority_seeds.iter().enumerate() {
                match authority_seed {
                    SeedElement::Literal(lit) => {
                        let value = lit.value();
                        auth_seed_refs.push(quote! { #value.as_bytes() });
                    }
                    SeedElement::Expression(expr) => {
                        let mut handled = false;
                        match &**expr {
                            syn::Expr::Field(field_expr) => {
                                if let syn::Member::Named(field_name) = &field_expr.member {
                                    if let syn::Expr::Field(nested_field) = &*field_expr.base {
                                        if let syn::Member::Named(base_name) = &nested_field.member
                                        {
                                            if base_name == "accounts" {
                                                if let syn::Expr::Path(path) = &*nested_field.base {
                                                    if let Some(segment) =
                                                        path.path.segments.first()
                                                    {
                                                        if segment.ident == "ctx" {
                                                            let binding_name = syn::Ident::new(
                                                                &format!("authority_seed_{}", i),
                                                                expr.span(),
                                                            );
                                                            let field_name_str =
                                                                field_name.to_string();
                                                            let is_standard_field = matches!(
                                                                field_name_str.as_str(),
                                                                "fee_payer" | "rent_payer" | "config" | "rent_sponsor"
                                                                    | "ctoken_rent_sponsor" | "ctoken_program"
                                                                    | "ctoken_cpi_authority" | "ctoken_config"
                                                                    | "compression_authority" | "ctoken_compression_authority"
                                                            );
                                                            if is_standard_field {
                                                                auth_bindings.push(quote! {
                                                                    let #binding_name = ctx.accounts.#field_name.key();
                                                                });
                                                            } else {
                                                                auth_bindings.push(quote! {
                                                                    let #binding_name = ctx.accounts.#field_name
                                                                        .as_ref()
                                                                        .ok_or_else(|| -> anchor_lang::error::Error {
                                                                            anchor_lang::prelude::ProgramError::Custom(
                                                                                CompressibleInstructionError::MissingSeedAccount.into()
                                                                            ).into()
                                                                        })?
                                                                        .key();
                                                                });
                                                            }
                                                            auth_seed_refs.push(
                                                                quote! { #binding_name.as_ref() },
                                                            );
                                                            handled = true;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else if let syn::Expr::Path(path) = &*field_expr.base {
                                        if let Some(segment) = path.path.segments.first() {
                                            if segment.ident == "ctx" {
                                                let binding_name = syn::Ident::new(
                                                    &format!("authority_seed_{}", i),
                                                    expr.span(),
                                                );
                                                let field_name_str = field_name.to_string();
                                                let is_standard_field = matches!(
                                                    field_name_str.as_str(),
                                                    "fee_payer"
                                                        | "rent_payer"
                                                        | "config"
                                                        | "rent_sponsor"
                                                        | "ctoken_rent_sponsor"
                                                        | "ctoken_program"
                                                        | "ctoken_cpi_authority"
                                                        | "ctoken_config"
                                                        | "compression_authority"
                                                        | "ctoken_compression_authority"
                                                );
                                                if is_standard_field {
                                                    auth_bindings.push(quote! {
                                                        let #binding_name = ctx.accounts.#field_name.key();
                                                    });
                                                } else {
                                                    auth_bindings.push(quote! {
                                                        let #binding_name = ctx.accounts.#field_name
                                                            .as_ref()
                                                            .ok_or_else(|| -> anchor_lang::error::Error {
                                                                anchor_lang::prelude::ProgramError::Custom(
                                                                    CompressibleInstructionError::MissingSeedAccount.into()
                                                                ).into()
                                                            })?
                                                            .key();
                                                    });
                                                }
                                                auth_seed_refs
                                                    .push(quote! { #binding_name.as_ref() });
                                                handled = true;
                                            }
                                        }
                                    }
                                }
                            }
                            syn::Expr::MethodCall(_mc) => {
                                auth_seed_refs.push(quote! { (#expr).as_ref() });
                                handled = true;
                            }
                            syn::Expr::Path(path_expr) => {
                                if let Some(ident) = path_expr.path.get_ident() {
                                    let ident_str = ident.to_string();
                                    if ident_str.chars().all(|c| c.is_uppercase() || c == '_') {
                                        // Special handling for LIGHT_CPI_SIGNER - use .cpi_signer field
                                        if ident_str == "LIGHT_CPI_SIGNER" {
                                            auth_seed_refs
                                                .push(quote! { #ident.cpi_signer.as_ref() });
                                        } else {
                                            auth_seed_refs.push(quote! { #ident.as_bytes() });
                                        }
                                        handled = true;
                                    }
                                }
                            }
                            _ => {}
                        }

                        if !handled {
                            auth_seed_refs.push(quote! { (#expr).as_ref() });
                        }
                    }
                }
            }

            let authority_arm = quote! {
                CTokenAccountVariant::#variant_name => {
                    #(#auth_bindings)*
                    let seeds: &[&[u8]] = &[#(#auth_seed_refs),*];
                    let (authority_pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, &crate::ID);
                    let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
                    seeds_vec.extend(seeds.iter().map(|s| s.to_vec()));
                    seeds_vec.push(vec![bump]);
                    Ok((seeds_vec, authority_pda))
                }
            };
            get_authority_seeds_match_arms.push(authority_arm);
        } else {
            let authority_arm = quote! {
                CTokenAccountVariant::#variant_name => {
                    Err(anchor_lang::prelude::ProgramError::Custom(
                        CompressibleInstructionError::MissingSeedAccount.into()
                    ).into())
                }
            };
            get_authority_seeds_match_arms.push(authority_arm);
        }
    }

    Ok(quote! {
        /// Auto-generated CTokenSeedProvider implementation
        impl ctoken_seed_system::CTokenSeedProvider for CTokenAccountVariant {
            fn get_seeds<'a, 'info>(
                &self,
                ctx: &ctoken_seed_system::CTokenSeedContext<'a, 'info>,
            ) -> Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey)> {
                match self {
                    #(#get_seeds_match_arms)*
                    _ => Err(anchor_lang::prelude::ProgramError::Custom(
                        CompressibleInstructionError::MissingSeedAccount.into()
                    ).into())
                }
            }

            fn get_authority_seeds<'a, 'info>(
                &self,
                ctx: &ctoken_seed_system::CTokenSeedContext<'a, 'info>,
            ) -> Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey)> {
                match self {
                    #(#get_authority_seeds_match_arms)*
                    _ => Err(anchor_lang::prelude::ProgramError::Custom(
                        CompressibleInstructionError::MissingSeedAccount.into()
                    ).into())
                }
            }
        }
    })
}

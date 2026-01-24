//! DecompressContext trait generation.
//!
//! Generates the implementation of the DecompressContext trait for the
//! DecompressAccountsIdempotent struct. This uses a zero-allocation two-pass approach:
//! - Pass 1 (collect_layout_and_tokens): Count PDAs, collect output_data_lens, collect tokens
//! - Pass 2 (create_and_write_pda): Create PDA on Solana, return data for zero-copy buffer

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Result};

// Re-export from variant_enum for convenience
pub use crate::light_pdas::program::variant_enum::PdaCtxSeedInfo;
use crate::light_pdas::shared_utils::{
    make_packed_type, make_packed_variant_name, qualify_type_with_crate,
};

pub fn generate_decompress_context_trait_impl(
    pda_ctx_seeds: Vec<PdaCtxSeedInfo>,
    token_variant_ident: Ident,
    lifetime: syn::Lifetime,
) -> Result<TokenStream> {
    // Generate match arms for collect_layout_and_tokens - count PDAs that need decompression
    let collect_layout_pda_arms: Vec<_> = pda_ctx_seeds
        .iter()
        .map(|info| {
            let variant_name = &info.variant_name;
            let packed_variant_name = make_packed_variant_name(variant_name);
            quote! {
                LightAccountVariant::#packed_variant_name { .. } => {
                    // PDA variant: only count if not already initialized (idempotent check)
                    if solana_accounts[i].data_is_empty() {
                        pda_indices[pda_count] = i;
                        pda_count += 1;
                    }
                }
                LightAccountVariant::#variant_name { .. } => {
                    return std::result::Result::Err(light_sdk::error::LightSdkError::UnexpectedUnpackedVariant.into());
                }
            }
        })
        .collect();

    // Generate match arms for create_and_write_pda - unpack, derive seeds, create PDA, return data
    let create_pda_match_arms: Vec<_> = pda_ctx_seeds
        .iter()
        .map(|info| {
            let variant_name = &info.variant_name;
            let inner_type = qualify_type_with_crate(&info.inner_type);
            let packed_variant_name = make_packed_variant_name(variant_name);
            let packed_inner_type = make_packed_type(&info.inner_type)
                .expect("inner_type should be a valid type path");
            let ctx_seeds_struct_name = format_ident!("{}CtxSeeds", variant_name);
            let ctx_fields = &info.ctx_seed_fields;
            let params_only_fields = &info.params_only_seed_fields;

            // Generate pattern to extract idx fields from packed variant
            let idx_field_patterns: Vec<_> = ctx_fields.iter().map(|field| {
                let idx_field = format_ident!("{}_idx", field);
                quote! { #idx_field }
            }).collect();
            // Generate pattern to extract params-only fields from packed variant
            let params_field_patterns: Vec<_> = params_only_fields.iter().map(|(field, _, _)| {
                quote! { #field }
            }).collect();
            // Generate code to resolve idx fields to Pubkeys
            // Note: when matching on &compressed_data.data, idx fields are references, so we dereference
            let resolve_ctx_seeds: Vec<_> = ctx_fields.iter().map(|field| {
                let idx_field = format_ident!("{}_idx", field);
                quote! {
                    let #field = *post_system_accounts
                        .get(*#idx_field as usize)
                        .ok_or(solana_program_error::ProgramError::InvalidAccountData)?
                        .key;
                }
            }).collect();
            // Generate CtxSeeds struct construction
            let ctx_seeds_construction = if ctx_fields.is_empty() {
                quote! { let ctx_seeds = #ctx_seeds_struct_name; }
            } else {
                let field_inits: Vec<_> = ctx_fields.iter().map(|field| {
                    quote! { #field }
                }).collect();
                quote! { let ctx_seeds = #ctx_seeds_struct_name { #(#field_inits),* }; }
            };
            // Generate SeedParams update with params-only field values
            // Note: when matching on &compressed_data.data, params fields are references, so we dereference
            let seed_params_update = if params_only_fields.is_empty() {
                quote! {}
            } else {
                let field_inits: Vec<_> = params_only_fields.iter().map(|(field, _, _)| {
                    quote! { #field: std::option::Option::Some(*#field) }
                }).collect();
                quote! { variant_seed_params = SeedParams { #(#field_inits,)* ..Default::default() }; }
            };

            quote! {
                LightAccountVariant::#packed_variant_name { data: packed, #(#idx_field_patterns,)* #(#params_field_patterns,)* .. } => {
                    #(#resolve_ctx_seeds)*
                    #ctx_seeds_construction
                    #seed_params_update

                    // Unpack the data
                    let data: #inner_type = <#packed_inner_type as light_sdk::interface::Unpack>::unpack(&packed, post_system_accounts)?;

                    // Use helper function to derive seeds, verify PDA, create account, and write to zero-copy buffer
                    // Pass data and compressed_meta by reference to reduce caller stack usage
                    light_sdk::interface::derive_verify_create_and_write_pda::<#inner_type, _, _>(
                        &program_id,
                        &data,
                        &ctx_seeds,
                        seed_params,
                        &variant_seed_params,
                        compressed_meta,
                        address_space,
                        solana_account,
                        &*self.rent_sponsor,
                        cpi_accounts,
                        zc_info,
                    )
                }
                LightAccountVariant::#variant_name { .. } => {
                    return std::result::Result::Err(light_sdk::error::LightSdkError::UnexpectedUnpackedVariant.into());
                }
            }
        })
        .collect();

    // For mint-only programs (no PDA variants), add an arm for the Empty variant
    let empty_variant_arm_collect = if pda_ctx_seeds.is_empty() {
        quote! {
            LightAccountVariant::Empty => {
                return std::result::Result::Err(solana_program_error::ProgramError::InvalidAccountData);
            }
        }
    } else {
        quote! {}
    };

    let empty_variant_arm_create = if pda_ctx_seeds.is_empty() {
        quote! {
            LightAccountVariant::Empty => {
                return std::result::Result::Err(solana_program_error::ProgramError::InvalidAccountData);
            }
        }
    } else {
        quote! {}
    };

    let packed_token_variant_ident = format_ident!("Packed{}", token_variant_ident);

    Ok(quote! {
        impl<#lifetime> light_sdk::interface::DecompressContext<#lifetime> for DecompressAccountsIdempotent<#lifetime> {
            type CompressedData = LightAccountData;
            type PackedTokenData = light_token::compat::PackedCTokenData<#packed_token_variant_ident>;
            type CompressedMeta = light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
            type SeedParams = SeedParams;

            fn fee_payer(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                &*self.fee_payer
            }

            fn config(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                &self.config
            }

            fn rent_sponsor(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                &self.rent_sponsor
            }

            fn token_rent_sponsor(&self) -> std::option::Option<&solana_account_info::AccountInfo<#lifetime>> {
                self.ctoken_rent_sponsor.as_ref()
            }

            fn token_program(&self) -> std::option::Option<&solana_account_info::AccountInfo<#lifetime>> {
                self.light_token_program.as_ref().map(|a| &**a)
            }

            fn token_cpi_authority(&self) -> std::option::Option<&solana_account_info::AccountInfo<#lifetime>> {
                self.light_token_cpi_authority.as_ref().map(|a| &**a)
            }

            fn token_config(&self) -> std::option::Option<&solana_account_info::AccountInfo<#lifetime>> {
                self.ctoken_config.as_ref().map(|a| &**a)
            }

            #[allow(clippy::type_complexity)]
            fn collect_layout_and_tokens(
                &self,
                compressed_accounts: &[Self::CompressedData],
                solana_accounts: &[solana_account_info::AccountInfo<#lifetime>],
                pda_indices: &mut [usize; light_sdk::interface::MAX_DECOMPRESS_ACCOUNTS],
            ) -> std::result::Result<(usize, Vec<(Self::PackedTokenData, Self::CompressedMeta)>), solana_program_error::ProgramError> {
                let mut pda_count: usize = 0;
                let mut compressed_token_accounts = Vec::with_capacity(compressed_accounts.len());

                for (i, compressed_data) in compressed_accounts.iter().enumerate() {
                    let meta = compressed_data.meta.clone();
                    match &compressed_data.data {
                        #(#collect_layout_pda_arms)*
                        LightAccountVariant::PackedCTokenData(data) => {
                            let mut token_data = data.clone();
                            token_data.token_data.version = 3;
                            compressed_token_accounts.push((token_data, meta));
                        }
                        LightAccountVariant::CTokenData(_) => {
                            return std::result::Result::Err(light_sdk::error::LightSdkError::UnexpectedUnpackedVariant.into());
                        }
                        #empty_variant_arm_collect
                    }
                }

                std::result::Result::Ok((pda_count, compressed_token_accounts))
            }

            #[inline(never)]
            #[allow(clippy::too_many_arguments)]
            fn create_and_write_pda<'b, 'c>(
                &self,
                cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, #lifetime>,
                address_space: &solana_pubkey::Pubkey,
                compressed_data: &Self::CompressedData,
                solana_account: &solana_account_info::AccountInfo<#lifetime>,
                seed_params: std::option::Option<&Self::SeedParams>,
                zc_info: &mut light_sdk::interface::ZCompressedAccountInfoMut<'c>,
            ) -> std::result::Result<bool, solana_program_error::ProgramError> {
                let post_system_offset = cpi_accounts.system_accounts_end_offset();
                let all_infos = cpi_accounts.account_infos();
                let post_system_accounts = &all_infos[post_system_offset..];
                let program_id = crate::ID;
                let compressed_meta = &compressed_data.meta;
                let mut variant_seed_params = SeedParams::default();
                let _ = &variant_seed_params; // Suppress unused warning when no params-only fields

                match &compressed_data.data {
                    #(#create_pda_match_arms)*
                    LightAccountVariant::PackedCTokenData(_) => {
                        // Tokens are handled separately, skip here
                        std::result::Result::Ok(false)
                    }
                    LightAccountVariant::CTokenData(_) => {
                        return std::result::Result::Err(light_sdk::error::LightSdkError::UnexpectedUnpackedVariant.into());
                    }
                    #empty_variant_arm_create
                }
            }

            #[inline(never)]
            #[allow(clippy::too_many_arguments)]
            fn process_tokens<'b>(
                &self,
                remaining_accounts: &[solana_account_info::AccountInfo<#lifetime>],
                fee_payer: &solana_account_info::AccountInfo<#lifetime>,
                token_program: &solana_account_info::AccountInfo<#lifetime>,
                token_rent_sponsor: &solana_account_info::AccountInfo<#lifetime>,
                token_cpi_authority: &solana_account_info::AccountInfo<#lifetime>,
                token_config: &solana_account_info::AccountInfo<#lifetime>,
                config: &solana_account_info::AccountInfo<#lifetime>,
                token_accounts: Vec<(Self::PackedTokenData, Self::CompressedMeta)>,
                proof: light_sdk::instruction::ValidityProof,
                cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, #lifetime>,
                post_system_accounts: &[solana_account_info::AccountInfo<#lifetime>],
                has_prior_context: bool,
            ) -> std::result::Result<(), solana_program_error::ProgramError> {
                light_token::compressible::process_decompress_tokens_runtime(
                    remaining_accounts,
                    fee_payer,
                    token_program,
                    token_rent_sponsor,
                    token_cpi_authority,
                    token_config,
                    config,
                    token_accounts,
                    proof,
                    cpi_accounts,
                    post_system_accounts,
                    has_prior_context,
                    &crate::ID,
                )
            }
        }
    })
}

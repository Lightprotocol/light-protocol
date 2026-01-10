//! DecompressContext trait generation.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Result};

// Re-export from variant_enum for convenience
pub use crate::compressible::variant_enum::PdaCtxSeedInfo;

pub fn generate_decompress_context_trait_impl(
    pda_ctx_seeds: Vec<PdaCtxSeedInfo>,
    token_variant_ident: Ident,
    lifetime: syn::Lifetime,
) -> Result<TokenStream> {
    // Generate match arms that extract idx fields, resolve Pubkeys, construct CtxSeeds
    let pda_match_arms: Vec<_> = pda_ctx_seeds
        .iter()
        .map(|info| {
            let pda_type = &info.type_name;
            let packed_name = format_ident!("Packed{}", pda_type);
            let ctx_seeds_struct_name = format_ident!("{}CtxSeeds", pda_type);
            let ctx_fields = &info.ctx_seed_fields;
            
            // Generate pattern to extract idx fields from packed variant
            let idx_field_patterns: Vec<_> = ctx_fields.iter().map(|field| {
                let idx_field = format_ident!("{}_idx", field);
                quote! { #idx_field }
            }).collect();
            
            // Generate code to resolve idx fields to Pubkeys
            let resolve_ctx_seeds: Vec<_> = ctx_fields.iter().map(|field| {
                let idx_field = format_ident!("{}_idx", field);
                quote! {
                    let #field = *post_system_accounts
                        .get(#idx_field as usize)
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
            
            if ctx_fields.is_empty() {
                quote! {
                    RentFreeAccountVariant::#packed_name { data: packed, .. } => {
                        #ctx_seeds_construction
                        match light_sdk::compressible::handle_packed_pda_variant::<#pda_type, #packed_name, _, _>(
                            &*self.rent_sponsor,
                            cpi_accounts,
                            address_space,
                            &solana_accounts[i],
                            i,
                            &packed,
                            &meta,
                            post_system_accounts,
                            &mut compressed_pda_infos,
                            &program_id,
                            &ctx_seeds,
                            seed_params,
                        ) {
                            std::result::Result::Ok(()) => {},
                            std::result::Result::Err(e) => return std::result::Result::Err(e),
                        }
                    }
                    RentFreeAccountVariant::#pda_type { .. } => {
                        unreachable!("Unpacked variants should not be present during decompression");
                    }
                }
            } else {
                quote! {
                    RentFreeAccountVariant::#packed_name { data: packed, #(#idx_field_patterns,)* .. } => {
                        #(#resolve_ctx_seeds)*
                        #ctx_seeds_construction
                        match light_sdk::compressible::handle_packed_pda_variant::<#pda_type, #packed_name, _, _>(
                            &*self.rent_sponsor,
                            cpi_accounts,
                            address_space,
                            &solana_accounts[i],
                            i,
                            &packed,
                            &meta,
                            post_system_accounts,
                            &mut compressed_pda_infos,
                            &program_id,
                            &ctx_seeds,
                            seed_params,
                        ) {
                            std::result::Result::Ok(()) => {},
                            std::result::Result::Err(e) => return std::result::Result::Err(e),
                        }
                    }
                    RentFreeAccountVariant::#pda_type { .. } => {
                        unreachable!("Unpacked variants should not be present during decompression");
                    }
                }
            }
        })
        .collect();

    let packed_token_variant_ident = format_ident!("Packed{}", token_variant_ident);
    
    Ok(quote! {
        impl<#lifetime> light_sdk::compressible::DecompressContext<#lifetime> for DecompressAccountsIdempotent<#lifetime> {
<<<<<<< HEAD
            type CompressedData = CompressedAccountData;
            type PackedTokenData = light_token_sdk::compat::PackedCTokenData<#token_variant_ident>;
=======
            type CompressedData = RentFreeAccountData;
            type PackedTokenData = light_ctoken_sdk::compat::PackedCTokenData<#packed_token_variant_ident>;
>>>>>>> a606eb113 (wip)
            type CompressedMeta = light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
            type SeedParams = ();

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
                self.ctoken_program.as_ref().map(|a| &**a)
            }

            fn token_cpi_authority(&self) -> std::option::Option<&solana_account_info::AccountInfo<#lifetime>> {
                self.ctoken_cpi_authority.as_ref().map(|a| &**a)
            }

            fn token_config(&self) -> std::option::Option<&solana_account_info::AccountInfo<#lifetime>> {
                self.ctoken_config.as_ref().map(|a| &**a)
            }

            fn collect_pda_and_token<'b>(
                &self,
                cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, #lifetime>,
                address_space: solana_pubkey::Pubkey,
                compressed_accounts: Vec<Self::CompressedData>,
                solana_accounts: &[solana_account_info::AccountInfo<#lifetime>],
                seed_params: std::option::Option<&Self::SeedParams>,
            ) -> std::result::Result<(
                Vec<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>,
                Vec<(Self::PackedTokenData, Self::CompressedMeta)>,
            ), solana_program_error::ProgramError> {
                let post_system_offset = cpi_accounts.system_accounts_end_offset();
                let all_infos = cpi_accounts.account_infos();
                let post_system_accounts = &all_infos[post_system_offset..];
                let program_id = &crate::ID;

                let mut compressed_pda_infos = Vec::with_capacity(compressed_accounts.len());
                let mut compressed_token_accounts = Vec::with_capacity(compressed_accounts.len());

                for (i, compressed_data) in compressed_accounts.into_iter().enumerate() {
                    let meta = compressed_data.meta;
                    match compressed_data.data {
                        #(#pda_match_arms)*
                        RentFreeAccountVariant::PackedCTokenData(mut data) => {
                            data.token_data.version = 3;
                            compressed_token_accounts.push((data, meta));
                        }
                        RentFreeAccountVariant::CTokenData(_) => {
                            unreachable!();
                        }
                    }
                }

                std::result::Result::Ok((compressed_pda_infos, compressed_token_accounts))
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
<<<<<<< HEAD
                light_token_sdk::compressible::process_decompress_tokens_runtime(
                    self,
=======
                light_ctoken_sdk::compressible::process_decompress_tokens_runtime(
>>>>>>> a606eb113 (wip)
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

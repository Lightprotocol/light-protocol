//! DecompressContext trait generation.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Result};

/// Generate DecompressContext impl for PDA-only programs (no token support).
/// Uses `()` as PackedTokenData and returns empty token vec.
pub fn generate_pda_only_decompress_context_trait_impl(
    lifetime: syn::Lifetime,
) -> Result<TokenStream> {
    Ok(quote! {
        impl<#lifetime> light_sdk::interface::DecompressContext<#lifetime> for DecompressAccountsIdempotent<#lifetime> {
            type CompressedData = LightAccountData;
            type PackedTokenData = ();
            type CompressedMeta = light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;

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
                None
            }

            fn token_program(&self) -> std::option::Option<&solana_account_info::AccountInfo<#lifetime>> {
                None
            }

            fn token_cpi_authority(&self) -> std::option::Option<&solana_account_info::AccountInfo<#lifetime>> {
                None
            }

            fn token_config(&self) -> std::option::Option<&solana_account_info::AccountInfo<#lifetime>> {
                None
            }

            fn collect_pda_and_token<'b>(
                &self,
                cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, #lifetime>,
                _address_space: solana_pubkey::Pubkey,
                compressed_accounts: Vec<Self::CompressedData>,
                solana_accounts: &[solana_account_info::AccountInfo<#lifetime>],
                rent: &solana_program::sysvar::rent::Rent,
                current_slot: u64,
            ) -> std::result::Result<(
                Vec<::light_sdk::compressed_account::CompressedAccountInfo>,
                Vec<(Self::PackedTokenData, Self::CompressedMeta)>,
            ), solana_program_error::ProgramError> {
                use light_sdk::interface::DecompressVariant;

                let post_system_offset = cpi_accounts.system_accounts_end_offset();
                let all_infos = cpi_accounts.account_infos();
                let remaining_accounts = &all_infos[post_system_offset..];
                let program_id = &crate::ID;

                // Load LightConfig from the config AccountInfo
                let light_config = light_sdk::interface::LightConfig::load_checked(&self.config, &crate::ID)
                    .map_err(|_| solana_program_error::ProgramError::InvalidAccountData)?;

                let mut compressed_pda_infos = Vec::with_capacity(compressed_accounts.len());

                for (i, compressed_data) in compressed_accounts.into_iter().enumerate() {
                    let meta = compressed_data.meta;

                    // PDA-only programs don't have tokens - all accounts are PDAs
                    let mut ctx = light_sdk::interface::DecompressCtx {
                        program_id,
                        cpi_accounts,
                        remaining_accounts,
                        rent_sponsor: &*self.rent_sponsor,
                        light_config: &light_config,
                        rent,
                        current_slot,
                        compressed_account_infos: Vec::new(),
                    };

                    // Call decompress on the PackedLightAccountVariant - returns () not Option
                    compressed_data.data.decompress(&meta, &solana_accounts[i], &mut ctx)?;
                    // Push all collected infos from ctx
                    compressed_pda_infos.extend(ctx.compressed_account_infos);
                }

                // Return empty token vec for PDA-only programs
                std::result::Result::Ok((compressed_pda_infos, Vec::new()))
            }

            #[inline(never)]
            #[allow(clippy::too_many_arguments)]
            fn process_tokens<'b>(
                &self,
                _remaining_accounts: &[solana_account_info::AccountInfo<#lifetime>],
                _fee_payer: &solana_account_info::AccountInfo<#lifetime>,
                _token_program: &solana_account_info::AccountInfo<#lifetime>,
                _token_rent_sponsor: &solana_account_info::AccountInfo<#lifetime>,
                _token_cpi_authority: &solana_account_info::AccountInfo<#lifetime>,
                _token_config: &solana_account_info::AccountInfo<#lifetime>,
                _config: &solana_account_info::AccountInfo<#lifetime>,
                _token_accounts: Vec<(Self::PackedTokenData, Self::CompressedMeta)>,
                _proof: light_sdk::instruction::ValidityProof,
                _cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, #lifetime>,
                _post_system_accounts: &[solana_account_info::AccountInfo<#lifetime>],
                _has_prior_context: bool,
            ) -> std::result::Result<(), solana_program_error::ProgramError> {
                // PDA-only programs don't process tokens
                Ok(())
            }
        }
    })
}

/// Generate DecompressContext impl for programs with token support.
pub fn generate_decompress_context_trait_impl(
    token_variant_ident: Ident,
    lifetime: syn::Lifetime,
) -> Result<TokenStream> {
    let packed_token_variant_ident = format_ident!("Packed{}", token_variant_ident);

    Ok(quote! {
        impl<#lifetime> light_sdk::interface::DecompressContext<#lifetime> for DecompressAccountsIdempotent<#lifetime> {
            type CompressedData = LightAccountData;
            type PackedTokenData = light_token::compat::PackedCTokenData<#packed_token_variant_ident>;
            type CompressedMeta = light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;

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

            fn collect_pda_and_token<'b>(
                &self,
                cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, #lifetime>,
                _address_space: solana_pubkey::Pubkey,
                compressed_accounts: Vec<Self::CompressedData>,
                solana_accounts: &[solana_account_info::AccountInfo<#lifetime>],
                rent: &solana_program::sysvar::rent::Rent,
                current_slot: u64,
            ) -> std::result::Result<(
                Vec<::light_sdk::compressed_account::CompressedAccountInfo>,
                Vec<(Self::PackedTokenData, Self::CompressedMeta)>,
            ), solana_program_error::ProgramError> {
                use light_sdk::interface::DecompressVariant;

                let post_system_offset = cpi_accounts.system_accounts_end_offset();
                let all_infos = cpi_accounts.account_infos();
                let remaining_accounts = &all_infos[post_system_offset..];
                let program_id = &crate::ID;

                // Load LightConfig from the config AccountInfo
                let light_config = light_sdk::interface::LightConfig::load_checked(&self.config, &crate::ID)
                    .map_err(|_| solana_program_error::ProgramError::InvalidAccountData)?;

                let mut compressed_pda_infos = Vec::with_capacity(compressed_accounts.len());
                let mut compressed_token_accounts = Vec::with_capacity(compressed_accounts.len());

                for (i, compressed_data) in compressed_accounts.into_iter().enumerate() {
                    let meta = compressed_data.meta;

                    // Check if this is a token variant by matching
                    match &compressed_data.data {
                        PackedLightAccountVariant::PackedCTokenData(mut data) => {
                            data.token_data.version = 3;
                            compressed_token_accounts.push((data.clone(), meta));
                        }
                        _ => {
                            // PDA variant - use DecompressVariant trait
                            let mut ctx = light_sdk::interface::DecompressCtx {
                                program_id,
                                cpi_accounts,
                                remaining_accounts,
                                rent_sponsor: &*self.rent_sponsor,
                                light_config: &light_config,
                                rent,
                                current_slot,
                                compressed_account_infos: Vec::new(),
                            };

                            // Call decompress on the PackedLightAccountVariant
                            compressed_data.data.decompress(&meta, &solana_accounts[i], &mut ctx)?;
                            // Push all collected infos from ctx
                            compressed_pda_infos.extend(ctx.compressed_account_infos);
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

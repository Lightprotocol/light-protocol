//! Compress code generation.
//!
//! This module provides the `CompressBuilder` for generating compress instruction
//! code including context implementation, processor, entrypoint, and accounts struct.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Result, Type};

use super::parsing::InstructionVariant;
use crate::light_pdas::shared_utils::qualify_type_with_crate;

// =============================================================================
// COMPRESS BUILDER
// =============================================================================

/// Builder for generating compress instruction code.
///
/// Encapsulates the account types and variant configuration needed to generate
/// all compress-related code: context implementation, processor function,
/// instruction entrypoint, and accounts struct.
pub(super) struct CompressBuilder {
    /// Account types that can be compressed.
    account_types: Vec<Type>,
    /// The instruction variant (PdaOnly, TokenOnly, or Mixed).
    variant: InstructionVariant,
}

impl CompressBuilder {
    /// Create a new CompressBuilder with the given account types and variant.
    ///
    /// # Arguments
    /// * `account_types` - The account types that can be compressed
    /// * `variant` - The instruction variant determining what gets generated
    ///
    /// # Returns
    /// A new CompressBuilder instance
    pub fn new(account_types: Vec<Type>, variant: InstructionVariant) -> Self {
        Self {
            account_types,
            variant,
        }
    }

    // -------------------------------------------------------------------------
    // Query Methods
    // -------------------------------------------------------------------------

    /// Returns true if this builder generates PDA compression code.
    ///
    /// This is true for `PdaOnly` and `Mixed` variants.
    pub fn has_pdas(&self) -> bool {
        matches!(
            self.variant,
            InstructionVariant::PdaOnly | InstructionVariant::Mixed
        )
    }

    /// Validate the builder configuration.
    ///
    /// Checks that:
    /// - At least one account type is provided (for PDA variants)
    /// - All account sizes are within the 800-byte limit
    ///
    /// # Returns
    /// `Ok(())` if validation passes, or a `syn::Error` describing the issue.
    pub fn validate(&self) -> Result<()> {
        // For variants that include PDAs, require at least one account type
        if self.has_pdas() && self.account_types.is_empty() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "CompressBuilder requires at least one account type for PDA compression",
            ));
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Code Generation Methods
    // -------------------------------------------------------------------------

    /// Generate the compress context implementation module.
    ///
    /// Creates a module containing the `CompressContext` trait implementation
    /// that handles discriminator-based deserialization and compression.
    pub fn generate_context_impl(&self) -> Result<syn::ItemMod> {
        let lifetime: syn::Lifetime = syn::parse_quote!('info);

        let compress_arms: Vec<_> = self.account_types.iter().map(|account_type| {
            let name = qualify_type_with_crate(account_type);
            quote! {
                d if d == #name::LIGHT_DISCRIMINATOR => {
                    drop(data);
                    let data_borrow = account_info.try_borrow_data().map_err(__anchor_to_program_error)?;
                    let mut account_data = #name::try_deserialize(&mut &data_borrow[..])
                        .map_err(__anchor_to_program_error)?;
                    drop(data_borrow);

                    let compressed_info = light_sdk::compressible::compress_account::prepare_account_for_compression::<#name>(
                        program_id,
                        account_info,
                        &mut account_data,
                        meta,
                        cpi_accounts,
                        &compression_config.address_space,
                    )?;
                    Ok(Some(compressed_info))
                }
            }
        }).collect();

        Ok(syn::parse_quote! {
            mod __compress_context_impl {
                use super::*;
                use light_sdk::LightDiscriminator;
                use light_sdk::compressible::HasCompressionInfo;

                #[inline(always)]
                fn __anchor_to_program_error<E: Into<anchor_lang::error::Error>>(e: E) -> solana_program_error::ProgramError {
                    let err: anchor_lang::error::Error = e.into();
                    let program_error: anchor_lang::prelude::ProgramError = err.into();
                    let code = match program_error {
                        anchor_lang::prelude::ProgramError::Custom(code) => code,
                        _ => 0,
                    };
                    solana_program_error::ProgramError::Custom(code)
                }

                impl<#lifetime> light_sdk::compressible::CompressContext<#lifetime> for CompressAccountsIdempotent<#lifetime> {
                    fn fee_payer(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                        &*self.fee_payer
                    }

                    fn config(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                        &self.config
                    }

                    fn rent_sponsor(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                        &self.rent_sponsor
                    }

                    fn compression_authority(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                        &self.compression_authority
                    }

                    fn compress_pda_account(
                        &self,
                        account_info: &solana_account_info::AccountInfo<#lifetime>,
                        meta: &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                        cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'_, #lifetime>,
                        compression_config: &light_sdk::compressible::CompressibleConfig,
                        program_id: &solana_pubkey::Pubkey,
                    ) -> std::result::Result<Option<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>, solana_program_error::ProgramError> {
                        let data = account_info.try_borrow_data().map_err(__anchor_to_program_error)?;
                        let discriminator = &data[0..8];

                        match discriminator {
                            #(#compress_arms)*
                            _ => Err(__anchor_to_program_error(anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch))
                        }
                    }
                }
            }
        })
    }

    /// Generate the processor function for compress accounts.
    pub fn generate_processor(&self) -> Result<syn::ItemFn> {
        Ok(syn::parse_quote! {
            #[inline(never)]
            pub fn process_compress_accounts_idempotent<'info>(
                accounts: &CompressAccountsIdempotent<'info>,
                remaining_accounts: &[solana_account_info::AccountInfo<'info>],
                compressed_accounts: Vec<light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress>,
                system_accounts_offset: u8,
            ) -> Result<()> {
                light_sdk::compressible::compress_runtime::process_compress_pda_accounts_idempotent(
                    accounts,
                    remaining_accounts,
                    compressed_accounts,
                    system_accounts_offset,
                    LIGHT_CPI_SIGNER,
                    &crate::ID,
                )
                .map_err(|e: solana_program_error::ProgramError| -> anchor_lang::error::Error { e.into() })
            }
        })
    }

    /// Generate the compress instruction entrypoint function.
    pub fn generate_entrypoint(&self) -> Result<syn::ItemFn> {
        Ok(syn::parse_quote! {
            #[inline(never)]
            #[allow(clippy::too_many_arguments)]
            pub fn compress_accounts_idempotent<'info>(
                ctx: Context<'_, '_, '_, 'info, CompressAccountsIdempotent<'info>>,
                proof: light_sdk::instruction::ValidityProof,
                compressed_accounts: Vec<light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress>,
                system_accounts_offset: u8,
            ) -> Result<()> {
                __processor_functions::process_compress_accounts_idempotent(
                    &ctx.accounts,
                    &ctx.remaining_accounts,
                    compressed_accounts,
                    system_accounts_offset,
                )
            }
        })
    }

    /// Generate the compress accounts struct.
    ///
    /// The accounts struct is the same for all variants since it provides
    /// shared infrastructure for compression operations. For `TokenOnly`,
    /// the struct is still generated but PDA compression will return errors.
    pub fn generate_accounts_struct(&self) -> Result<syn::ItemStruct> {
        // All variants use the same accounts struct - it's shared infrastructure
        // for compression operations. The variant behavior is determined by
        // the context impl, not the accounts struct.
        Ok(syn::parse_quote! {
            #[derive(Accounts)]
            pub struct CompressAccountsIdempotent<'info> {
                #[account(mut)]
                pub fee_payer: Signer<'info>,
                /// CHECK: Checked by SDK
                pub config: AccountInfo<'info>,
                /// CHECK: Checked by SDK
                #[account(mut)]
                pub rent_sponsor: AccountInfo<'info>,
                /// CHECK: Checked by SDK
                #[account(mut)]
                pub compression_authority: AccountInfo<'info>,
            }
        })
    }

    /// Generate compile-time size validation for compressed accounts.
    pub fn generate_size_validation(&self) -> Result<TokenStream> {
        let size_checks: Vec<_> = self.account_types.iter().map(|account_type| {
            let qualified_type = qualify_type_with_crate(account_type);
            quote! {
                const _: () = {
                    const COMPRESSED_SIZE: usize = 8 + <#qualified_type as light_sdk::compressible::compression_info::CompressedInitSpace>::COMPRESSED_INIT_SPACE;
                    if COMPRESSED_SIZE > 800 {
                        panic!(concat!(
                            "Compressed account '", stringify!(#qualified_type), "' exceeds 800-byte compressible account size limit. If you need support for larger accounts, send a message to team@lightprotocol.com"
                        ));
                    }
                };
            }
        }).collect();

        Ok(quote! { #(#size_checks)* })
    }

    /// Generate the error codes enum.
    ///
    /// The error codes enum is the same for all variants. It includes all
    /// possible error conditions even if some don't apply to specific variants.
    /// This ensures consistent error handling across different instruction types.
    pub fn generate_error_codes(&self) -> Result<TokenStream> {
        // All variants use the same error codes - shared infrastructure
        // that covers all possible error conditions.
        Ok(quote! {
            #[error_code]
            pub enum LightInstructionError {
                #[msg("Rent sponsor mismatch")]
                InvalidRentSponsor,
                #[msg("Missing seed account")]
                MissingSeedAccount,
                #[msg("Seed value does not match account data")]
                SeedMismatch,
                #[msg("Not implemented")]
                CTokenDecompressionNotImplemented,
                #[msg("Not implemented")]
                PdaDecompressionNotImplemented,
                #[msg("Not implemented")]
                TokenCompressionNotImplemented,
                #[msg("Not implemented")]
                PdaCompressionNotImplemented,
            }
        })
    }
}
